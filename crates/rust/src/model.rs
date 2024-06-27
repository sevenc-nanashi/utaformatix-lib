use serde::{Deserialize, Serialize};

/// Represents the format of the data.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Format {
    /// Standard MIDI file. (`.mid`)
    StandardMid,
    /// MusicXML file. (`.musicxml`)
    MusicXml,
    /// CeVIO's project file. (`.ccs`)
    Ccs,
    /// DeepVocal's project file. (`.dv`)
    Dv,
    /// OpenUtau's project file. (`.ustx`)
    Ustx,
    /// Piapro Studio's project file. (`.ppsf`)
    Ppsf,
    /// Old Synthesizer V's project file. (`.s5p`)
    S5p,
    /// Synthesizer V's project file. (`.svp`)
    Svp,
    /// UtaFormatix data. (`.ufdata`)
    UfData,
    /// UTAU's project file. (`.ust`)
    Ust,
    /// VOCALOID 1's project file. (`.mid`)
    VocaloidMid,
    /// VOCALOID 2's project file. (`.vsq`)
    Vsq,
    /// VOCALOID 3/4's project file. (`.vsqx`)
    Vsqx,
    /// VOCALOID 5's project file. (`.vpr`)
    Vpr,
}

/// Represents the options for parsing data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseOptions {
    /// Whether to parse the pitch data.
    pub pitch: bool,
    /// The default lyric to use when the note's lyric is empty.
    pub default_lyric: String,
}
impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            pitch: true,
            default_lyric: "あ".to_string(),
        }
    }
}

/// Represents the root document object of UtaFormatix data.
///
/// See: <https://github.com/sdercolin/utaformatix-data?tab=readme-ov-file#root-document-object>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UfData {
    /// Format version of the data.
    format_version: i32,
    // TODO: Support multiple versions: https://github.com/serde-rs/serde/issues/745
    /// Project object.
    project: Project,
}

/// Represents the project object of UtaFormatix data v1.
///
/// See: <https://github.com/sdercolin/utaformatix-data?tab=readme-ov-file#project-object>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Project name.
    pub name: String,
    /// Track list.
    pub tracks: Vec<Track>,
    /// Time signatures.
    pub time_signatures: Vec<TimeSignature>,
    /// Tempo changes.
    pub tempos: Vec<Tempo>,
    /// Count of measure prefixes (measures that cannot contain notes, restricted by some editors).
    pub measure_prefix: i32,
}

/// Represents a track object of UtaFormatix data v1.
///
/// See: <https://github.com/sdercolin/utaformatix-data?tab=readme-ov-file#track-object>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// Track name.
    pub name: String,
    /// Note list.
    pub notes: Vec<Note>,
    /// Pitch data.
    pub pitch: Option<Pitch>,
}

/// Represents a note object of UtaFormatix data v1.
///
/// See: <https://github.com/sdercolin/utaformatix-data?tab=readme-ov-file#note-object>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    /// Semitone value of the note's key (Center C = 60).
    pub key: i32,
    /// Tick position of the note's start.
    pub tick_on: i64,
    /// Tick position of the note's end.
    pub tick_off: i64,
    /// Lyric.
    pub lyric: String,
    /// Phoneme (if available).
    pub phoneme: Option<String>,
}

/// Represents a pitch object of UtaFormatix data v1.
///
/// See: <https://github.com/sdercolin/utaformatix-data?tab=readme-ov-file#pitch-object>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pitch {
    /// Tick positions of the data points.
    pub ticks: Vec<i32>,
    /// Semitone values of the data points.
    /// When [Pitch::is_absolute] is true, `null` can be included to represent default values.
    pub values: Vec<Option<f64>>,
    /// Whether the pitch values are absolute or relative to the note's key.
    pub is_absolute: bool,
}

/// Represents a time signature object of UtaFormatix data v1.
///
/// See: <https://github.com/sdercolin/utaformatix-data?tab=readme-ov-file#time-signature-object>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSignature {
    /// Measure (bar) position of the time signature.
    pub measure_position: i32,
    /// Beats per measure.
    pub numerator: i32,
    /// Note value per beat.
    pub denominator: i32,
}

/// Represents a tempo object of UtaFormatix data v1.
///
/// See: <https://github.com/sdercolin/utaformatix-data?tab=readme-ov-file#tempo-object>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tempo {
    /// Tick position of the tempo change.
    pub tick_position: i64,
    /// Tempo in beats-per-minute
    pub bpm: i32,
}
