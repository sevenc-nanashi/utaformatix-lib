use crate::{
    error::{Error, Result},
    model::{Format, GenerateOptions, JapaneseLyricsType, ParseOptions, UfData},
    ConvertJapaneseLyricsOptions, IllegalFile,
};
use std::{cell::OnceCell, str::FromStr};

use anyhow::anyhow;
use boa_engine::{
    js_string,
    object::builtins::{JsArray, JsTypedArray},
    JsResult, JsString, JsValue, NativeFunction,
};
use educe::Educe;
use tracing::info;
use uuid::Uuid;

pub(crate) struct Message<T> {
    pub(crate) message: T,
    pub(crate) nonce: Uuid,
}

impl<T> Message<T> {
    pub(crate) fn new(message: T) -> Self {
        Self {
            message,
            nonce: Uuid::new_v4(),
        }
    }
}

#[derive(Educe, Clone)]
#[educe(Debug)]
pub(crate) enum RequestMessageData {
    ParseSingle {
        #[educe(Debug(ignore))]
        data: Vec<u8>,
        options: ParseOptions,
        format: Format,
    },
    ParseMultiple {
        #[educe(Debug(ignore))]
        data: Vec<Vec<u8>>,
        options: ParseOptions,
        format: Format,
    },
    GenerateSingle {
        #[educe(Debug(ignore))]
        data: UfData,
        options: GenerateOptions,
        format: Format,
    },
    GenerateMultiple {
        #[educe(Debug(ignore))]
        data: UfData,
        options: GenerateOptions,
        format: Format,
    },
    AnalyzeJapaneseLyricsType {
        #[educe(Debug(ignore))]
        data: UfData,
    },
    ConvertJapaneseLyrics {
        #[educe(Debug(ignore))]
        data: UfData,
        source_type: JapaneseLyricsType,
        target_type: JapaneseLyricsType,
        options: ConvertJapaneseLyricsOptions,
    },
}

#[derive(Educe, Clone)]
#[educe(Debug)]
pub(crate) enum ResponseMessageData {
    Panic,
    Parse(Result<UfData>),
    GenerateSingle(Result<Vec<u8>>),
    GenerateMultiple(Result<Vec<Vec<u8>>>),
    AnalyzeJapaneseLyricsType(Result<Option<JapaneseLyricsType>>),
    ConvertJapaneseLyrics(Result<UfData>),
}

pub(crate) struct SyncThread {
    pub(crate) handle: OnceCell<std::thread::JoinHandle<()>>,
    pub(crate) request_sender: async_channel::Sender<Message<RequestMessageData>>,
    pub(crate) response_receiver: async_channel::Receiver<Message<ResponseMessageData>>,
}

impl Drop for SyncThread {
    fn drop(&mut self) {
        info!("Dropping SyncThread");
        self.request_sender.close();
        info!("Closed request sender");
        self.handle
            .take()
            .expect("Failed to get handle")
            .join()
            .expect("Failed to join thread");
    }
}

