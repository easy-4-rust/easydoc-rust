//! 核心扩展 trait 体系。
//!
//! 这些 trait 构成 easydoc-rust 扩展性的骨干，对标 easyexcel-rust 的
//! `ExcelRow`、`Converter<T>`、`ReadListener<T>`、`WriteHandler`。
//! 新增 trait 必须放在本文件，按"模型-转换-读-写-事件"分组。
//!
//! 对应 Java: `com.alibaba.excel` (`EasyExcel` 4.0.3) 的核心扩展点

use crate::converter::ConverterRegistry;
use crate::error::Result;
use crate::metadata::TableColumn;
use crate::types::{CellData, DocValue, ErrorAction, RowData, TableData};

// ============================================================================
// DocxRow — struct ↔ table row mapping
// ============================================================================

/// 结构体与 DOCX 表格行的双向映射。
///
/// 类似 easyexcel-rust 的 `ExcelRow`。通过 `#[derive(DocxRow)]` 自动生成实现。
///
/// 对应 Java: `com.alibaba.excel.annotation.ExcelProperty` + 反射机制
///
/// # 示例
///
/// ```ignore
/// #[derive(DocxRow)]
/// struct User {
///     #[docx(name = "姓名", order = 0)]
///     name: String,
///     #[docx(name = "年龄", order = 1)]
///     age: u32,
/// }
/// ```
pub trait DocxRow {
    /// 返回列 schema：表头名称、索引和格式提示。
    ///
    /// 对应 Java: `ExcelProperty` 注解的 `value()` / `order()` / `format()` 等属性
    fn schema() -> &'static [TableColumn]
    where
        Self: Sized;

    /// 从原始单元格值反序列化为结构体（使用默认转换器）。
    ///
    /// 对应 Java: `EasyExcel` 内部通过反射将 `ReadCellData` 映射到字段
    fn from_row(row: &RowData) -> Result<Self>
    where
        Self: Sized;

    /// 使用自定义转换器注册表从原始单元格值反序列化为结构体。
    ///
    /// 对应 Java: `ConverterRegistry` + `ExcelProperty(converter = ...)`
    fn from_row_with_converters(row: &RowData, registry: &ConverterRegistry) -> Result<Self>
    where
        Self: Sized;

    /// 将自身序列化为单元格值列表（使用默认转换器）。
    ///
    /// 对应 Java: `EasyExcel` 内部通过反射将字段值转为 `WriteCellData`
    fn to_row(&self) -> Result<Vec<CellData>>;

    /// 使用自定义转换器注册表将自身序列化为单元格值列表。
    ///
    /// 对应 Java: `ConverterRegistry` + `ExcelProperty(converter = ...)`
    fn to_row_with_converters(&self, registry: &ConverterRegistry) -> Result<Vec<CellData>>;
}

// ============================================================================
// DocConverter — bidirectional type conversion
// ============================================================================

/// Rust 类型 `T` 与 [`DocValue`] 之间的双向转换。
///
/// 对应 Java: `com.alibaba.excel.converters.Converter<T>`
///
/// 通过 [`ConverterRegistry`] 或 builder 的 `register_converter` 方法注册自定义转换器。
pub trait DocConverter<T> {
    /// 返回此转换器处理的 `TypeId`。
    ///
    /// 对应 Java: `Converter#supportJavaTypeKey`
    fn support_type() -> std::any::TypeId
    where
        Self: Sized;

    /// 将 Rust 值转换为文档值（用于写入）。
    ///
    /// 对应 Java: `Converter#convertToExcelData`
    ///
    /// # Errors
    ///
    /// 值无法转换时返回 [`crate::DocError::Conversion`]。
    fn to_doc_value(&self, value: &T, column: &TableColumn) -> Result<DocValue>;

    /// 将文档值转换回 Rust 值（用于读取）。
    ///
    /// 对应 Java: `Converter#convertToJavaData`
    ///
    /// # Errors
    ///
    /// 值无法转换时返回 [`crate::DocError::Conversion`]。
    fn from_doc_value(&self, value: &DocValue, column: &TableColumn) -> Result<T>;
}

// ============================================================================
// DocReadListener — streaming read callbacks
// ============================================================================

/// 读取监听器回调的上下文信息。
#[derive(Debug, Clone)]
pub struct DocReadContext {
    /// 当前文档路径。
    pub path: String,
    /// 当前段落或表格索引（从零开始）。
    pub index: usize,
}

/// 流式读取过程中接收已解析内容的监听器。
///
/// 对应 Java: `com.alibaba.excel.read.listener.ReadListener<T>`
pub trait DocReadListener<T> {
    /// 每解析一个数据项（段落文本或表格行）时调用。
    ///
    /// 对应 Java: `ReadListener#invoke`
    ///
    /// # Errors
    ///
    /// 返回错误将停止读取；可恢复错误请通过 `on_error` 返回 `ErrorAction`。
    fn invoke(&mut self, data: T, context: &DocReadContext) -> Result<()>;

    /// 遇到完整表格时调用。
    ///
    /// 对应 Java: `ReadListener#invokeHead`（表头行场景）
    fn invoke_table(&mut self, table: &TableData, context: &DocReadContext) -> Result<()> {
        let _ = (table, context);
        Ok(())
    }

