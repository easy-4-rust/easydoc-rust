//! 文档结构与表格 schema 的元数据类型。
//!
//! 这些类型描述表格和文档的结构，对标 easyexcel-rust 的
//! `ExcelColumn` / `WriteSheet` / `ReadSheet`。
//!
//! 对应 Java: com.alibaba.excel.metadata

/// Table column descriptor.
pub mod column;
/// Document-level metadata.
pub mod document;

pub use column::TableColumn;
pub use document::DocumentMeta;
