use super::{Message, RequestMessageData, ResponseMessageData};
use crate::{
    error::{Error, Result},
    model::{Format, GenerateOptions, JapaneseLyricsType, ParseOptions, UfData},
    ConvertJapaneseLyricsOptions, IllegalFile,
};
use std::str::FromStr;

use anyhow::anyhow;
use rquickjs::{
    function::{Async, Func},
    Array, AsyncContext, AsyncRuntime, CatchResultExt, CaughtError, CaughtResult, Ctx, FromJs,
    Function, Object, Promise, TypedArray, Value,
};
use tracing::info;
use uuid::Uuid;

pub(super) fn runner_entry(
    receiver: async_channel::Receiver<Message<RequestMessageData>>,
    sender: async_channel::Sender<Message<ResponseMessageData>>,
) {
    info!("JS runner thread started");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime");

    let main = std::panic::catch_unwind(|| {
        let sender = sender.clone();
        rt.block_on(runner_entry_inner(receiver, sender));
    });
    if main.is_err() {
        sender
            .send_blocking(Message {
                nonce: Uuid::new_v4(),
                message: ResponseMessageData::Panic,
            })
            .expect("Failed to send panic message");
    }
}
async fn runner_entry_inner(
    receiver: async_channel::Receiver<Message<RequestMessageData>>,
    sender: async_channel::Sender<Message<ResponseMessageData>>,
) {
    info!("Loading utaformatix");
    let runtime = AsyncRuntime::new().expect("Failed to create JS runtime");
    let context = AsyncContext::full(&runtime)
        .await
        .expect("Failed to create JS context");
    context
        .async_with(async |ctx| {
            let global = ctx.globals();
            let sleep = Function::new(ctx.clone(), Async(crate::js_impls::sleep))
                .expect("Failed to create sleep function")
                .with_name("__host_sleep")
                .expect("Failed to name sleep function");
            global
                .set("__host_sleep", sleep)
                .expect("Failed to register sleep function");
            global
                .set("__host_encode", Func::from(crate::js_impls::encode))
                .expect("Failed to register encode function");
            global
                .set("__host_decode", Func::from(crate::js_impls::decode))
                .expect("Failed to register decode function");
            global
                .set("__host_log", Func::from(crate::js_impls::log))
                .expect("Failed to register log function");
            ctx.eval::<(), _>(include_str!("../utaformatix.js"))
                .catch(&ctx)
                .unwrap_or_else(|error| panic!("Failed to evaluate script: {:?}", error));
            let _: Object = global
                .get("utaformatix")
                .expect("Failed to initialize utaformatix");
        })
        .await;

    info!("Loaded utaformatix");

    loop {
        info!("Waiting for message");
        let Ok(Message { message, nonce }) = receiver.recv_blocking() else {
            info!("Runner channel closed");
            break;
        };
        info!("Received message: {:?}", message);
        match message {
            RequestMessageData::ParseSingle {
                data,
                options,
                format,
            } => {
                let result = parse_single(&context, format, data, options).await;
                info!("Completed parsing");
                sender
                    .send_blocking(Message {
                        nonce,
                        message: ResponseMessageData::Parse(result),
                    })
                    .expect("Failed to send response");
            }
            RequestMessageData::ParseMultiple {
                data,
                options,
                format,
            } => {
                let result = parse_multiple(&context, format, data, options).await;
                info!("Completed parsing multiple");
                sender
                    .send_blocking(Message {
                        nonce,
                        message: ResponseMessageData::Parse(result),
                    })
                    .expect("Failed to send response");
            }
            RequestMessageData::GenerateSingle {
                data,
                options,
                format,
            } => {
                let result = generate_single(&context, format, data, options).await;
                info!("Completed generating");
                sender
                    .send_blocking(Message {
                        nonce,
                        message: ResponseMessageData::GenerateSingle(result),
                    })
                    .expect("Failed to send response");
            }
            RequestMessageData::GenerateMultiple {
                data,
                options,
                format,
            } => {
                let result = generate_multiple(&context, format, data, options).await;
                info!("Completed generating multiple");
                sender
                    .send_blocking(Message {
                        nonce,
                        message: ResponseMessageData::GenerateMultiple(result),
                    })
                    .expect("Failed to send response");
            }
            RequestMessageData::AnalyzeJapaneseLyricsType { data } => {
                let result = analyze_japanese_lyrics_type(&context, data).await;
                info!("Completed analyzing Japanese lyrics type: {:?}", result);
                sender
                    .send_blocking(Message {
                        nonce,
                        message: ResponseMessageData::AnalyzeJapaneseLyricsType(result),
                    })
                    .expect("Failed to send response");
            }
            RequestMessageData::ConvertJapaneseLyrics {
                data,
                source_type,
                target_type,
                options,
            } => {
                let result =
                    convert_japanese_lyrics(&context, data, source_type, target_type, options)
                        .await;
                info!("Completed converting Japanese lyrics");
                sender
                    .send_blocking(Message {
                        nonce,
                        message: ResponseMessageData::ConvertJapaneseLyrics(result),
                    })
                    .expect("Failed to send response");
            }
        }
        info!("Sent response");
    }
}

