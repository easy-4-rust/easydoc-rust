//! Unified error type for easydoc-rs.
//!
//! Mirrors the single-enum error pattern from `easyexcel-rs`.

use thiserror::Error;

/// The single, flat error enum used by all `easydoc-rs` crates.
///
/// Java `easyexcel` splits errors across seven `RuntimeException` subclasses;
/// this enum collapses them into one idiomatic Rust type.
#[derive(Debug, Error)]
pub enum DocError {
    /// I/O error wrapping `std::io::Error`.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// ZIP package error from docx-rs packaging.
    #[error("ZIP error: {0}")]
    Zip(String),

    /// Invalid or unsupported document format.
    #[error("Format error: {0}")]
    Format(String),

    /// Template placeholder could not be resolved or processed.
    #[error("Template error at placeholder '{placeholder}': {message}")]
    Template {
        /// The placeholder token that caused the error.
        placeholder: String,
        /// Human-readable description.
        message: String,
    },

    /// A cell or field value could not be converted to/from the target type.
    #[error("Conversion error: field '{field}', value '{value}': {message}")]
    Conversion {
        /// The field or column name.
        field: String,
        /// The value that failed conversion.
        value: String,
        /// Human-readable description.
        message: String,
    },

    /// The requested operation is not supported for this format or configuration.
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// Generic document-level error.
    #[error("Document error: {0}")]
    Document(String),
}

/// The standard `Result` type alias used throughout `easydoc-rs`.
pub type Result<T> = std::result::Result<T, DocError>;

impl From<zip::result::ZipError> for DocError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Zip(e.to_string())
    }
}
