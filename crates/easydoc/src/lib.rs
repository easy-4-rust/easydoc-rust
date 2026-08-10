//! Public facade for easy DOC/DOCX document operations.

#![deny(unsafe_code)]

mod easy_doc;

pub use easy_doc::EasyDoc;

// Core types (explicit)
pub use easydoc_core::{
    CellContext, CellData, Color, ContentCollector, ConverterRegistry, DocConverter, DocError,
    DocReadContext, DocReadListener, DocValue, DocWriteContext, DocWriteHandler, DocumentBlock,
    DocumentContent, DocumentEvent, DocumentImage, DocumentList, DocumentListItem, DocumentMeta,
    DocumentReader, DocumentSection, DocumentTable, DocumentTableCell, DocumentTableRow,
    DocumentTextRun, DocxRow, ErrorAction, EventSink, FontConfig, HeadingLevel,
    HorizontalAlignment, ImageData, ParagraphContext, ParagraphStyle, Result, RichRun, RowData,
    TableColumn, TableData, TableStyle, TableWriteContext,
};

pub use easydoc_derive::DocxRow as DocxRowDerive;
pub use easydoc_markdown::{
    ConversionWarning, ExtractedAsset, MarkdownBuilder, MarkdownOptions, MarkdownResult,
};
pub use easydoc_ooxml::{AtomicFile, PackageLimits, PackageRewriter};
pub use easydoc_reader::{
    CollectListener, DocReadBuilder, DocumentFormat, DocxSaxReader, ViewMode, detect_format,
    render_view,
};
pub use easydoc_template::{FillConfig, FillDirection, Placeholder, TemplateFillBuilder};
pub use easydoc_writer::content_renderer;
pub use easydoc_writer::{
    AutoWidthStrategy, BandedRowsStrategy, DocBuilder, DocEditor, DocImage, DocWriteExecutor,
    Paragraph, Run, Table, TableWriteBuilder, TableWriteExecutor,
};

/// Java-compatible alias for `EasyDoc`.
pub type EasyDocFactory = EasyDoc;

/// Prelude module with the most commonly used types.
pub mod prelude {
    pub use super::EasyDoc;
    pub use easydoc_core::{
        CellData, Color, ContentCollector, DocError, DocValue, DocumentBlock, DocumentContent,
        DocumentTextRun, DocxRow, ErrorAction, EventSink, FontConfig, HeadingLevel,
        HorizontalAlignment, ParagraphStyle, Result, RowData, TableColumn, TableData, TableStyle,
    };
    pub use easydoc_derive::DocxRow as DocxRowDerive;
    pub use easydoc_markdown::{MarkdownBuilder, MarkdownOptions, MarkdownResult};
    pub use easydoc_reader::{DocxSaxReader, ViewMode, render_view};
    pub use easydoc_writer::{DocBuilder, Paragraph, Run, Table, TableWriteBuilder};
}
