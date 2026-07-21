//! Public facade for easy DOC/DOCX document operations.
//!
//! `easydoc` provides a fluent, annotation-driven API for creating, reading,
//! and template-filling Word documents — the DOC/DOCX counterpart to `easyexcel-rs`.
//!
//! # Quick start
//!
//! ```ignore
//! use easydoc::prelude::*;
//!
//! // Quick table write
//! EasyDoc::write_table("users.docx", &users)
//!     .title("User Report")
//!     .do_write()?;
//!
//! // Build a full document
//! EasyDoc::document("report.docx")
//!     .add_heading("Summary", HeadingLevel::H1)
//!     .add_paragraph("This is the report content.")
//!     .add_table(Table::from_data(&data))
//!     .build()?
//!     .save()?;
//!
//! // Read text
//! let text = EasyDoc::read_text("document.docx")?;
//! ```
//!
//! # Crate structure
//!
//! This facade re-exports everything from:
//! - `easydoc-core` — core types, traits, converters, styles
//! - `easydoc-derive` — `#[derive(DocxRow)]` proc-macro
//! - `easydoc-writer` — DOCX document generation
//! - `easydoc-reader` — DOCX/DOC document reading
//! - `easydoc-template` — template placeholder fill

mod easy_doc;

// Re-export everything from sub-crates
pub use easy_doc::EasyDoc;
pub use easydoc_core::*;
pub use easydoc_derive::DocxRow;
pub use easydoc_reader::*;
pub use easydoc_template::*;
pub use easydoc_writer::*;

/// Java-compatible alias for [`EasyDoc`].
pub type EasyDocFactory = EasyDoc;

/// Prelude module with the most commonly used types.
pub mod prelude {
    pub use super::EasyDoc;
    pub use easydoc_core::{
        CellData, Color, DocError, DocValue, DocxRow, ErrorAction, FontConfig, HeadingLevel,
        HorizontalAlignment, ParagraphStyle, Result, RowData, TableColumn, TableData, TableStyle,
    };
    pub use easydoc_derive::DocxRow as DocxRowDerive;
    pub use easydoc_writer::{DocBuilder, Paragraph, Run, TableWriteBuilder};
}
