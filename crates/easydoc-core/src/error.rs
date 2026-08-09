//! Unified error type for easydoc-rust.
//!
//! Mirrors the single-enum error pattern from `easyexcel-rust`.

use thiserror::Error;

/// The single, flat error enum used by all `easydoc-rust` crates.
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

/// The standard `Result` type alias used throughout `easydoc-rust`.
pub type Result<T> = std::result::Result<T, DocError>;

impl From<zip::result::ZipError> for DocError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Zip(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err = DocError::Io(io_err);
        assert!(format!("{}", err).contains("I/O error"));
    }

    #[test]
    fn error_display_zip() {
        let err = DocError::Zip("corrupt".into());
        assert!(format!("{}", err).contains("ZIP error"));
    }

    #[test]
    fn error_display_format() {
        let err = DocError::Format("bad xml".into());
        assert!(format!("{}", err).contains("Format error"));
    }

    #[test]
    fn error_display_template() {
        let err = DocError::Template {
            placeholder: "name".into(),
            message: "not found".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("name"));
        assert!(s.contains("not found"));
    }

    #[test]
    fn error_display_conversion() {
        let err = DocError::Conversion {
            field: "age".into(),
            value: "abc".into(),
            message: "not a number".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("age"));
        assert!(s.contains("abc"));
    }

    #[test]
    fn error_display_unsupported() {
        let err = DocError::Unsupported("macro".into());
        assert!(format!("{}", err).contains("Unsupported operation"));
    }

    #[test]
    fn error_display_document() {
        let err = DocError::Document("corrupted".into());
        assert!(format!("{}", err).contains("Document error"));
    }

    #[test]
    fn error_from_zip_error() {
        let zip_err = zip::result::ZipError::FileNotFound;
        let err: DocError = zip_err.into();
        assert!(matches!(err, DocError::Zip(_)));
    }

    #[test]
    fn error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: DocError = io_err.into();
        assert!(matches!(err, DocError::Io(_)));
    }

    #[test]
    fn error_debug() {
        let err = DocError::Document("test".into());
        let dbg = format!("{:?}", err);
        assert!(dbg.contains("Document"));
    }
}
