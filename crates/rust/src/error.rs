use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("The project is empty.")]
    EmptyProject,
    #[error("The file is illegal.")]
    IllegalFile,
    #[error("The note position is illegal.")]
    IllegalNotePosition,
    #[error("Notes are overlapping.")]
    NotesOverlapping,
    #[error("Unsupported file format.")]
    UnsupportedFileFormat,
    #[error("Unsupported legacy ppsf file format.")]
    UnsupportedLegacyPpsf,

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::Unexpected(e.to_string())
    }
}
