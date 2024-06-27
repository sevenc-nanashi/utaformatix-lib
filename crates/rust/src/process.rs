use crate::{
    error::{Error, Result},
    model::{Format, ParseOptions, UfData},
};
use anyhow::anyhow;
use boa_engine::{js_string, NativeFunction};
use educe::Educe;
use once_cell::sync::Lazy;
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
    Generate(UfData),
}

#[derive(Educe, Clone)]
#[educe(Debug)]
pub(crate) enum ResponseMessageData {
    Panic(String),
    Parse(Result<UfData>),
    Generate(Result<Vec<u8>>),
    GenerateMultiple(Result<Vec<Vec<u8>>>),
}

#[derive(Clone, Debug)]
struct ChannelContainer<T: Clone> {
    sender: async_channel::Sender<Message<T>>,
    receiver: async_channel::Receiver<Message<T>>,
}

static CHANNEL: Lazy<(
    ChannelContainer<RequestMessageData>,
    ChannelContainer<ResponseMessageData>,
)> = Lazy::new(|| {
    let (tx1, rx1) = async_channel::unbounded();
    let (tx2, rx2) = async_channel::unbounded();
    (
        ChannelContainer {
            sender: tx1,
            receiver: rx1,
        },
        ChannelContainer {
            sender: tx2,
            receiver: rx2,
        },
    )
});

static RUNNER: std::sync::OnceLock<std::thread::JoinHandle<()>> = std::sync::OnceLock::new();

pub fn channel() -> (
    async_channel::Sender<Message<RequestMessageData>>,
    async_channel::Receiver<Message<ResponseMessageData>>,
) {
    match RUNNER.get() {
        Some(handle) => {
            if handle.is_finished() {
                panic!("Runner thread has finished unexpectedly");
            }
        }
        None => {
            let handle = std::thread::spawn(runner_entry);
            RUNNER.set(handle).unwrap();
        }
    }

    let request = CHANNEL.0.clone();
    let response = CHANNEL.1.clone();
    (request.sender.clone(), response.receiver.clone())
}

fn runner_entry() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime");

    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        default_hook(panic_info);

        let channels = CHANNEL.clone();
        let sender = channels.1.sender.clone();
        sender
            .send_blocking(Message {
                nonce: Uuid::new_v4(),
                message: ResponseMessageData::Panic(format!("{:?}", panic_info)),
            })
            .expect("Failed to send panic message");
    }));
    rt.block_on(runner_entry_inner());
}

async fn runner_entry_inner() {
    let source = boa_engine::Source::from_bytes(include_str!("./utaformatix.js"));
    let queue = std::rc::Rc::new(crate::job_queue::TokioJobQueue::default());
    let mut context = boa_engine::Context::builder()
        .job_queue(queue)
        .build()
        .unwrap();

    context
        .register_global_builtin_callable(
            js_string!("sleep"),
            2,
            NativeFunction::from_async_fn(crate::js_impls::sleep),
        )
        .expect("Failed to register sleep function");
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

    let channels = CHANNEL.clone();
    let (sender, receiver) = (channels.1.sender.clone(), channels.0.receiver.clone());
    loop {
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
                info!("Completed parsing: {:?}", result);
                sender
                    .send_blocking(Message {
                        nonce,
                        message: ResponseMessageData::Parse(result),
                    })
                    .expect("Failed to send response");
            }
            _ => {}
        }
    }
}

async fn parse_single(
    utaformatix: &mut boa_engine::JsObject,
    context: &mut boa_engine::Context,
    format: Format,
    data: Vec<u8>,
    options: ParseOptions,
) -> Result<UfData> {
    let boa_engine::JsValue::Object(uint8array) = context
        .global_object()
        .get(js_string!("Uint8Array"), context)
        .unwrap()
    else {
        return Err(anyhow!("Failed to get Uint8Array").into());
    };
    let data = uint8array
        .construct(
            &[
                boa_engine::object::builtins::JsUint8Array::from_iter(data, context)
                    .map_err(|e| anyhow!("Failed to create Uint8Array: {:?}", e))?
                    .into(),
            ],
            None,
            context,
        )
        .map_err(|e| anyhow!("Failed to create Uint8Array: {:?}", e))?;
    let boa_engine::JsValue::Object(parser) = utaformatix
        .get(
            match format {
                Format::StandardMid => js_string!("parseStandardMid"),
                Format::MusicXml => js_string!("parseMusicXml"),
                Format::Ccs => js_string!("parseCcs"),
                Format::Dv => js_string!("parseDv"),
                Format::Ustx => js_string!("parseUstx"),
                Format::Ppsf => js_string!("parsePpsf"),
                Format::S5p => js_string!("parseS5p"),
                Format::Svp => js_string!("parseSvp"),
                Format::UfData => js_string!("parseUfData"),
                Format::VocaloidMid => js_string!("parseVocaloidMid"),
                Format::Vsq => js_string!("parseVsq"),
                Format::Vsqx => js_string!("parseVsqx"),
                Format::Vpr => js_string!("parseVpr"),
                _ => return Err(anyhow!("Unsupported format: {:?}", format).into()),
            },
            context,
        )
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

    let result = result.map_err(|e| {
        let value = e.to_opaque(context);
        for (error, name) in [
            (Error::EmptyProject, js_string!("EmptyProjectException")),
            (Error::IllegalFile, js_string!("IllegalFileException")),
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
        let value = value.to_json(context).expect("Failed to convert to JSON");
        anyhow!("Unexpected error: {:?}", value).into()
    })?;
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