    /// 所有内容解析完成后调用。
    ///
    /// 对应 Java: `ReadListener#doAfterAllAnalysed`
    fn on_complete(&mut self, _context: &DocReadContext) {}

    /// 读取过程中发生非致命错误时调用。
    ///
    /// 对应 Java: `ReadListener#onException`
    ///
    /// 返回 [`ErrorAction::Stop`] 以传播错误，或
    /// [`ErrorAction::Skip`] / [`ErrorAction::Continue`] 以继续。
    fn on_error(
        &mut self,
        _error: &crate::error::DocError,
        _context: &DocReadContext,
    ) -> ErrorAction {
        ErrorAction::Stop
    }

    /// 处理每个数据项前检查是否应继续读取。返回 `false` 可提前终止。
    ///
    /// 对应 Java: `ReadListener#hasNext`
    fn has_next(&self, _context: &DocReadContext) -> bool {
        true
    }
}

// ============================================================================
// DocWriteHandler — write lifecycle hooks
// ============================================================================

/// 文档级写入事件的上下文。
#[derive(Debug, Clone)]
pub struct DocWriteContext {
    /// 输出路径。
    pub path: String,
}

/// 段落级写入事件的上下文。
#[derive(Debug, Clone)]
pub struct ParagraphContext {
    /// 段落索引（从零开始）。
    pub index: usize,
}

/// 表格级写入事件的上下文。
#[derive(Debug, Clone)]
pub struct TableWriteContext {
    /// 表格索引（从零开始）。
    pub index: usize,
    /// 表格行数。
    pub row_count: usize,
}

/// 单元格级写入事件的上下文。
#[derive(Debug, Clone)]
pub struct CellContext {
    /// 行索引（从零开始）。
    pub row: usize,
    /// 列索引（从零开始）。
    pub column: usize,
    /// 单元格值。
    pub value: DocValue,
}

/// 写入生命周期拦截器 -- 在文档、段落、表格和单元格级别提供钩子。
///
/// 对应 Java: `com.alibaba.excel.write.handler.WriteHandler`
///
/// 所有方法均有空默认实现；只需覆盖需要的钩子。
pub trait DocWriteHandler {
    /// 执行顺序（值越小越先执行）。
    ///
    /// 对应 Java: `WriteHandler` 的 `order` 属性
    #[must_use]
    fn order() -> i32 {
        0
    }

    /// 文档创建前调用。
    ///
    /// 对应 Java: `WorkbookWriteHandler#beforeWorkbookCreate`
    fn before_document(&mut self, _ctx: &DocWriteContext) -> Result<()> {
        Ok(())
    }

    /// 文档完成后调用。
    ///
    /// 对应 Java: `WorkbookWriteHandler#afterWorkbookWrite`
    fn after_document(&mut self, _ctx: &DocWriteContext) -> Result<()> {
        Ok(())
    }

    /// 段落写入前调用。
    fn before_paragraph(&mut self, _ctx: &ParagraphContext) -> Result<()> {
        Ok(())
    }

    /// 段落写入后调用。
    fn after_paragraph(&mut self, _ctx: &ParagraphContext) -> Result<()> {
        Ok(())
    }

    /// 表格写入前调用。
    ///
    /// 对应 Java: `SheetWriteHandler#beforeSheetCreate`
    fn before_table(&mut self, _ctx: &TableWriteContext) -> Result<()> {
        Ok(())
    }

    /// 表格写入后调用。
    ///
    /// 对应 Java: `SheetWriteHandler#afterSheetWrite`
    fn after_table(&mut self, _ctx: &TableWriteContext) -> Result<()> {
        Ok(())
    }

    /// 单元格写入前调用。
    ///
    /// 对应 Java: `CellWriteHandler#beforeCellCreate`
    fn before_cell(&mut self, _ctx: &CellContext) -> Result<()> {
        Ok(())
    }

    /// 单元格写入后调用。
    ///
    /// 对应 Java: `CellWriteHandler#afterCellWrite`
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
/// 无直接 Java 对应（Java `EasyExcel` 不提供统一读取抽象），是 easydoc-rust 自创。
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
/// 类比 Java: `ReadListener<T>` 的回调方法（`invoke` / `doAfterAllAnalysed`）。
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
///
/// 类比 Java: `ReadListener` 的默认收集行为。
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
        assert_eq!(format!("{event:?}"), "DocumentStart");
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
            code: String::new(),
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
        assert!(format!("{ctx:?}").contains("test"));
    }

    #[test]
    fn doc_write_context_clone_debug() {
        let ctx = DocWriteContext {
            path: "out.docx".into(),
        };
        let ctx2 = ctx.clone();
        assert_eq!(ctx2.path, "out.docx");
        assert!(format!("{ctx:?}").contains("out.docx"));
    }

    #[test]
    fn paragraph_context_clone_debug() {
        let ctx = ParagraphContext { index: 3 };
        let ctx2 = ctx.clone();
        assert_eq!(ctx2.index, 3);
        assert!(format!("{ctx:?}").contains('3'));
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
        assert!(format!("{ctx:?}").contains("10"));
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
        assert!(format!("{ctx:?}").contains("42"));
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
                code: String::new(),
            },
        ];
        for event in &events {
            let _clone = event.clone();
            let _debug = format!("{event:?}");
        }
    }
}
