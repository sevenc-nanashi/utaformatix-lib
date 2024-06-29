use crate::error::Result;
use crate::model::UfData;
use crate::{
    model::{Format, GenerateOptions, ParseOptions},
    process::SyncThread,
};
use duplicate::duplicate_item;
use tracing::info;

/// Represents the main interface to UtaFormatix.
pub struct UtaFormatix {
    inner: SyncThread,
}

impl Default for UtaFormatix {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! send_and_receive {
    ($self:ident, $message:expr, $response:ident) => {{
        let sent_nonce = $message.nonce;
        $self
            .inner
            .request_sender
            .send($message)
            .await
            .map_err(anyhow::Error::from)?;
        info!("Sent message, waiting for response");
        let result: Result<_> = loop {
            let crate::process::Message { message, nonce } = $self
                .inner
                .response_receiver
                .recv()
                .await
                .map_err(anyhow::Error::from)?;
            if matches!(message, crate::process::ResponseMessageData::Panic) {
                panic!("JS thread panicked!");
            } else if sent_nonce == nonce {
                let crate::process::ResponseMessageData::$response(response) = message else {
                    panic!("Unexpected message: {:?}", message);
                };
                break response;
            }
        };
        info!("Received response");

        result
    }};
}

impl UtaFormatix {
    /// Creates a new instance of `UtaFormatix`.
    pub fn new() -> Self {
        let inner = SyncThread::new();
        Self { inner }
    }

    #[duplicate_item(
        fn_name              format_enum           kind;
        [parse_standard_mid] [Format::StandardMid] ["Standard MIDI"];
        [parse_music_xml]    [Format::MusicXml]    ["MusicXML"];
        [parse_ccs]          [Format::Ccs]         ["CeVIO's project"];
        [parse_dv]           [Format::Dv]          ["DeepVocal's project"];
        [parse_ustx]         [Format::Ustx]        ["OpenUtau's project"];
        [parse_ppsf]         [Format::Ppsf]        ["Piapro Studio's project"];
        [parse_s5p]          [Format::S5p]         ["Old Synthesizer V's project"];
        [parse_svp]          [Format::Svp]         ["Synthesizer V's project"];
        [parse_tssln]        [Format::Tssln]       ["VoiSona's project"];
        [parse_uf_data]      [Format::UfData]      ["UtaFormatix data"];
        [parse_vocaloid_mid] [Format::VocaloidMid] ["VOCALOID 1's project"];
        [parse_vsq]          [Format::Vsq]         ["VOCALOID 2's project"];
        [parse_vsqx]         [Format::Vsqx]        ["VOCALOID 3/4's project"];
        [parse_vpr]          [Format::Vpr]         ["VOCALOID 5's project"];
    )]
    #[doc = "Parses a "]
    #[doc = kind]
    #[doc = " file."]
    pub async fn fn_name(
        &self,
        data: &[u8],
        options: ParseOptions,
    ) -> Result<crate::model::UfData> {
        let message =
            crate::process::Message::new(crate::process::RequestMessageData::ParseSingle {
                data: data.to_vec(),
                options,
                format: format_enum,
            });
        send_and_receive!(self, message, Parse)
    }

    #[duplicate_item(
        fn_name              format_enum   kind;
        [parse_ust]          [Format::Ust]["UTAU's project"];
    )]
    #[doc = "Parses a "]
    #[doc = kind]
    #[doc = " file."]
    pub async fn fn_name(
        &self,
        data: &[&[u8]],
        options: ParseOptions,
    ) -> Result<crate::model::UfData> {
        let message =
            crate::process::Message::new(crate::process::RequestMessageData::ParseMultiple {
                data: data.iter().map(|d| d.to_vec()).collect(),
                options,
                format: format_enum,
            });

        send_and_receive!(self, message, Parse)
    }

    #[duplicate_item(
        fn_name                  format_enum          kind;
        [generate_standard_mid] [Format::StandardMid] ["Standard MIDI"];
        [generate_ccs]          [Format::Ccs]         ["CeVIO's project"];
        [generate_dv]           [Format::Dv]          ["DeepVocal's project"];
        [generate_ustx]         [Format::Ustx]        ["OpenUtau's project"];
        [generate_s5p]          [Format::S5p]         ["Old Synthesizer V's project"];
        [generate_svp]          [Format::Svp]         ["Synthesizer V's project"];
        [generate_tssln]        [Format::Tssln]       ["VoiSona's project"];
        [generate_uf_data]      [Format::UfData]      ["UtaFormatix data"];
        [generate_vocaloid_mid] [Format::VocaloidMid] ["VOCALOID 1's project"];
        [generate_vsq]          [Format::Vsq]         ["VOCALOID 2's project"];
        [generate_vsqx]         [Format::Vsqx]        ["VOCALOID 3/4's project"];
        [generate_vpr]          [Format::Vpr]         ["VOCALOID 5's project"];
    )]
    #[doc = "Generates a "]
    #[doc = kind]
    #[doc = " file."]
    pub async fn fn_name(&self, data: UfData, options: GenerateOptions) -> Result<Vec<u8>> {
        let message =
            crate::process::Message::new(crate::process::RequestMessageData::GenerateSingle {
                data,
                options,
                format: format_enum,
            });

        send_and_receive!(self, message, GenerateSingle)
    }

    #[duplicate_item(
        fn_name                  format_enum          kind;
        [generate_music_xml]    [Format::MusicXml]    ["MusicXML"];
        [generate_ust]          [Format::Ust]         ["UTAU's project"];
    )]
    #[doc = "Generates a "]
    #[doc = kind]
    #[doc = " file."]
    /// Returns the bytes of the generated file, each representing a track.
    pub async fn fn_name(&self, data: UfData, options: GenerateOptions) -> Result<Vec<Vec<u8>>> {
        let message =
            crate::process::Message::new(crate::process::RequestMessageData::GenerateMultiple {
                data,
                options,
                format: format_enum,
            });

        send_and_receive!(self, message, GenerateMultiple)
    }
}
