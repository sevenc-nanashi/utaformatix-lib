use crate::{
    error::Result,
    model::{Format, GenerateOptions, JapaneseLyricsType, ParseOptions, UfData},
    ConvertJapaneseLyricsOptions,
};
use std::cell::OnceCell;

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
            runtime::runner_entry(request_receiver, response_sender);
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

#[cfg(feature = "quickjs")]
#[path = "process/quickjs.rs"]
mod runtime;

#[cfg(all(feature = "boajs", not(feature = "quickjs")))]
#[path = "process/boa.rs"]
mod runtime;