impl SyncThread {
    pub(crate) fn new() -> Self {
        let (request_sender, request_receiver) = async_channel::unbounded();
        let (response_sender, response_receiver) = async_channel::unbounded();
        let handle = std::thread::spawn(move || {
            runner_entry(request_receiver, response_sender);
        });
        let handle_cell = OnceCell::new();
        handle_cell.set(handle).expect("Failed to set handle");
        Self {
            handle: handle_cell,
            request_sender,
            response_receiver,
        }
    }
}
fn runner_entry(
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
    let source = boa_engine::Source::from_bytes(include_str!("./utaformatix.js"));
    let queue = std::rc::Rc::new(crate::job_queue::TokioJobQueue::default());
    let mut context = boa_engine::Context::builder()
        .job_queue(queue)
        .build()
        .unwrap();

    context
        .register_global_builtin_callable(
            js_string!("__sleep"),
            2,
            NativeFunction::from_async_fn(crate::js_impls::sleep),
        )
        .expect("Failed to register sleep function");
    context
        .register_global_builtin_callable(
            js_string!("__encode"),
            1,
            NativeFunction::from_fn_ptr(crate::js_impls::encode),
        )
        .expect("Failed to register encode function");
    context
        .register_global_builtin_callable(
            js_string!("__decode"),
            1,
            NativeFunction::from_fn_ptr(crate::js_impls::decode),
        )
        .expect("Failed to register decode function");
    context.eval(source).expect("Failed to evaluate script");

    let mut utaformatix = match context
        .global_object()
        .get(js_string!("utaformatix"), &mut context)
    {
        Ok(boa_engine::JsValue::Object(val)) => val,
        Ok(_) => panic!("Failed to initialize utaformatix: Unexpected return value"),
        Err(error) => {
            let value = error.to_opaque(&mut context);
            panic!(
                "Failed to initialize utaformatix: {:?}",
                value.to_json(&mut context)
            );
        }
    };

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
                let result =
                    parse_single(&mut utaformatix, &mut context, format, data, options).await;
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
                let result =
                    parse_multiple(&mut utaformatix, &mut context, format, data, options).await;
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
                let result =
                    generate_single(&mut utaformatix, &mut context, format, data, options).await;
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
                let result =
                    generate_multiple(&mut utaformatix, &mut context, format, data, options).await;
                info!("Completed generating multiple");
                sender
                    .send_blocking(Message {
                        nonce,
                        message: ResponseMessageData::GenerateMultiple(result),
                    })
                    .expect("Failed to send response");
            }
            RequestMessageData::AnalyzeJapaneseLyricsType { data } => {
                let result = analyze_japanese_lyrics_type(&mut utaformatix, &mut context, data);
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
                let result = convert_japanese_lyrics(
                    &mut utaformatix,
                    &mut context,
                    data,
                    source_type,
                    target_type,
                    options,
                );
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

fn wrap_error(
    result: JsResult<boa_engine::JsValue>,
    utaformatix: &mut boa_engine::JsObject,
    context: &mut boa_engine::Context,
) -> Result<boa_engine::JsValue> {
    let result = result.map_err(|e| {
        let value = e.to_opaque(context);
        for (error, name) in [
            (Error::EmptyProject, js_string!("EmptyProjectException")),
            (
                Error::IllegalNotePosition,
                js_string!("IllegalNotePositionException"),
            ),
            (
                Error::NotesOverlapping,
                js_string!("NotesOverlappingException"),
            ),
            (
                Error::UnsupportedFileFormat,
                js_string!("UnsupportedFileFormatError"),
            ),
            (
                Error::UnsupportedLegacyPpsf,
                js_string!("UnsupportedLegacyPpsfError"),
            ),
        ] {
            let exception = utaformatix
                .get(name.to_owned(), context)
                .expect("Failed to get exception");
            if value
                .instance_of(&exception, context)
                .expect("Failed to check instance")
            {
                return error;
            }
        }
        let illegal_file_exception = utaformatix
            .get(js_string!("IllegalFileException"), context)
            .expect("Failed to get exception");
        if value
            .instance_of(&illegal_file_exception, context)
            .expect("Failed to check instance")
        {
            let value = value.as_object().expect("Failed to convert to object");
            let name = value
                .get(js_string!("constructor"), context)
                .expect("Failed to get constructor")
                .as_object()
                .expect("Failed to convert to object")
                .get(js_string!("name"), context)
                .expect("Failed to get name")
                .as_string()
                .expect("Failed to convert to string")
                .to_std_string()
                .expect("Failed to convert to string");
            let kind = IllegalFile::from_str(&name).expect("Failed to convert to IllegalFile");
            return Error::IllegalFile(kind);
        }

        let value = value.to_string(context).map_or_else(
            |_| "Unknown error".to_owned(),
            |v| v.to_std_string_escaped(),
        );
        Error::Unexpected(value)
    })?;

    Ok(result)
}

async fn parse_single(
    utaformatix: &mut boa_engine::JsObject,
    context: &mut boa_engine::Context,
    format: Format,
    data: Vec<u8>,
    options: ParseOptions,
) -> Result<UfData> {
    let data = boa_engine::object::builtins::JsUint8Array::from_iter(data, context)
        .map_err(|e| anyhow!("Failed to create Uint8Array: {:?}", e))?;
    let function_name = format!("parse{}", format.suffix());
    let boa_engine::JsValue::Object(parser) = utaformatix
        .get(JsString::from(function_name), context)
        .expect("Failed to get parse function")
    else {
        panic!("Failed to get parse function: Unexpected return value");
    };
    if !parser.is_callable() {
        panic!("Failed to get parse function: Unexpected return value");
    }
    let result_promise = parser
        .call(
            &boa_engine::JsValue::undefined(),
            &[
                data.into(),
                boa_engine::JsValue::from_json(
                    &serde_json::to_value(options).expect("Failed to convert to JSON"),
                    context,
                )
                .expect("Failed to convert to JsValue"),
            ],
            context,
        )
        .map_err(|e| anyhow!("Failed to call parse function: {:?}", e))?;
    let boa_engine::JsValue::Object(result_promise) = result_promise else {
        panic!("Failed to call parse function: Unexpected return value");
    };
    let result_promise = boa_engine::object::builtins::JsPromise::from_object(result_promise)
        .expect("Failed to convert to JsPromise");
    let future = result_promise.into_js_future(context);

    let runner = async { context.run_jobs_async().await };

    let (_, result) = tokio::join!(runner, future);

    let result = wrap_error(result, utaformatix, context)?;
    if !result.is_object() {
        return Err(anyhow!("Failed to parse: Unexpected return value: {:?}", result).into());
    }
    Ok(serde_json::from_value(
        result
            .to_json(context)
            .map_err(|e| anyhow!("Failed to convert to JSON: {:?}", e))?,
    )
    .map_err(|e| anyhow!("Failed to parse JSON: {:?}", e))?)
}

async fn parse_multiple(
    utaformatix: &mut boa_engine::JsObject,
    context: &mut boa_engine::Context,
    format: Format,
    data: Vec<Vec<u8>>,
    options: ParseOptions,
) -> Result<UfData> {
    let data = data
        .into_iter()
        .map(|data| boa_engine::object::builtins::JsUint8Array::from_iter(data, context))
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("Failed to create Uint8Array")
        .into_iter()
        .map(JsValue::from)
        .collect::<Vec<JsValue>>();

    let function_name = format!("parse{}", format.suffix());
    let boa_engine::JsValue::Object(parser) = utaformatix
        .get(JsString::from(function_name), context)
        .expect("Failed to get parse function")
    else {
        panic!("Failed to get parse function: Unexpected return value");
    };
    if !parser.is_callable() {
        panic!("Failed to get parse function: Unexpected return value");
    }
    let result_promise = parser
        .call(
            &boa_engine::JsValue::undefined(),
            &[
                boa_engine::object::builtins::JsArray::from_iter(data, context).into(),
                boa_engine::JsValue::from_json(
                    &serde_json::to_value(options).expect("Failed to convert to JSON"),
                    context,
                )
                .expect("Failed to convert to JsValue"),
            ],
            context,
        )
        .map_err(|e| anyhow!("Failed to call parse function: {:?}", e))?;
    let boa_engine::JsValue::Object(result_promise) = result_promise else {
        panic!("Failed to call parse function: Unexpected return value");
    };
    let result_promise = boa_engine::object::builtins::JsPromise::from_object(result_promise)
        .expect("Failed to convert to JsPromise");
    let future = result_promise.into_js_future(context);

    let runner = async { context.run_jobs_async().await };

    let (_, result) = tokio::join!(runner, future);

    let result = wrap_error(result, utaformatix, context)?;
    if !result.is_object() {
        return Err(anyhow!("Failed to parse: Unexpected return value: {:?}", result).into());
    }
    Ok(serde_json::from_value(
        result
            .to_json(context)
            .map_err(|e| anyhow!("Failed to convert to JSON: {:?}", e))?,
    )
    .map_err(|e| anyhow!("Failed to parse JSON: {:?}", e))?)
}

async fn generate_single(
    utaformatix: &mut boa_engine::JsObject,
    context: &mut boa_engine::Context,
    format: Format,
    data: UfData,
    options: GenerateOptions,
) -> Result<Vec<u8>> {
    let function_name = format!("generate{}", format.suffix());
    let boa_engine::JsValue::Object(parser) = utaformatix
        .get(JsString::from(function_name), context)
        .expect("Failed to get parse function")
    else {
        panic!("Failed to get parse function: Unexpected return value");
    };
    if !parser.is_callable() {
        panic!("Failed to get parse function: Unexpected return value");
    }
    let result_promise = parser
        .call(
            &boa_engine::JsValue::undefined(),
            &[
                boa_engine::JsValue::from_json(
                    &serde_json::to_value(data).expect("Failed to convert to JSON"),
                    context,
                )
                .expect("Failed to convert to JsValue"),
                boa_engine::JsValue::from_json(
                    &serde_json::to_value(options).expect("Failed to convert to JSON"),
                    context,
                )
                .expect("Failed to convert to JsValue"),
            ],
            context,
        )
        .map_err(|e| anyhow!("Failed to call parse function: {:?}", e))?;
    let boa_engine::JsValue::Object(result_promise) = result_promise else {
        panic!("Failed to call parse function: Unexpected return value");
    };
    let result_promise = boa_engine::object::builtins::JsPromise::from_object(result_promise)
        .expect("Failed to convert to JsPromise");
    let future = result_promise.into_js_future(context);

    let runner = async { context.run_jobs_async().await };

    let (_, result) = tokio::join!(runner, future);

    let result = wrap_error(result, utaformatix, context)?
        .as_object()
        .expect("Failed to convert to object")
        .to_owned();
    let array = JsTypedArray::from_object(result).expect("Failed to convert to JsTypedArray");
    let length = array.length(context).expect("Failed to get length");
    let mut data = Vec::with_capacity(length as usize);
    for i in 0..length {
        let value = array.get(i, context).expect("Failed to get value");
        data.push(value.as_number().expect("Failed to get number") as u8);
    }

    Ok(data)
}

async fn generate_multiple(
    utaformatix: &mut boa_engine::JsObject,
    context: &mut boa_engine::Context,
    format: Format,
    data: UfData,
    options: GenerateOptions,
) -> Result<Vec<Vec<u8>>> {
    let function_name = format!("generate{}", format.suffix());
    let boa_engine::JsValue::Object(parser) = utaformatix
        .get(JsString::from(function_name), context)
        .expect("Failed to get parse function")
    else {
        panic!("Failed to get parse function: Unexpected return value");
    };
    if !parser.is_callable() {
        panic!("Failed to get parse function: Unexpected return value");
    }
    let result_promise = parser
        .call(
            &boa_engine::JsValue::undefined(),
            &[
                boa_engine::JsValue::from_json(
                    &serde_json::to_value(data).expect("Failed to convert to JSON"),
                    context,
                )
                .expect("Failed to convert to JsValue"),
                boa_engine::JsValue::from_json(
                    &serde_json::to_value(options).expect("Failed to convert to JSON"),
                    context,
                )
                .expect("Failed to convert to JsValue"),
            ],
            context,
        )
        .map_err(|e| anyhow!("Failed to call parse function: {:?}", e))?;
    let boa_engine::JsValue::Object(result_promise) = result_promise else {
        panic!("Failed to call parse function: Unexpected return value");
    };
    let result_promise = boa_engine::object::builtins::JsPromise::from_object(result_promise)
        .expect("Failed to convert to JsPromise");
    let future = result_promise.into_js_future(context);

    let runner = async { context.run_jobs_async().await };

    let (_, result) = tokio::join!(runner, future);

    let result = wrap_error(result, utaformatix, context)?
        .as_object()
        .expect("Failed to convert to object")
        .to_owned();
    let result = JsArray::from_object(result).expect("Failed to convert to JsArray");
    let length = result.length(context).expect("Failed to get length");
    let mut files = vec![];
    for i in 0..length {
        let value = result.get(i, context).expect("Failed to get value");
        let array = JsTypedArray::from_object(
            value
                .as_object()
                .expect("Failed to convert to JsObject")
                .to_owned(),
        )
        .expect("Failed to convert to JsTypedArray");
        let length = array.length(context).expect("Failed to get length");
        let mut data = Vec::with_capacity(length as usize);
        for i in 0..length {
            let value = array.get(i, context).expect("Failed to get value");
            data.push(value.as_number().expect("Failed to get number") as u8);
        }
        files.push(data);
    }

    Ok(files)
}

fn analyze_japanese_lyrics_type(
    utaformatix: &mut boa_engine::JsObject,
    context: &mut boa_engine::Context,
    data: UfData,
) -> Result<Option<JapaneseLyricsType>> {
    let boa_engine::JsValue::Object(parser) = utaformatix
        .get(js_string!("analyzeJapaneseLyricsType"), context)
        .expect("Failed to get parse function")
    else {
        panic!("Failed to get parse function: Unexpected return value");
    };
    if !parser.is_callable() {
        panic!("Failed to get parse function: Unexpected return value");
    }
    let result = parser.call(
        &boa_engine::JsValue::undefined(),
        &[boa_engine::JsValue::from_json(
            &serde_json::to_value(data).expect("Failed to convert to JSON"),
            context,
        )
        .expect("Failed to convert to JsValue")],
        context,
    );
    let result = wrap_error(result, utaformatix, context)?
        .as_string()
        .expect("Failed to convert to string")
        .to_owned();
    let result = result.to_std_string().expect("Failed to convert to string");
    let result = JapaneseLyricsType::from_str(&result).ok();

    Ok(result)
}

fn convert_japanese_lyrics(
    utaformatix: &mut boa_engine::JsObject,
    context: &mut boa_engine::Context,
    data: UfData,
    source: JapaneseLyricsType,
    to: JapaneseLyricsType,
    options: ConvertJapaneseLyricsOptions,
) -> Result<UfData> {
    let boa_engine::JsValue::Object(parser) = utaformatix
        .get(js_string!("convertJapaneseLyrics"), context)
        .expect("Failed to get parse function")
    else {
        panic!("Failed to get parse function: Unexpected return value");
    };
    if !parser.is_callable() {
        panic!("Failed to get parse function: Unexpected return value");
    }
    let result = parser.call(
        &boa_engine::JsValue::undefined(),
        &[
            boa_engine::JsValue::from_json(
                &serde_json::to_value(data).expect("Failed to convert to JSON"),
                context,
            )
            .expect("Failed to convert to JsValue"),
            JsString::from(source.to_string()).into(),
            JsString::from(to.to_string()).into(),
            boa_engine::JsValue::from_json(
                &serde_json::to_value(options).expect("Failed to convert to JSON"),
                context,
            )
            .expect("Failed to convert to JsValue"),
        ],
        context,
    );
    let result = wrap_error(result, utaformatix, context)?;

    Ok(serde_json::from_value(
        result
            .to_json(context)
            .map_err(|e| anyhow!("Failed to convert to JSON: {:?}", e))?,
    )
    .map_err(|e| anyhow!("Failed to parse JSON: {:?}", e))?)
}
