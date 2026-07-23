//! Metadata types for document structure and table schema.
//!
//! These types describe the structure of tables and documents,
//! analogous to `ExcelColumn` / `WriteSheet` / `ReadSheet` in `easyexcel-rust`.

/// Table column descriptor.
pub mod column;
/// Document-level metadata.
pub mod document;

pub use column::TableColumn;
pub use document::DocumentMeta;