fn wrap_error<'js>(
    ctx: &Ctx<'js>,
    utaformatix: &Object<'js>,
    result: CaughtResult<'js, Value<'js>>,
) -> Result<Value<'js>> {
    result.map_err(|error| map_js_error(ctx, utaformatix, error))
}

fn map_js_error<'js>(_ctx: &Ctx<'js>, utaformatix: &Object<'js>, error: CaughtError<'js>) -> Error {
    let object = match &error {
        CaughtError::Exception(exception) => Some(exception.as_object().clone()),
        CaughtError::Value(value) => value.as_object().cloned(),
        CaughtError::Error(_) => None,
    };

    if let Some(object) = object {
        for (error, name) in [
            (Error::EmptyProject, "EmptyProjectException"),
            (Error::IllegalNotePosition, "IllegalNotePositionException"),
            (Error::NotesOverlapping, "NotesOverlappingException"),
            (Error::UnsupportedFileFormat, "UnsupportedFileFormatError"),
            (Error::UnsupportedLegacyPpsf, "UnsupportedLegacyPpsfError"),
        ] {
            let exception: Value = utaformatix.get(name).expect("Failed to get exception");
            if object.is_instance_of(&exception) {
                return error;
            }
        }

        let illegal_file_exception: Value = utaformatix
            .get("IllegalFileException")
            .expect("Failed to get exception");
        if object.is_instance_of(&illegal_file_exception) {
            let constructor: Object = object
                .get("constructor")
                .expect("Failed to get constructor");
            let name: String = constructor.get("name").expect("Failed to get name");
            let kind = IllegalFile::from_str(&name).expect("Failed to convert to IllegalFile");
            return Error::IllegalFile(kind);
        }
    }

    Error::Unexpected(format!("{:?}", error))
}

fn json_to_js<'js>(ctx: &Ctx<'js>, value: serde_json::Value) -> rquickjs::Result<Value<'js>> {
    let json = serde_json::to_string(&value).expect("Failed to stringify JSON");
    let json_object: Object = ctx.globals().get("JSON")?;
    let parse: Function = json_object.get("parse")?;
    parse.call((json,))
}

fn js_to_json<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<serde_json::Value> {
    let json_object: Object = ctx
        .globals()
        .get("JSON")
        .map_err(|e| anyhow!("Failed to get JSON object: {:?}", e))?;
    let stringify: Function = json_object
        .get("stringify")
        .map_err(|e| anyhow!("Failed to get JSON.stringify: {:?}", e))?;
    let json: String = stringify
        .call((value,))
        .map_err(|e| anyhow!("Failed to stringify JSON: {:?}", e))?;
    Ok(serde_json::from_str(&json).map_err(|e| anyhow!("Failed to parse JSON: {:?}", e))?)
}

