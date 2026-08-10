//! DOCX/DOC 文档读取器。
//!
//! 提供文本提取、表格读取和流式文档分析，底层包装 `office_oxide` 解析器。
//!
//! 对应 Java: `com.alibaba.excel` (`EasyExcel` 读取层)

#![deny(unsafe_code)]

mod builder;
pub mod extractor;
mod listener;
mod read_document;
mod read_tables;
mod read_text;
pub mod security;
pub mod view;

pub use builder::read_builder::DocReadBuilder;
pub use extractor::numbering::Numbering;
pub use extractor::sax::DocxSaxReader;
pub use extractor::{DocumentFormat, detect_format};
pub use listener::collect::CollectListener;
pub use read_document::read_document;
pub use read_tables::read_tables;
pub use read_text::read_text;
pub use view::{ViewMode, render_view};
