use crate::{
    base::UtaFormatix,
    error::Result,
    model::{
        ConvertJapaneseLyricsOptions, GenerateOptions, JapaneseLyricsType, ParseOptions, UfData,
    },
};
use duplicate::duplicate_item;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::warn;

#[derive(Debug, Clone)]
/// The project data.
/// This struct allows you to interact with the project, with object-oriented methods.
pub struct Project {
    pub data: UfData,
}

impl Serialize for Project {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Project {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            data: UfData::deserialize(deserializer)?,
        })
    }
}

static UTAFORMATIX: Lazy<Mutex<UtaFormatix>> = Lazy::new(|| Mutex::new(UtaFormatix::new()));

impl Project {
    /// Creates a new instance of `Project`.
    pub fn new(data: UfData) -> Self {
        Self { data }
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
    pub async fn fn_name(data: &[u8], options: ParseOptions) -> Result<Self> {
        let utaformatix = UTAFORMATIX.lock().await;
        utaformatix.fn_name(data, options).await.map(Self::new)
    }

    #[duplicate_item(
        fn_name     format_enum   kind;
        [parse_ust] [Format::Ust] ["UTAU's project"];
    )]
    #[doc = "Parses a "]
    #[doc = kind]
    #[doc = " file."]
    pub async fn fn_name(data: &[u8], options: ParseOptions) -> Result<Self> {
        let utaformatix = UTAFORMATIX.lock().await;
        utaformatix.fn_name(&[data], options).await.map(Self::new)
    }

    #[duplicate_item(
        fn_name              original_fn_name format_enum   kind;
        [parse_ust_multiple] [parse_ust]      [Format::Ust] ["UTAU's project"];
    )]
    #[doc = "Parses a "]
    #[doc = kind]
    #[doc = " file."]
    /// You can pass multiple files to parse at once, each file will be parsed as a track.
    pub async fn fn_name(data: &[&[u8]], options: ParseOptions) -> Result<Self> {
        let utaformatix = UTAFORMATIX.lock().await;
        utaformatix
            .original_fn_name(data, options)
            .await
            .map(Self::new)
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
    pub async fn fn_name(&self, options: GenerateOptions) -> Result<Vec<u8>> {
        let utaformatix = UTAFORMATIX.lock().await;
        utaformatix.fn_name(&self.data, options).await
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
    pub async fn fn_name(&self, options: GenerateOptions) -> Result<Vec<Vec<u8>>> {
        let utaformatix = UTAFORMATIX.lock().await;
        utaformatix.fn_name(&self.data, options).await
    }

    /// Analyzes the type of Japanese lyrics.
    /// Returns `None` if the lyrics type cannot be determined.
    pub async fn analyze_japanese_lyrics_type(&self) -> Result<Option<JapaneseLyricsType>> {
        let utaformatix = UTAFORMATIX.lock().await;
        utaformatix
            .analyze_japanese_lyrics_type(self.data.clone())
            .await
    }

    /// Converts Japanese lyrics.
    pub async fn convert_japanese_lyrics(
        &self,
        source_type: Option<JapaneseLyricsType>,
        target_type: JapaneseLyricsType,
        options: ConvertJapaneseLyricsOptions,
    ) -> Result<Self> {
        let source_type = if let Some(source_type) = source_type {
            Some(source_type)
        } else {
            self.analyze_japanese_lyrics_type().await?
        };
        if source_type.is_none() {
            warn!("Failed to determine the source type of the Japanese lyrics");
            return Ok(Self::new(self.data.clone()));
        }
        let utaformatix = UTAFORMATIX.lock().await;
        utaformatix
            .convert_japanese_lyrics(
                self.data.clone(),
                source_type.unwrap(),
                target_type,
                options,
            )
            .await
            .map(Self::new)
    }
}
