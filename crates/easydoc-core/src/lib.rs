//! easydoc-rust 核心数据模型与扩展点。
//!
//! 对应 Java: `com.alibaba.excel` (`EasyExcel` 4.0.3 核心层)

#![deny(unsafe_code)]

pub mod converter;
pub mod document;
pub mod error;
pub mod metadata;
#[cfg(feature = "serde")]
pub mod serde_bridge;
pub mod style;
pub mod traits;
pub mod types;

pub use converter::{ConverterRegistry, ErasedConverter};
pub use document::{
    DocumentBlock, DocumentContent, DocumentImage, DocumentList, DocumentListItem, DocumentTable,
    DocumentTableCell, DocumentTableRow, DocumentTextRun,
};
pub use error::{DocError, Result};
pub use metadata::{DocumentMeta, TableColumn};
pub use style::{Color, FontConfig, ParagraphStyle, TableStyle};
pub use traits::{
    CellContext, ContentCollector, DocConverter, DocReadContext, DocReadListener, DocWriteContext,
    DocWriteHandler, DocumentEvent, DocumentReader, DocxRow, EventSink, ParagraphContext,
    TableWriteContext,
};
pub use types::{
    CellData, DocValue, ErrorAction, HeadingLevel, HorizontalAlignment, ImageData, RichRun,
    RowData, TableData,
};

/// 文档分区类型，用于 Section 块。
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DocumentSection {
    /// 连续分区（不换页）。
    Continuous,
    /// 下一页开始新分区。
    NextPage,
    /// 下一列开始新分区。
    NextColumn,
    /// 偶数页开始新分区。
    EvenPage,
    /// 奇数页开始新分区。
    OddPage,
}
