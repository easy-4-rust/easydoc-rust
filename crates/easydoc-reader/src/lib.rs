//! DOCX/DOC document reader for `easydoc-rs`.
//!
//! Provides text extraction, table reading, and streaming document analysis,
//! wrapping `office_oxide` for backend parsing.

#![deny(unsafe_code)]

use std::path::Path;

use easydoc_core::{DocxRow, Result};

mod builder;
mod extractor;
mod listener;

pub use builder::read_builder::DocReadBuilder;
pub use extractor::{DocumentFormat, detect_format};
pub use listener::collect::CollectListener;

/// Synchronously reads all plain text from a document.
///
/// Auto-detects DOCX/DOC format via `office_oxide`.
///
/// # Errors
///
/// Returns I/O or format errors if the file cannot be read.
pub fn read_text(path: &Path) -> Result<String> {
    extractor::text::extract_text(path)
}

/// Synchronously reads all tables from a document, deserialising each into `Vec<T>`.
///
/// # Errors
///
/// Returns I/O, format, or conversion errors.
pub fn read_tables<T: DocxRow>(path: &Path) -> Result<Vec<Vec<T>>> {
    extractor::table::extract_tables::<T>(path)
}
