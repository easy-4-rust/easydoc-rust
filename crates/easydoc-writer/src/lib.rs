//! DOCX 文档写入器。
//!
//! 在 `docx-rs` 之上提供 Fluent 构建器 API，对标 `easyexcel-writer` 的模式。
//!
//! 对应 Java: `com.alibaba.excel` (`EasyExcel` 写入层)

#![deny(unsafe_code)]

mod builder;
pub mod content_renderer;
mod doc_editor;
mod executor;
mod handler;
pub mod math_omml;
mod style;
pub mod util;

mod doc_image;
mod paragraph;
mod run;
mod table;

pub use builder::doc_builder::DocBuilder;
pub use builder::table_builder::TableWriteBuilder;
pub use doc_editor::DocEditor;
pub use executor::table_executor::TableWriteExecutor;
pub use executor::write_executor::DocWriteExecutor;
pub use handler::DocWriteHandler;
pub use style::auto_width::AutoWidthStrategy;
pub use style::banded_rows::BandedRowsStrategy;

pub use doc_image::DocImage;
pub use paragraph::Paragraph;
pub use run::Run;
pub use table::Table;

// Re-export key types for the facade
pub use easydoc_core::{
    CellData, Color, DocValue, DocumentBlock, DocumentContent, DocumentTextRun, DocxRow,
    FontConfig, HorizontalAlignment, ParagraphStyle, TableStyle,
};
