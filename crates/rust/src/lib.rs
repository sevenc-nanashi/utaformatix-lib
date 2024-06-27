use duplicate::duplicate_item;
use error::Result;
use model::{Format, ParseOptions};

mod error;
mod job_queue;
mod js_impls;
mod model;
mod process;

#[duplicate_item(
    fn_name              format_enum;
    [parse_standard_mid] [Format::StandardMid];
)]
pub async fn fn_name(data: &[u8], options: ParseOptions) -> Result<model::UfData> {
    let (tx, rx) = process::channel();

    let message = process::Message::new(process::RequestMessageData::ParseSingle {
        data: data.to_vec(),
        options,
        format: format_enum,
    });
    let sent_nonce = message.nonce;
    tx.send(message).await.map_err(anyhow::Error::from)?;
    let result: Result<model::UfData> = loop {
        let process::Message { message, nonce } = rx.recv().await.map_err(anyhow::Error::from)?;
        if let process::ResponseMessageData::Panic(message) = message {
            panic!("JS thread panicked: {}", message);
        } else if sent_nonce == nonce {
            let process::ResponseMessageData::Parse(response) = message else {
                panic!("Unexpected message: {:?}", message);
            };
            break response;
        }
    };

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[duplicate_item(
        test_name                 function             path;
        [test_parse_standard_mid] [parse_standard_mid] ["generated/standard.mid"];
    )]
    #[tokio::test]
    async fn test_name() {
        let data = include_bytes!(concat!("../utaformatix-ts/testAssets/", path));
        let options = ParseOptions::default();
        let result = function(data, options).await;

        let parsed = result.expect("Failed to parse data");

        dbg!(&parsed);
    }
}
