//! Core data model and extension points for `easydoc-rs`.
//!
//! This crate provides the foundational types, traits, and converters that
//! all other `easydoc-rs` crates build upon. It mirrors the architecture of
//! `easyexcel-core` but for the DOC/DOCX domain.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

pub mod converter;
pub mod error;
pub mod metadata;
pub mod style;
pub mod traits;
pub mod types;

// Re-export the most commonly used items.
pub use converter::ConverterRegistry;
pub use error::{DocError, Result};
pub use metadata::{DocumentMeta, TableColumn};
pub use style::{Color, FontConfig, ParagraphStyle, TableStyle};
pub use traits::{
    CellContext, DocConverter, DocReadContext, DocReadListener, DocWriteContext, DocWriteHandler,
    DocxRow, ParagraphContext, TableWriteContext,
};
pub use types::{
    CellData, DocValue, ErrorAction, HeadingLevel, HorizontalAlignment, ImageData, RichRun,
    RowData, TableData,
};
