//! Error types shared by all easydoc backends.

use std::path::PathBuf;

/// Result alias used by easydoc APIs.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced while constructing or rendering a document.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An input value does not satisfy the document model constraints.
    #[error("invalid document input: {0}")]
    InvalidInput(String),
    /// A named style was referenced but not registered.
    #[error("unknown style: {0}")]
    UnknownStyle(String),
    /// An input or output operation failed.
    #[error("I/O operation failed for {path}: {source}")]
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The selected format backend failed.
    #[error("document backend failed: {0}")]
    Backend(String),
}

impl Error {
    /// Creates a path-aware I/O error.
    #[must_use]
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