async fn call_async_function<'js>(
    ctx: &Ctx<'js>,
    utaformatix: &Object<'js>,
    function: Function<'js>,
    args: impl rquickjs::function::IntoArgs<'js>,
) -> Result<Value<'js>> {
    let promise = function
        .call::<_, Promise>(args)
        .catch(ctx)
        .map_err(|e| map_js_error(ctx, utaformatix, e))?;
    wrap_error(
        ctx,
        utaformatix,
        promise.into_future::<Value>().await.catch(ctx),
    )
}

async fn parse_single(
    context: &AsyncContext,
    format: Format,
    data: Vec<u8>,
    options: ParseOptions,
) -> Result<UfData> {
    context
        .async_with(async move |ctx| {
            let utaformatix: Object = ctx
                .globals()
                .get("utaformatix")
                .expect("Failed to get utaformatix");
            let data = TypedArray::new(ctx.clone(), data)
                .map_err(|e| anyhow!("Failed to create Uint8Array: {:?}", e))?;
            let options = json_to_js(
                &ctx,
                serde_json::to_value(options).expect("Failed to convert to JSON"),
            )
            .map_err(|e| anyhow!("Failed to convert options to JS: {:?}", e))?;
            let function_name = format!("parse{}", format.suffix());
            let parser: Function = utaformatix
                .get(function_name)
                .expect("Failed to get parse function");
            let result = call_async_function(&ctx, &utaformatix, parser, (data, options)).await?;
            Ok(serde_json::from_value(js_to_json(&ctx, result)?)
                .map_err(|e| anyhow!("Failed to parse JSON: {:?}", e))?)
        })
        .await
}

async fn parse_multiple(
    context: &AsyncContext,
    format: Format,
    data: Vec<Vec<u8>>,
    options: ParseOptions,
) -> Result<UfData> {
    context
        .async_with(async move |ctx| {
            let utaformatix: Object = ctx
                .globals()
                .get("utaformatix")
                .expect("Failed to get utaformatix");
            let data = data
                .into_iter()
                .map(|data| {
                    TypedArray::new(ctx.clone(), data)
                        .map_err(|e| anyhow!("Failed to create Uint8Array: {:?}", e))
                })
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| anyhow!("Failed to create Uint8Array: {:?}", e))?;
            let options = json_to_js(
                &ctx,
                serde_json::to_value(options).expect("Failed to convert to JSON"),
            )
            .map_err(|e| anyhow!("Failed to convert options to JS: {:?}", e))?;
            let function_name = format!("parse{}", format.suffix());
            let parser: Function = utaformatix
                .get(function_name)
                .expect("Failed to get parse function");
            let result = call_async_function(&ctx, &utaformatix, parser, (data, options)).await?;
            Ok(serde_json::from_value(js_to_json(&ctx, result)?)
                .map_err(|e| anyhow!("Failed to parse JSON: {:?}", e))?)
        })
        .await
}

async fn generate_single(
    context: &AsyncContext,
    format: Format,
    data: UfData,
    options: GenerateOptions,
) -> Result<Vec<u8>> {
    context
        .async_with(async move |ctx| {
            let utaformatix: Object = ctx
                .globals()
                .get("utaformatix")
                .expect("Failed to get utaformatix");
            let data = json_to_js(
                &ctx,
                serde_json::to_value(data).expect("Failed to convert to JSON"),
            )
            .map_err(|e| anyhow!("Failed to convert data to JS: {:?}", e))?;
            let options = json_to_js(
                &ctx,
                serde_json::to_value(options).expect("Failed to convert to JSON"),
            )
            .map_err(|e| anyhow!("Failed to convert options to JS: {:?}", e))?;
            let function_name = format!("generate{}", format.suffix());
            let generator: Function = utaformatix
                .get(function_name)
                .expect("Failed to get generate function");
            let result =
                call_async_function(&ctx, &utaformatix, generator, (data, options)).await?;
            let array =
                TypedArray::<u8>::from_value(result).expect("Failed to convert to Uint8Array");
            Ok(array
                .as_bytes()
                .expect("Failed to get typed array bytes")
                .to_vec())
        })
        .await
}

