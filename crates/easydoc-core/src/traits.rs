//! Core extension traits for `easydoc-rust`.
//!
//! These four traits form the backbone of the extensibility system,
//! mirroring `ExcelRow`, `Converter<T>`, `ReadListener<T>`, and
//! `WriteHandler` from `easyexcel-rust`.

use crate::converter::ConverterRegistry;
use crate::error::Result;
use crate::metadata::TableColumn;
use crate::types::{CellData, DocValue, ErrorAction, RowData, TableData};

// ============================================================================
// DocxRow — struct ↔ table row mapping
// ============================================================================

/// Maps a Rust struct to/from a DOCX table row.
///
/// Analogous to `ExcelRow` in `easyexcel-rust`.
/// Generated automatically by `#[derive(DocxRow)]`.
///
/// # Example
///
/// ```ignore
/// #[derive(DocxRow)]
/// struct User {
///     #[docx(name = "Name", order = 0)]
///     name: String,
///     #[docx(name = "Age", order = 1)]
///     age: u32,
/// }
/// ```
pub trait DocxRow {
    /// Returns the column schema: header names, indices, and format hints.
    fn schema() -> &'static [TableColumn]
    where
        Self: Sized;

    /// Deserialises a row from raw cell values using default converters.
    fn from_row(row: &RowData) -> Result<Self>
    where
        Self: Sized;

    /// Deserialises a row using a custom converter registry.
    fn from_row_with_converters(row: &RowData, registry: &ConverterRegistry) -> Result<Self>
    where
        Self: Sized;

    /// Serialises self into a row of cell values using default converters.
    fn to_row(&self) -> Result<Vec<CellData>>;

    /// Serialises self into a row of cell values using a custom converter registry.
    fn to_row_with_converters(&self, registry: &ConverterRegistry) -> Result<Vec<CellData>>;
}

// ============================================================================
// DocConverter — bidirectional type conversion
// ============================================================================

/// Converts between a Rust type `T` and a [`DocValue`].
///
/// Analogous to `Converter<T>` in `easyexcel-rust`. Register custom converters
/// via [`ConverterRegistry`] or the builder's `register_converter` method.
pub trait DocConverter<T> {
    /// Returns the `TypeId` this converter handles.
    fn support_type() -> std::any::TypeId
    where
        Self: Sized;

    /// Converts a Rust value into a document value for writing.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DocError::Conversion`] if the value cannot be converted.
    fn to_doc_value(&self, value: &T, column: &TableColumn) -> Result<DocValue>;

    /// Converts a document value back into a Rust value for reading.
    ///
    /// # Errors
    ///
    /// Returns [`crate::DocError::Conversion`] if the value cannot be converted.
    fn from_doc_value(&self, value: &DocValue, column: &TableColumn) -> Result<T>;
}

// ============================================================================
// DocReadListener — streaming read callbacks
// ============================================================================

/// Context passed to read listener callbacks.
#[derive(Debug, Clone)]
pub struct DocReadContext {
    /// Current document path.
    pub path: String,
    /// Current paragraph or table index (0-based).
    pub index: usize,
}

/// Receives parsed content during streaming document reads.
///
/// Analogous to `ReadListener<T>` in `easyexcel-rust`.
pub trait DocReadListener<T> {
    /// Called for each parsed data item (paragraph text or row).
    ///
    /// # Errors
    ///
    /// Return an error to stop reading; use `ErrorAction` via `on_error`
    /// for recoverable errors.
    fn invoke(&mut self, data: T, context: &DocReadContext) -> Result<()>;

    /// Called when a complete table is encountered.
    fn invoke_table(&mut self, table: &TableData, context: &DocReadContext) -> Result<()> {
        let _ = (table, context);
        Ok(())
    }

    /// Called after all content has been parsed.
    fn on_complete(&mut self, _context: &DocReadContext) {}

    /// Called when a non-fatal error occurs during reading.
    ///
    /// Return [`ErrorAction::Stop`] to propagate the error, or
    /// [`ErrorAction::Skip`] / [`ErrorAction::Continue`] to proceed.
    fn on_error(
        &mut self,
        _error: &crate::error::DocError,
        _context: &DocReadContext,
    ) -> ErrorAction {
        ErrorAction::Stop
    }

