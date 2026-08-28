//! Errors. Each variant is a distinct failure the caller can act on — the UI
//! shows a different message for "this EPUB has no spine" than for "the file
//! we wrote did not read back as the text you typed".

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocError {
    #[error("io error on {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The path is not one of the formats this crate handles.
    #[error("unsupported document format: {0}")]
    Unsupported(String),

    /// The ZIP / bundle container could not be read or rebuilt.
    #[error("container error: {0}")]
    Container(String),

    /// The container opened but its contents did not match the format.
    #[error("parse error: {0}")]
    Parse(String),

    /// The edit asked for a structural change the writer cannot make safely.
    #[error("{0}")]
    Structure(String),

    /// The rewritten document did not read back as the text that was asked for.
    /// The original file is left untouched when this is returned.
    #[error("write verification failed: {0}")]
    Verification(String),
}

impl DocError {
    pub fn io(path: impl AsRef<std::path::Path>, source: std::io::Error) -> Self {
        DocError::Io {
            path: path.as_ref().display().to_string(),
            source,
        }
    }

    /// Short machine-readable kind, for the UI and for logs.
    pub fn kind(&self) -> &'static str {
        match self {
            DocError::Io { .. } => "io",
            DocError::Unsupported(_) => "unsupported",
            DocError::Container(_) => "container",
            DocError::Parse(_) => "parse",
            DocError::Structure(_) => "structure",
            DocError::Verification(_) => "verification",
        }
    }
}