async fn generate_multiple(
    context: &AsyncContext,
    format: Format,
    data: UfData,
    options: GenerateOptions,
) -> Result<Vec<Vec<u8>>> {
    context
        .async_with(async move |ctx| {
            let utaformatix: Object = ctx
                .globals()
                .get("utaformatix")
                .expect("Failed to get utaformatix");
            let data = json_to_js(
                &ctx,
                serde_json::to_value(data).expect("Failed to convert to JSON"),
            )
            .map_err(|e| anyhow!("Failed to convert data to JS: {:?}", e))?;
            let options = json_to_js(
                &ctx,
                serde_json::to_value(options).expect("Failed to convert to JSON"),
            )
            .map_err(|e| anyhow!("Failed to convert options to JS: {:?}", e))?;
            let function_name = format!("generate{}", format.suffix());
            let generator: Function = utaformatix
                .get(function_name)
                .expect("Failed to get generate function");
            let result =
                call_async_function(&ctx, &utaformatix, generator, (data, options)).await?;
            let result = Array::from_value(result).expect("Failed to convert to array");
            let mut files = vec![];
            for i in 0..result.len() {
                let array: TypedArray<u8> = result.get(i).expect("Failed to get result file");
                files.push(
                    array
                        .as_bytes()
                        .expect("Failed to get typed array bytes")
                        .to_vec(),
                );
            }
            Ok(files)
        })
        .await
}

async fn analyze_japanese_lyrics_type(
    context: &AsyncContext,
    data: UfData,
) -> Result<Option<JapaneseLyricsType>> {
    context
        .async_with(async move |ctx| {
            let utaformatix: Object = ctx
                .globals()
                .get("utaformatix")
                .expect("Failed to get utaformatix");
            let data = json_to_js(
                &ctx,
                serde_json::to_value(data).expect("Failed to convert to JSON"),
            )
            .map_err(|e| anyhow!("Failed to convert data to JS: {:?}", e))?;
            let parser: Function = utaformatix
                .get("analyzeJapaneseLyricsType")
                .expect("Failed to get parse function");
            let result = wrap_error(&ctx, &utaformatix, parser.call((data,)).catch(&ctx))?;
            let result: String =
                String::from_js(&ctx, result).expect("Failed to convert lyrics type to string");
            Ok(JapaneseLyricsType::from_str(&result).ok())
        })
        .await
}

async fn convert_japanese_lyrics(
    context: &AsyncContext,
    data: UfData,
    source: JapaneseLyricsType,
    to: JapaneseLyricsType,
    options: ConvertJapaneseLyricsOptions,
) -> Result<UfData> {
    context
        .async_with(async move |ctx| {
            let utaformatix: Object = ctx
                .globals()
                .get("utaformatix")
                .expect("Failed to get utaformatix");
            let data = json_to_js(
                &ctx,
                serde_json::to_value(data).expect("Failed to convert to JSON"),
            )
            .map_err(|e| anyhow!("Failed to convert data to JS: {:?}", e))?;
            let options = json_to_js(
                &ctx,
                serde_json::to_value(options).expect("Failed to convert to JSON"),
            )
            .map_err(|e| anyhow!("Failed to convert options to JS: {:?}", e))?;
            let parser: Function = utaformatix
                .get("convertJapaneseLyrics")
                .expect("Failed to get parse function");
            let result = wrap_error(
                &ctx,
                &utaformatix,
                parser
                    .call((data, source.to_string(), to.to_string(), options))
                    .catch(&ctx),
            )?;
            Ok(serde_json::from_value(js_to_json(&ctx, result)?)
                .map_err(|e| anyhow!("Failed to parse JSON: {:?}", e))?)
        })
        .await
}