    /// Called before processing each item to check whether reading should continue.
    /// Return `false` to stop early.
    fn has_next(&self, _context: &DocReadContext) -> bool {
        true
    }
}

// ============================================================================
// DocWriteHandler — write lifecycle hooks
// ============================================================================

/// Context for document-level write events.
#[derive(Debug, Clone)]
pub struct DocWriteContext {
    /// Output path.
    pub path: String,
}

/// Context for paragraph-level write events.
#[derive(Debug, Clone)]
pub struct ParagraphContext {
    /// Paragraph index (0-based).
    pub index: usize,
}

/// Context for table-level write events.
#[derive(Debug, Clone)]
pub struct TableWriteContext {
    /// Table index (0-based).
    pub index: usize,
    /// Number of rows in the table.
    pub row_count: usize,
}

/// Context for cell-level write events.
#[derive(Debug, Clone)]
pub struct CellContext {
    /// Row index (0-based).
    pub row: usize,
    /// Column index (0-based).
    pub column: usize,
    /// Cell value.
    pub value: DocValue,
}

/// Write lifecycle interceptor — hooks at document, paragraph, table, and cell level.
///
/// Analogous to `WriteHandler` in `easyexcel-rust`. All methods have no-op defaults;
/// override only the hooks you need.
pub trait DocWriteHandler {
    /// Execution order (lower values execute first).
    #[must_use]
    fn order() -> i32 {
        0
    }

    /// Called before the document is created.
    fn before_document(&mut self, _ctx: &DocWriteContext) -> Result<()> {
        Ok(())
    }

    /// Called after the document is finalised.
    fn after_document(&mut self, _ctx: &DocWriteContext) -> Result<()> {
        Ok(())
    }

    /// Called before a paragraph is written.
    fn before_paragraph(&mut self, _ctx: &ParagraphContext) -> Result<()> {
        Ok(())
    }

    /// Called after a paragraph is written.
    fn after_paragraph(&mut self, _ctx: &ParagraphContext) -> Result<()> {
        Ok(())
    }

    /// Called before a table is written.
    fn before_table(&mut self, _ctx: &TableWriteContext) -> Result<()> {
        Ok(())
    }

    /// Called after a table is written.
    fn after_table(&mut self, _ctx: &TableWriteContext) -> Result<()> {
        Ok(())
    }

    /// Called before a cell is written.
    fn before_cell(&mut self, _ctx: &CellContext) -> Result<()> {
        Ok(())
    }

    /// Called after a cell is written.
    fn after_cell(&mut self, _ctx: &CellContext) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// DocumentReader — 统一读取入口 trait
// ============================================================================

/// 统一的文档读取接口。
///
/// 后端实现（如 `office_oxide`）实现此 trait 即可接入 easydoc 读取体系。
pub trait DocumentReader {
    /// 读取文件并返回语义文档模型。
    ///
    /// # 错误
    /// 文件无法打开或解析时返回错误。
    fn read_model(&self, path: &std::path::Path) -> crate::Result<crate::DocumentContent>;

