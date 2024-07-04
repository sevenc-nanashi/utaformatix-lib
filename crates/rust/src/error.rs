use strum::EnumString;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Error)]
/// Represents an error that can occur during the conversion process.
pub enum Error {
    #[error("The project is empty.")]
    /// The project is empty.
    EmptyProject,
    #[error("The file is illegal.")]
    /// The file is illegal.
    IllegalFile(IllegalFile),
    #[error("The note position is illegal.")]
    /// The note position is illegal.
    IllegalNotePosition,
    #[error("Notes are overlapping.")]
    /// Notes are overlapping.
    NotesOverlapping,
    #[error("Unsupported file format.")]
    /// Unsupported file format.
    UnsupportedFileFormat,
    #[error("Unsupported legacy ppsf file format.")]
    /// Unsupported legacy ppsf file format.
    UnsupportedLegacyPpsf,

    #[error("Unexpected error: {0}")]
    /// An unexpected error occurred.
    Unexpected(String),
}

#[derive(Debug, Clone, Error, EnumString)]
/// Represents an error that can occur when the file is illegal.
pub enum IllegalFile {
    #[error("Unknown vsq version.")]
    /// Unknown vsq version.
    UnknownVsqVersion,
    #[error("Failed to find root of XML.")]
    /// Failed to find root of XML.
    XmlRootNotFound,
    #[error("Failed to find element in XML.")]
    /// Failed to find element in XML.
    XmlElementNotFound { name: String },
    #[error("The value of XML element is illegal.")]
    /// The value of XML element is illegal.
    IllegalXmlValue { name: String },
    #[error("The attribute of XML element is illegal.")]
    /// The attribute of XML element is illegal.
    IllegalXmlAttribute { name: String, attribute: String },
    #[error("Illegal MIDI file.")]
    /// Illegal MIDI file.
    IllegalMidiFile,
    #[error("Illegal tssln file.")]
    /// Illegal tssln file.
    IllegalTsslnFile,
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::Unexpected(e.to_string())
    }
}