    /// 读取文件并以事件流方式推送内容。
    ///
    /// # 错误
    /// 文件无法打开或解析时返回错误。
    fn read_events(&self, path: &std::path::Path, sink: &mut dyn EventSink) -> crate::Result<()>;
}

// ============================================================================
// DocumentEvent — 文档事件枚举
// ============================================================================

/// 文档解析过程中产生的事件。
///
/// 用于流式读取场景，替代一次性返回完整文档模型。
#[derive(Clone, Debug, PartialEq)]
pub enum DocumentEvent {
    /// 遇到标题。
    Heading {
        /// 标题级别。
        level: u8,
        /// 富文本片段。
        runs: Vec<crate::DocumentTextRun>,
    },
    /// 遇到段落。
    Paragraph(Vec<crate::DocumentTextRun>),
    /// 遇到表格。
    Table(crate::DocumentTable),
    /// 遇到列表。
    List(crate::DocumentList),
    /// 遇到图片。
    Image(crate::DocumentImage),
    /// 遇到分页。
    PageBreak,
    /// 遇到分栏。
    ColumnBreak,
    /// 遇到代码块。
    CodeBlock {
        /// 可选语言标记。
        language: Option<String>,
        /// 代码文本。
        code: String,
    },
    /// 遇到分区。
    Section {
        /// 分区类型。
        section_type: Option<String>,
    },
    /// 文档开始。
    DocumentStart,
    /// 文档结束。
    DocumentEnd,
}

// ============================================================================
// EventSink — 事件消费接口
// ============================================================================

/// 事件消费回调接口。
///
/// 实现此 trait 以处理流式读取过程中产生的文档事件。
pub trait EventSink {
    /// 处理一个文档事件。
    ///
    /// # 错误
    /// 返回错误将中止读取。
    fn on_event(&mut self, event: &DocumentEvent) -> crate::Result<()>;

    /// 读取完成时调用。
    fn on_complete(&mut self) {}
}

/// 将事件流收集为 `DocumentContent` 的默认实现。
pub struct ContentCollector {
    blocks: Vec<crate::DocumentBlock>,
}

impl ContentCollector {
    /// 创建新的收集器。
    #[must_use]
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    /// 将收集的事件转换为语义文档。
    #[must_use]
    pub fn into_content(self) -> crate::DocumentContent {
        crate::DocumentContent {
            metadata: crate::DocumentMeta::default(),
            blocks: self.blocks,
        }
    }
}

impl Default for ContentCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSink for ContentCollector {
    fn on_event(&mut self, event: &DocumentEvent) -> crate::Result<()> {
        match event {
            DocumentEvent::Heading { level, runs } => {
                self.blocks.push(crate::DocumentBlock::Heading {
                    level: *level,
                    runs: runs.clone(),
                });
            }
            DocumentEvent::Paragraph(runs) => {
                self.blocks
                    .push(crate::DocumentBlock::Paragraph(runs.clone()));
            }
            DocumentEvent::Table(table) => {
                self.blocks.push(crate::DocumentBlock::Table(table.clone()));
            }
            DocumentEvent::List(list) => {
                self.blocks.push(crate::DocumentBlock::List(list.clone()));
            }
            DocumentEvent::Image(image) => {
                self.blocks.push(crate::DocumentBlock::Image(image.clone()));
            }
            DocumentEvent::PageBreak => {
                self.blocks.push(crate::DocumentBlock::PageBreak);
            }
            DocumentEvent::ColumnBreak => {
                self.blocks.push(crate::DocumentBlock::ColumnBreak);
            }
            DocumentEvent::CodeBlock { language, code } => {
                self.blocks.push(crate::DocumentBlock::CodeBlock {
                    language: language.clone(),
                    code: code.clone(),
                });
            }
            DocumentEvent::Section { section_type } => {
                self.blocks.push(crate::DocumentBlock::Section {
                    blocks: Vec::new(),
                    section_type: section_type.clone(),
                });
            }
            DocumentEvent::DocumentStart | DocumentEvent::DocumentEnd => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod event_tests {
    use super::*;

    #[test]
    fn document_event_debug() {
        let event = DocumentEvent::DocumentStart;
        assert_eq!(format!("{:?}", event), "DocumentStart");
    }

    #[test]
    fn document_event_heading() {
        let event = DocumentEvent::Heading {
            level: 1,
            runs: vec![crate::DocumentTextRun {
                text: "Title".into(),
                ..crate::DocumentTextRun::default()
            }],
        };
        match &event {
            DocumentEvent::Heading { level, runs } => {
                assert_eq!(*level, 1);
                assert_eq!(runs[0].text, "Title");
            }
            _ => panic!("expected Heading"),
        }
    }

    #[test]
    fn content_collector_roundtrip() {
        let mut collector = ContentCollector::new();
        collector.on_event(&DocumentEvent::DocumentStart).unwrap();
        collector
            .on_event(&DocumentEvent::Paragraph(vec![crate::DocumentTextRun {
                text: "Hello".into(),
                ..crate::DocumentTextRun::default()
            }]))
            .unwrap();
        collector.on_event(&DocumentEvent::PageBreak).unwrap();
        collector.on_event(&DocumentEvent::DocumentEnd).unwrap();

        let content = collector.into_content();
        assert_eq!(content.blocks.len(), 2);
        assert!(matches!(
            content.blocks[0],
            crate::DocumentBlock::Paragraph(_)
        ));
        assert!(matches!(content.blocks[1], crate::DocumentBlock::PageBreak));
    }

    #[test]
    fn content_collector_table_and_list() {
        let mut collector = ContentCollector::new();
        collector
            .on_event(&DocumentEvent::Table(crate::DocumentTable { rows: vec![] }))
            .unwrap();
        collector
            .on_event(&DocumentEvent::List(crate::DocumentList {
                ordered: false,
                start_number: None,
                items: vec![],
            }))
            .unwrap();
        let content = collector.into_content();
        assert_eq!(content.blocks.len(), 2);
    }

    #[test]
    fn content_collector_codeblock() {
        let mut collector = ContentCollector::new();
        collector
            .on_event(&DocumentEvent::CodeBlock {
                language: Some("rust".into()),
                code: "fn main() {}".into(),
            })
            .unwrap();
        let content = collector.into_content();
        match &content.blocks[0] {
            crate::DocumentBlock::CodeBlock { language, code } => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert_eq!(code, "fn main() {}");
            }
            _ => panic!("expected CodeBlock"),
        }
    }

    #[test]
    fn content_collector_section() {
        let mut collector = ContentCollector::new();
        collector
            .on_event(&DocumentEvent::Section {
                section_type: Some("continuous".into()),
            })
            .unwrap();
        let content = collector.into_content();
        match &content.blocks[0] {
            crate::DocumentBlock::Section {
                blocks,
                section_type,
            } => {
                assert!(blocks.is_empty());
                assert_eq!(section_type.as_deref(), Some("continuous"));
            }
            _ => panic!("expected Section"),
        }
    }
}

#[cfg(test)]
mod trait_coverage_tests {
    use super::*;

    struct NoopHandler;
    impl DocWriteHandler for NoopHandler {}

    #[test]
    fn noop_handler_all_defaults() {
        let mut h = NoopHandler;
        assert_eq!(NoopHandler::order(), 0);
        let ctx = DocWriteContext {
            path: "test".into(),
        };
        h.before_document(&ctx).unwrap();
        h.after_document(&ctx).unwrap();
        let pctx = ParagraphContext { index: 0 };
        h.before_paragraph(&pctx).unwrap();
        h.after_paragraph(&pctx).unwrap();
        let tctx = TableWriteContext {
            index: 0,
            row_count: 1,
        };
        h.before_table(&tctx).unwrap();
        h.after_table(&tctx).unwrap();
        let cctx = CellContext {
            row: 0,
            column: 0,
            value: DocValue::Empty,
        };
        h.before_cell(&cctx).unwrap();
        h.after_cell(&cctx).unwrap();
    }

    #[test]
    fn read_listener_defaults() {
        struct TestListener;
        impl DocReadListener<String> for TestListener {
            fn invoke(&mut self, _: String, _: &DocReadContext) -> crate::Result<()> {
                Ok(())
            }
        }
        let mut listener = TestListener;
        let ctx = DocReadContext {
            path: "test".into(),
            index: 0,
        };
        assert!(listener.has_next(&ctx));
        assert!(matches!(
            listener.on_error(&crate::DocError::Document("x".into()), &ctx),
            ErrorAction::Stop
        ));
        listener.on_complete(&ctx);
    }

    #[test]
    fn read_listener_invoke_table_default() {
        struct TestListener;
        impl DocReadListener<String> for TestListener {
            fn invoke(&mut self, _: String, _: &DocReadContext) -> crate::Result<()> {
                Ok(())
            }
        }
        let mut listener = TestListener;
        let ctx = DocReadContext {
            path: "test".into(),
            index: 0,
        };
        let table = TableData {
            headers: None,
            rows: vec![],
        };
        listener.invoke_table(&table, &ctx).unwrap();
    }

    #[test]
    fn content_collector_all_event_types() {
        let mut c = ContentCollector::new();
        c.on_event(&DocumentEvent::DocumentStart).unwrap();
        c.on_event(&DocumentEvent::Heading {
            level: 1,
            runs: vec![],
        })
        .unwrap();
        c.on_event(&DocumentEvent::Paragraph(vec![])).unwrap();
        c.on_event(&DocumentEvent::Table(crate::DocumentTable { rows: vec![] }))
            .unwrap();
        c.on_event(&DocumentEvent::List(crate::DocumentList {
            ordered: false,
            start_number: None,
            items: vec![],
        }))
        .unwrap();
        c.on_event(&DocumentEvent::Image(crate::DocumentImage {
            alt_text: None,
            data: None,
            extension: None,
        }))
        .unwrap();
        c.on_event(&DocumentEvent::PageBreak).unwrap();
        c.on_event(&DocumentEvent::ColumnBreak).unwrap();
        c.on_event(&DocumentEvent::CodeBlock {
            language: None,
            code: "".into(),
        })
        .unwrap();
        c.on_event(&DocumentEvent::Section { section_type: None })
            .unwrap();
        c.on_event(&DocumentEvent::DocumentEnd).unwrap();
        c.on_complete();
        let content = c.into_content();
        assert_eq!(content.blocks.len(), 9); // DocumentStart/DocumentEnd produce no blocks
    }

    #[test]
    fn content_collector_default() {
        let c = ContentCollector::default();
        let content = c.into_content();
        assert!(content.blocks.is_empty());
    }

    #[test]
    fn doc_read_context_clone_debug() {
        let ctx = DocReadContext {
            path: "test".into(),
            index: 5,
        };
        let ctx2 = ctx.clone();
        assert_eq!(ctx2.index, 5);
        assert!(format!("{:?}", ctx).contains("test"));
    }

    #[test]
    fn doc_write_context_clone_debug() {
        let ctx = DocWriteContext {
            path: "out.docx".into(),
        };
        let ctx2 = ctx.clone();
        assert_eq!(ctx2.path, "out.docx");
        assert!(format!("{:?}", ctx).contains("out.docx"));
    }

    #[test]
    fn paragraph_context_clone_debug() {
        let ctx = ParagraphContext { index: 3 };
        let ctx2 = ctx.clone();
        assert_eq!(ctx2.index, 3);
        assert!(format!("{:?}", ctx).contains("3"));
    }

    #[test]
    fn table_write_context_clone_debug() {
        let ctx = TableWriteContext {
            index: 1,
            row_count: 10,
        };
        let ctx2 = ctx.clone();
        assert_eq!(ctx2.index, 1);
        assert_eq!(ctx2.row_count, 10);
        assert!(format!("{:?}", ctx).contains("10"));
    }

    #[test]
    fn cell_context_clone_debug() {
        let ctx = CellContext {
            row: 2,
            column: 3,
            value: DocValue::Int(42),
        };
        let ctx2 = ctx.clone();
        assert_eq!(ctx2.row, 2);
        assert!(format!("{:?}", ctx).contains("42"));
    }

    #[test]
    fn document_event_clone_debug() {
        let events = vec![
            DocumentEvent::DocumentStart,
            DocumentEvent::DocumentEnd,
            DocumentEvent::PageBreak,
            DocumentEvent::ColumnBreak,
            DocumentEvent::Heading {
                level: 1,
                runs: vec![],
            },
            DocumentEvent::Paragraph(vec![]),
            DocumentEvent::Section { section_type: None },
            DocumentEvent::CodeBlock {
                language: None,
                code: "".into(),
            },
        ];
        for event in &events {
            let _clone = event.clone();
            let _debug = format!("{:?}", event);
        }
    }
}
