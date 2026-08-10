//! 基于 SAX 的流式 DOCX 读取器。
//!
//! 使用 `quick-xml` 解析 `.docx` ZIP 归档内的 `word/document.xml`，
//! 内存开销为 O(1)（与文档大小无关）。每个块级元素转换为 [`DocumentEvent`]
//! 并推送给 [`EventSink`]。
//!
//! 类比 easyexcel-rust 的 `XlsxSaxAnalyser`。

use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use super::image::{Relationships, extension_from_filename, read_zip_part};
use crate::security::{SecurityPolicy, SsrfGuard};
use easydoc_core::{
    DocError, DocumentBlock, DocumentEvent, DocumentImage, DocumentList, DocumentListItem,
    DocumentTableCell, DocumentTableRow, DocumentTextRun, EventSink, Result,
};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;

// ---------------------------------------------------------------------------
// OOXML element / attribute name constants (with w: namespace prefix)
// ---------------------------------------------------------------------------

const W_P: &[u8] = b"w:p";
const W_R: &[u8] = b"w:r";
const W_T: &[u8] = b"w:t";
const W_PPR: &[u8] = b"w:pPr";
const W_PSTYLE: &[u8] = b"w:pStyle";
const W_RPR: &[u8] = b"w:rPr";
const W_B: &[u8] = b"w:b";
const W_I: &[u8] = b"w:i";
const W_STRIKE: &[u8] = b"w:strike";
const W_TBL: &[u8] = b"w:tbl";
const W_TR: &[u8] = b"w:tr";
const W_TC: &[u8] = b"w:tc";
const W_BR: &[u8] = b"w:br";
const W_DRAWING: &[u8] = b"w:drawing";
const W_TCPR: &[u8] = b"w:tcPr";
const W_GRIDSPAN: &[u8] = b"w:gridSpan";
const W_VMERGE: &[u8] = b"w:vMerge";

const A_BLIP: &[u8] = b"a:blip";
const WP_DOC_PR: &[u8] = b"wp:docPr";
const R_EMBED: &[u8] = b"r:embed";

const W_VAL: &[u8] = b"w:val";
const W_TYPE: &[u8] = b"w:type";

// List numbering constants
const W_NUMPR: &[u8] = b"w:numPr";
const W_NUMID: &[u8] = b"w:numId";
const W_ILVL: &[u8] = b"w:ilvl";

// Hyperlink constants
const W_HYPERLINK: &[u8] = b"w:hyperlink";
const R_ID: &[u8] = b"r:id";

// OMML math namespace constants (m: prefix)
const M_OMATH: &[u8] = b"m:oMath";
const M_OMATHPARA: &[u8] = b"m:oMathPara";

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

/// Internal parser state, maintained as a stack.
#[derive(Debug)]
enum ParseState {
    /// Top-level `<w:document>`.
    Document,
    /// Inside `<w:p>` -- accumulating runs.
    Paragraph {
        /// Runs collected so far.
        runs: Vec<DocumentTextRun>,
        /// Heading level parsed from `<w:pStyle>`, if any.
        heading_level: Option<u8>,
        /// Whether we are currently inside `<w:pPr>`.
        in_ppr: bool,
        /// Whether we are currently inside a `<w:r>`.
        in_run: bool,
        /// Whether we are currently inside `<w:rPr>` of the current run.
        in_rpr: bool,
        /// Bold state for the current run (propagated from `<w:rPr>`).
        run_bold: bool,
        /// Italic state for the current run.
        run_italic: bool,
        /// Strikethrough state for the current run.
        run_strike: bool,
        /// Text buffer for the current `<w:t>` element.
        text_buf: String,
        /// Whether we are inside `<w:t>`.
        in_text: bool,
        /// Whether `xml:space="preserve"` is set on the current `<w:t>`.
        preserve_space: bool,
        /// Whether this paragraph contains `<w:numPr>` (is a list item).
        has_num_pr: bool,
        /// `numId` from `<w:numId w:val="..."/>` inside `<w:numPr>`.
        num_id: Option<u32>,
        /// `ilvl` from `<w:ilvl w:val="..."/>` inside `<w:numPr>`.
        ilvl: Option<u8>,
        /// Whether we are currently inside `<w:hyperlink>`.
        in_hyperlink: bool,
        /// The `r:id` of the current `<w:hyperlink>`, if any.
        hyperlink_rid: Option<String>,
    },
    /// Inside `<w:tbl>` -- accumulating rows.
    Table {
        /// Rows collected so far.
        rows: Vec<DocumentTableRow>,
        /// Current row being built, if inside `<w:tr>`.
        current_row: Option<TableRowBuilder>,
    },
    /// Inside `<w:drawing>` -- accumulating image metadata.
    Drawing {
        /// Relationship ID from `<a:blip r:embed="..."/>`.
        pending_rid: Option<String>,
        /// Alt text from `<wp:docPr descr="..." name="..."/>`.
        pending_alt: Option<String>,
    },
}

/// Builder for a table row while parsing `<w:tr>`.
#[derive(Debug)]
struct TableRowBuilder {
    cells: Vec<TableCellBuilder>,
}

/// Vertical merge kind for OOXML `<w:vMerge>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VMerge {
    /// `<w:vMerge w:val="restart"/>` -- first cell in a vertical merge.
    Restart,
    /// `<w:vMerge/>` or `<w:vMerge w:val="continue"/>` -- continuation cell.
    Continue,
}

/// Builder for a table cell while parsing `<w:tc>`.
#[derive(Debug)]
struct TableCellBuilder {
    text: String,
    /// Column span from `<w:gridSpan w:val="N"/>` (default 1).
    column_span: u32,
    /// Row span; set to 0 for vMerge continue cells (merged into cell above).
    row_span: u32,
    /// Parsed vMerge state, if any.
    v_merge: Option<VMerge>,
    /// Whether we are currently inside `<w:tcPr>`.
    in_tcpr: bool,
    /// Blocks accumulated from nested tables or other block-level content.
    blocks: Vec<DocumentBlock>,
}

// ---------------------------------------------------------------------------
// Internal ParseSink abstraction
// ---------------------------------------------------------------------------

/// Internal trait unifying event-based and direct block output during parsing.
///
/// This allows the SAX loop to emit [`DocumentBlock::Math`] inline alongside
/// normal [`DocumentEvent`]s, preserving document ordering.
trait ParseSink {
    /// Push a document event (converted to block(s) by the implementation).
    fn on_event(&mut self, event: &DocumentEvent) -> Result<()>;
    /// Push a block directly (used for `DocumentBlock::Math`).
    fn push_block(&mut self, block: DocumentBlock);
    /// Called when parsing is complete.
    fn on_complete(&mut self) {}
}

/// Wraps an [`EventSink`] as a [`ParseSink`].
///
/// Math blocks are silently dropped because [`DocumentEvent`] has no `Math`
/// variant. Callers requiring math should use [`DocxSaxReader::read_blocks`].
struct EventSinkAdapter<'a>(&'a mut dyn EventSink);

impl ParseSink for EventSinkAdapter<'_> {
    fn on_event(&mut self, event: &DocumentEvent) -> Result<()> {
        self.0.on_event(event)
    }

    fn push_block(&mut self, _block: DocumentBlock) {
        // DocumentEvent has no Math variant; blocks are dropped here.
        // Use read_blocks() for full math support.
    }

    fn on_complete(&mut self) {
        self.0.on_complete();
    }
}

/// Collects parsed content as `Vec<DocumentBlock>`, preserving ordering.
///
/// Both event-derived and directly-pushed blocks go into a single sequence.
struct BlockCollector(Vec<DocumentBlock>);

impl ParseSink for BlockCollector {
    fn on_event(&mut self, event: &DocumentEvent) -> Result<()> {
        match event {
            DocumentEvent::Heading { level, runs } => {
                self.0.push(DocumentBlock::Heading {
                    level: *level,
                    runs: runs.clone(),
                });
            }
            DocumentEvent::Paragraph(runs) => {
                self.0.push(DocumentBlock::Paragraph(runs.clone()));
            }
            DocumentEvent::Table(table) => {
                self.0.push(DocumentBlock::Table(table.clone()));
            }
            DocumentEvent::List(list) => {
                self.0.push(DocumentBlock::List(list.clone()));
            }
            DocumentEvent::Image(image) => {
                self.0.push(DocumentBlock::Image(image.clone()));
            }
            DocumentEvent::PageBreak => {
                self.0.push(DocumentBlock::PageBreak);
            }
            DocumentEvent::ColumnBreak => {
                self.0.push(DocumentBlock::ColumnBreak);
            }
            DocumentEvent::CodeBlock { language, code } => {
                self.0.push(DocumentBlock::CodeBlock {
                    language: language.clone(),
                    code: code.clone(),
                });
            }
            DocumentEvent::Section { section_type } => {
                self.0.push(DocumentBlock::Section {
                    blocks: Vec::new(),
                    section_type: section_type.clone(),
                });
            }
            DocumentEvent::DocumentStart | DocumentEvent::DocumentEnd => {}
        }
        Ok(())
    }

    fn push_block(&mut self, block: DocumentBlock) {
        self.0.push(block);
    }
}

/// Emits the accumulated runs of the current paragraph (if non-empty) without
/// popping the `Paragraph` state from the stack. Used to flush partial content
/// before entering a math region.
fn flush_paragraph_runs(sink: &mut dyn ParseSink, stack: &mut [ParseState]) -> Result<()> {
    if let Some(ParseState::Paragraph { runs, .. }) = stack.last_mut()
        && !runs.is_empty()
    {
        let taken = std::mem::take(runs);
        sink.on_event(&DocumentEvent::Paragraph(taken))?;
    }
    Ok(())
}

/// Flushes accumulated list items as a single [`DocumentBlock::List`].
///
/// Called when a non-list paragraph, heading, table, or document end is
/// encountered after one or more consecutive list-item paragraphs.
///
/// Uses `first_num_id` and `first_ilvl` (from the first list item) to look up
/// the numbering definition and determine whether the list is ordered and what
/// its start number is. Falls back to `ordered=false` if the numbering is
/// unavailable.
///
/// Builds nested list structure from `(item, ilvl)` pairs using
/// [`build_nested_items`] before emitting.
///
/// Resolves hyperlink rIds to URLs using the provided [`Relationships`].
fn flush_list(
    sink: &mut dyn ParseSink,
    list_items: &mut Vec<(DocumentListItem, u8)>,
    first_num_id: &mut Option<u32>,
    first_ilvl: &mut u8,
    numbering: Option<&super::numbering::Numbering>,
    relationships: Option<&Relationships>,
    ssrf: &SsrfGuard,
) -> Result<()> {
    if !list_items.is_empty() {
        // Resolve hyperlinks in all list items (including nested).
        resolve_hyperlinks_in_flat_items(list_items, relationships, ssrf);

        // Resolve list type from numbering definitions.
        let (ordered, start_number) = if let (Some(num_id), Some(num)) = (*first_num_id, numbering)
        {
            match num.lookup(num_id, *first_ilvl) {
                Some(level) => {
                    let start = if level.ordered { level.start } else { None };
                    (level.ordered, start)
                }
                None => (false, None),
            }
        } else {
            (false, None)
        };

        let flat = std::mem::take(list_items);
        let items = build_nested_items(flat);
        sink.push_block(DocumentBlock::List(DocumentList {
            ordered,
            start_number,
            items,
        }));
        *first_num_id = None;
        *first_ilvl = 0;
    }
    Ok(())
}

/// Builds a nested list tree from a flat sequence of `(item, ilvl)` pairs.
///
/// Each list item's `ilvl` (indentation level) determines where it appears in
/// the tree. Items at `ilvl == 0` become top-level items. Items at `ilvl > 0`
/// are nested inside the most recent ancestor with `ilvl == new_ilvl - 1`.
///
/// If an item jumps levels (e.g. `ilvl` goes from 0 to 2, skipping 1),
/// intermediate empty nested lists are created to maintain the hierarchy.
fn build_nested_items(flat: Vec<(DocumentListItem, u8)>) -> Vec<DocumentListItem> {
    let mut items: Vec<DocumentListItem> = Vec::new();

    for (new_item, ilvl) in flat {
        if ilvl == 0 {
            items.push(new_item);
        } else {
            // Attach to the last top-level item's nested subtree.
            if let Some(parent) = items.last_mut() {
                attach_to_nested(parent, new_item, ilvl);
            } else {
                // No parent exists -- promote to top level (defensive).
                items.push(new_item);
            }
        }
    }

    items
}

/// Recursively attaches `new_item` at the given `ilvl` depth inside `parent`.
///
/// If `ilvl == 1`, the item is appended to `parent.nested.items`. If `ilvl > 1`,
/// the function recurses into the last item of `parent.nested.items` with
/// `ilvl - 1`. Missing intermediate nested lists are created automatically.
fn attach_to_nested(parent: &mut DocumentListItem, new_item: DocumentListItem, ilvl: u8) {
    if ilvl == 1 {
        // Direct child of parent.
        let nested = parent
            .nested
            .get_or_insert_with(|| Box::new(DocumentList::default()));
        nested.items.push(new_item);
    } else {
        // ilvl > 1: ensure parent has a nested list, then recurse into its last item.
        let nested = parent
            .nested
            .get_or_insert_with(|| Box::new(DocumentList::default()));
        if let Some(last) = nested.items.last_mut() {
            attach_to_nested(last, new_item, ilvl - 1);
        } else {
            // No items in the intermediate level -- push directly.
            // This handles ilvl-jump edge cases (e.g. 0 -> 2 with no level-1 items).
            nested.items.push(new_item);
        }
    }
}

/// Resolves hyperlink rIds to URLs in all runs of all list items.
fn resolve_hyperlinks_in_items(
    items: &mut [DocumentListItem],
    relationships: Option<&Relationships>,
    ssrf: &SsrfGuard,
) {
    let Some(rels) = relationships else { return };
    for item in items.iter_mut() {
        resolve_hyperlinks_in_blocks(&mut item.blocks, rels, ssrf);
        // Recurse into nested lists.
        if let Some(nested) = item.nested.as_mut() {
            resolve_hyperlinks_in_items(&mut nested.items, Some(rels), ssrf);
        }
    }
}

/// Resolves hyperlink rIds to URLs in all runs of flat `(item, ilvl)` list items.
fn resolve_hyperlinks_in_flat_items(
    items: &mut [(DocumentListItem, u8)],
    relationships: Option<&Relationships>,
    ssrf: &SsrfGuard,
) {
    let Some(rels) = relationships else { return };
    for (item, _ilvl) in items.iter_mut() {
        resolve_hyperlinks_in_blocks(&mut item.blocks, rels, ssrf);
    }
}

/// Recursively resolves hyperlink rIds in a list of blocks.
fn resolve_hyperlinks_in_blocks(
    blocks: &mut [DocumentBlock],
    rels: &Relationships,
    ssrf: &SsrfGuard,
) {
    for block in blocks {
        match block {
            DocumentBlock::Paragraph(runs) | DocumentBlock::Heading { runs, .. } => {
                resolve_hyperlinks_in_runs(runs, rels, ssrf);
            }
            DocumentBlock::List(list) => {
                resolve_hyperlinks_in_items(&mut list.items, Some(rels), ssrf);
            }
            DocumentBlock::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        resolve_hyperlinks_in_blocks(&mut cell.blocks, rels, ssrf);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Resolves hyperlink rIds in a list of text runs.
///
/// When an SSRF guard is provided, resolved URLs are validated and
/// blocked URLs are stripped (set to `None`) to prevent SSRF.
/// Raw rIds that could not be resolved from relationships are kept
/// as-is (they are not network-reachable URLs).
fn resolve_hyperlinks_in_runs(
    runs: &mut [DocumentTextRun],
    rels: &Relationships,
    ssrf: &SsrfGuard,
) {
    for run in runs.iter_mut() {
        if let Some(rid) = run.hyperlink.take() {
            match rels.resolve_hyperlink(&rid) {
                Some(resolved_url) => {
                    // Only SSRF-check URLs that were actually resolved
                    // from relationships (network-reachable targets).
                    if ssrf.check_url(resolved_url).is_ok() {
                        run.hyperlink = Some(resolved_url.to_owned());
                    }
                    // Blocked: drop the hyperlink entirely.
                }
                None => {
                    // Not resolved -- keep the raw rId (not a URL).
                    run.hyperlink = Some(rid);
                }
            }
        }
    }
}

/// Extracts the `r:id` attribute value from a start tag (used for hyperlinks).
fn extract_rid(tag: &quick_xml::events::BytesStart) -> Option<String> {
    for attr in tag.attributes().flatten() {
        if attr.key.as_ref() == R_ID {
            return attr
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(std::borrow::Cow::into_owned);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// DocxSaxReader
// ---------------------------------------------------------------------------

/// 基于 SAX 风格 XML 解析的流式 DOCX 读取器。
///
/// 打开 `.docx` 文件（ZIP 归档），提取 `word/document.xml`，并使用 `quick-xml`
/// 逐事件解析。每个块级元素转换为 [`DocumentEvent`] 并转发给提供的 [`EventSink`]。
///
/// 内存使用量相对于文档大小为 O(1) -- 仅当前正在解析的元素驻留在内存中。
/// 类比 easyexcel-rust 的 `XlsxSaxAnalyser`。
///
/// # 示例
///
/// ```no_run
/// use std::path::Path;
/// use easydoc_reader::extractor::sax::DocxSaxReader;
/// use easydoc_core::ContentCollector;
///
/// let mut reader = DocxSaxReader::from_path(Path::new("test.docx")).unwrap();
/// let mut collector = ContentCollector::new();
/// reader.read_events(&mut collector).unwrap();
/// let content = collector.into_content();
/// ```
pub struct DocxSaxReader<R: Read> {
    reader: XmlReader<BufReader<R>>,
    /// ZIP archive handle, only present when created via [`Self::from_path`].
    archive: Option<zip::ZipArchive<File>>,
    /// Parsed relationships from `word/_rels/document.xml.rels`.
    relationships: Option<Relationships>,
    /// Parsed numbering definitions from `word/numbering.xml`.
    numbering: Option<super::numbering::Numbering>,
    /// Security policy for SSRF and ZIP bomb protection.
    security: SecurityPolicy,
}

impl DocxSaxReader<std::io::Cursor<Vec<u8>>> {
    /// 从文件路径创建读取器（使用默认安全策略）。
    ///
    /// 打开 `.docx` ZIP 归档，按默认 [`SecurityPolicy`]（ZIP 炸弹 / 元素爆炸防护）
    /// 验证，定位 `word/document.xml` 部分，并读入内存进行流式 XML 解析。
    ///
    /// 类比 easyexcel-rust 的 `XlsxSaxAnalyser::new()`。
    ///
    /// # Errors
    ///
    /// 文件不存在、不是有效 ZIP、不包含 `word/document.xml` 或安全验证失败时返回错误。
    pub fn from_path(path: &Path) -> Result<Self> {
        Self::from_path_with_security(path, SecurityPolicy::new())
    }

    /// 从文件路径创建读取器（使用自定义安全策略）。
    ///
    /// 与 [`from_path`](Self::from_path) 类似，但使用提供的策略进行
    /// ZIP 归档验证和超链接 SSRF 检查。
    ///
    /// # Errors
    ///
    /// 文件不存在、不是有效 ZIP、不包含 `word/document.xml` 或安全验证失败时返回错误。
    pub fn from_path_with_security(path: &Path, security: SecurityPolicy) -> Result<Self> {
        let file = File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        // Validate ZIP archive against security limits (bomb / element explosion).
        security
            .limits
            .validate_archive(&mut archive)
            .map_err(|msg| DocError::Format(format!("security: {msg}")))?;

        // Resolve the entry name first to avoid overlapping borrows.
        let entry_name = if archive.index_for_name("word/document.xml").is_some() {
            "word/document.xml".to_owned()
        } else {
            find_word_document_xml(&mut archive)?
        };

        // Parse relationships for image extraction.
        let relationships = if archive
            .index_for_name("word/_rels/document.xml.rels")
            .is_some()
        {
            let rels_bytes = {
                let mut entry = archive.by_name("word/_rels/document.xml.rels")?;
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut buf)?;
                buf
            };
            let rels_xml = String::from_utf8(rels_bytes)
                .map_err(|e| DocError::Format(format!("rels XML not valid UTF-8: {e}")))?;
            Some(Relationships::parse(&rels_xml)?)
        } else {
            None
        };

        // Parse numbering definitions (for ordered/unordered list detection).
        let numbering = if archive.index_for_name("word/numbering.xml").is_some() {
            let num_bytes = {
                let mut entry = archive.by_name("word/numbering.xml")?;
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut buf)?;
                buf
            };
            match String::from_utf8(num_bytes) {
                Ok(xml) => super::numbering::Numbering::parse(&xml).ok(),
                Err(_) => None,
            }
        } else {
            None
        };

        // Extract the XML bytes from the ZIP entry.
        let xml_bytes = {
            let mut entry = archive.by_name(&entry_name)?;
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut buf)?;
            buf
        };

        let buf_reader = BufReader::new(std::io::Cursor::new(xml_bytes));
        let mut reader = XmlReader::from_reader(buf_reader);
        reader.config_mut().trim_text(false);

        Ok(Self {
            reader,
            archive: Some(archive),
            relationships,
            numbering,
            security,
        })
    }
}

impl<R: Read> DocxSaxReader<R> {
    /// 创建包装现有 `Read` 源（包含原始 XML）的读取器。
    ///
    /// 适用于测试场景或 XML 已被提取的情况。
    pub fn from_reader(source: R) -> Self {
        let buf_reader = BufReader::new(source);
        let mut reader = XmlReader::from_reader(buf_reader);
        reader.config_mut().trim_text(false);
        Self {
            reader,
            archive: None,
            relationships: None,
            numbering: None,
            security: SecurityPolicy::new(),
        }
    }

    /// 流式遍历文档，将 [`DocumentEvent`] 推送给 `sink`。
    ///
    /// 开头发出 [`DocumentEvent::DocumentStart`]，结尾发出 [`DocumentEvent::DocumentEnd`]。
    ///
    /// **注意：** [`DocumentBlock::Math`] 无法表示为 [`DocumentEvent`]，会被静默丢弃。
    /// 如需提取数学公式，请使用 [`read_blocks`](Self::read_blocks)。
    ///
    /// # Errors
    ///
    /// XML 解析失败或 sink 返回错误时返回错误。
    pub fn read_events(&mut self, sink: &mut dyn EventSink) -> Result<()> {
        let mut adapter = EventSinkAdapter(sink);
        self.parse_with_sink(&mut adapter)
    }

    /// 读取文档并按文档顺序返回所有块，包括 OMML 公式的 [`DocumentBlock::Math`]。
    ///
    /// 需要提取数学公式时推荐使用此入口。返回的块保留原始文档顺序：
    /// 段落、表格、图片和数学公式按与源 DOCX 相同的顺序出现。
    ///
    /// Math 块的 `omml` 设置为原始 `<m:oMath>` 或 `<m:oMathPara>` XML，
    /// `latex` 为 `None`，`display` 指示公式是块级（`<m:oMathPara>`）还是行内（`<m:oMath>`）。
    ///
    /// # Errors
    ///
    /// XML 解析失败时返回错误。
    pub fn read_blocks(&mut self) -> Result<Vec<DocumentBlock>> {
        let mut collector = BlockCollector(Vec::new());
        self.parse_with_sink(&mut collector)?;
        Ok(collector.0)
    }

    /// Core parsing loop shared by [`read_events`](Self::read_events) and
    /// [`read_blocks`](Self::read_blocks).
    fn parse_with_sink(&mut self, sink: &mut dyn ParseSink) -> Result<()> {
        sink.on_event(&DocumentEvent::DocumentStart)?;

        let mut state_stack: Vec<ParseState> = vec![ParseState::Document];
        let mut buf = Vec::new();

        // List accumulation state: consecutive `<w:numPr>` paragraphs are
        // collected here and flushed as a single `DocumentBlock::List` when a
        // non-list paragraph, heading, or table boundary is encountered.
        // Each entry is `(item, ilvl)` where `ilvl` drives nesting.
        let mut list_items: Vec<(DocumentListItem, u8)> = Vec::new();
        // The numId/ilvl from the first list item in the current run, used to
        // look up ordered/start_number from the numbering definitions.
        let mut first_list_num_id: Option<u32> = None;
        let mut first_list_ilvl: u8 = 0;

        // Math accumulation state.
        let mut in_math = false;
        let mut math_is_para = false;
        let mut math_depth: u32 = 0;
        let mut math_xml_buf = String::new();

        loop {
            let event = self
                .reader
                .read_event_into(&mut buf)
                .map_err(|e| DocError::Format(format!("XML parse error: {e}")))?;

            // ---- Math accumulation mode ----
            // When inside an <m:oMath> or <m:oMathPara>, accumulate raw XML
            // bytes for every event (start tags, text, end tags, etc.).
            if in_math {
                match &event {
                    Event::Start(start) => {
                        let name = start.name();
                        let name_bytes = name.as_ref();
                        if name_bytes == M_OMATH || name_bytes == M_OMATHPARA {
                            math_depth += 1;
                        }
                        math_xml_buf.push('<');
                        math_xml_buf.push_str(std::str::from_utf8(start.as_ref()).unwrap_or(""));
                        math_xml_buf.push('>');
                    }
                    Event::End(end) => {
                        let name = end.name();
                        let name_bytes = name.as_ref();

                        // Append closing tag to buffer first.
                        math_xml_buf.push_str("</");
                        math_xml_buf.push_str(std::str::from_utf8(name_bytes).unwrap_or(""));
                        math_xml_buf.push('>');

                        // Check whether this end tag closes the root math
                        // element that started the accumulation.
                        let is_closing_root = if math_is_para {
                            name_bytes == M_OMATHPARA
                        } else {
                            name_bytes == M_OMATH
                        };

                        if name_bytes == M_OMATH || name_bytes == M_OMATHPARA {
                            math_depth = math_depth.saturating_sub(1);
                        }

                        if is_closing_root && math_depth == 0 {
                            sink.push_block(DocumentBlock::Math {
                                omml: Some(std::mem::take(&mut math_xml_buf)),
                                latex: None,
                                display: math_is_para,
                            });
                            in_math = false;
                        }
                    }
                    Event::Empty(empty) => {
                        math_xml_buf.push('<');
                        math_xml_buf.push_str(std::str::from_utf8(empty.as_ref()).unwrap_or(""));
                        math_xml_buf.push_str("/>");
                    }
                    Event::Text(text) => {
                        math_xml_buf.push_str(std::str::from_utf8(text.as_ref()).unwrap_or(""));
                    }
                    _ => {}
                }
                buf.clear();
                continue;
            }

            // ---- Normal processing ----
            match event {
                Event::Eof => break,
                Event::Start(ref start) => {
                    let name = start.name();
                    let name_bytes = name.as_ref();
                    if name_bytes == M_OMATH || name_bytes == M_OMATHPARA {
                        // Flush any accumulated paragraph runs before entering
                        // math so they appear as a separate Paragraph block.
                        flush_paragraph_runs(sink, &mut state_stack)?;
                        in_math = true;
                        math_is_para = name_bytes == M_OMATHPARA;
                        math_depth = 1;
                        math_xml_buf.clear();
                        math_xml_buf.push('<');
                        math_xml_buf.push_str(std::str::from_utf8(start.as_ref()).unwrap_or(""));
                        math_xml_buf.push('>');
                    } else {
                        handle_start(start, &mut state_stack)?;
                    }
                }
                Event::Empty(ref empty) => {
                    let name = empty.name();
                    let name_bytes = name.as_ref();
                    if name_bytes == M_OMATH || name_bytes == M_OMATHPARA {
                        // Self-closing math element (rare but possible).
                        flush_paragraph_runs(sink, &mut state_stack)?;
                        let display = name_bytes == M_OMATHPARA;
                        let mut xml = String::from("<");
                        xml.push_str(std::str::from_utf8(empty.as_ref()).unwrap_or(""));
                        xml.push_str("/>");
                        sink.push_block(DocumentBlock::Math {
                            omml: Some(xml),
                            latex: None,
                            display,
                        });
                    } else {
                        handle_empty(empty, sink, &mut state_stack)?;
                    }
                }
                Event::Text(ref text) => {
                    handle_text(text, &mut state_stack)?;
                }
                Event::End(ref end) => {
                    handle_end(
                        end,
                        sink,
                        &mut state_stack,
                        &mut ParseContext {
                            archive: self.archive.as_mut(),
                            relationships: self.relationships.as_ref(),
                            numbering: self.numbering.as_ref(),
                            list_items: &mut list_items,
                            first_list_num_id: &mut first_list_num_id,
                            first_list_ilvl: &mut first_list_ilvl,
                            ssrf: &self.security.ssrf,
                        },
                    )?;
                }
                _ => {}
            }

            buf.clear();
        }

        // Flush any remaining list items at document end.
        flush_list(
            sink,
            &mut list_items,
            &mut first_list_num_id,
            &mut first_list_ilvl,
            self.numbering.as_ref(),
            self.relationships.as_ref(),
            &self.security.ssrf,
        )?;

        sink.on_event(&DocumentEvent::DocumentEnd)?;
        sink.on_complete();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Element handlers (free functions -- no &mut self needed)
// ---------------------------------------------------------------------------

fn handle_start(start: &quick_xml::events::BytesStart, stack: &mut Vec<ParseState>) -> Result<()> {
    let name = start.name();
    let local = name.as_ref();

    match local {
        W_P => {
            // Inside a table cell, paragraphs are structural wrappers for
            // text content that should flow into the cell buffer, not be
            // emitted as separate events.
            if !inside_table(stack) {
                stack.push(ParseState::Paragraph {
                    runs: Vec::new(),
                    heading_level: None,
                    in_ppr: false,
                    in_run: false,
                    in_rpr: false,
                    run_bold: false,
                    run_italic: false,
                    run_strike: false,
                    text_buf: String::new(),
                    in_text: false,
                    preserve_space: false,
                    has_num_pr: false,
                    num_id: None,
                    ilvl: None,
                    in_hyperlink: false,
                    hyperlink_rid: None,
                });
            }
        }
        W_TBL => {
            stack.push(ParseState::Table {
                rows: Vec::new(),
                current_row: None,
            });
        }
        W_DRAWING => {
            stack.push(ParseState::Drawing {
                pending_rid: None,
                pending_alt: None,
            });
        }
        W_PPR => {
            if let Some(ParseState::Paragraph { in_ppr, .. }) = stack.last_mut() {
                *in_ppr = true;
            }
        }
        W_NUMPR => {
            // `<w:numPr>` inside `<w:pPr>` marks this paragraph as a list item.
            if let Some(ParseState::Paragraph { has_num_pr, .. }) = stack.last_mut() {
                *has_num_pr = true;
            }
        }
        W_NUMID => {
            if let Some(ParseState::Paragraph { num_id, .. }) = stack.last_mut() {
                *num_id = extract_val(start).and_then(|v| v.parse::<u32>().ok());
            }
        }
        W_ILVL => {
            if let Some(ParseState::Paragraph { ilvl, .. }) = stack.last_mut() {
                *ilvl = extract_val(start).and_then(|v| v.parse::<u8>().ok());
            }
        }
        W_HYPERLINK => {
            // `<w:hyperlink r:id="...">` inside a paragraph marks the following
            // runs as hyperlink content.  The `r:id` is stored for resolution.
            if let Some(ParseState::Paragraph {
                in_hyperlink,
                hyperlink_rid,
                ..
            }) = stack.last_mut()
            {
                *in_hyperlink = true;
                *hyperlink_rid = extract_rid(start);
            }
        }
        W_PSTYLE => {
            if let Some(ParseState::Paragraph { heading_level, .. }) = stack.last_mut() {
                *heading_level = extract_val(start).and_then(|v| parse_heading_level(&v));
            }
        }
        W_R => {
            if let Some(ParseState::Paragraph {
                in_run,
                run_bold,
                run_italic,
                run_strike,
                ..
            }) = stack.last_mut()
            {
                *in_run = true;
                *run_bold = false;
                *run_italic = false;
                *run_strike = false;
            }
        }
        W_RPR => {
            if let Some(ParseState::Paragraph { in_rpr, .. }) = stack.last_mut() {
                *in_rpr = true;
            }
        }
        W_B => {
            if let Some(ParseState::Paragraph {
                in_rpr, run_bold, ..
            }) = stack.last_mut()
                && *in_rpr
            {
                *run_bold = extract_bool_attr(start).unwrap_or(true);
            }
        }
        W_I => {
            if let Some(ParseState::Paragraph {
                in_rpr, run_italic, ..
            }) = stack.last_mut()
                && *in_rpr
            {
                *run_italic = extract_bool_attr(start).unwrap_or(true);
            }
        }
        W_STRIKE => {
            if let Some(ParseState::Paragraph {
                in_rpr, run_strike, ..
            }) = stack.last_mut()
                && *in_rpr
            {
                *run_strike = extract_bool_attr(start).unwrap_or(true);
            }
        }
        W_T => {
            if let Some(ParseState::Paragraph {
                in_text,
                preserve_space,
                ..
            }) = stack.last_mut()
            {
                *in_text = true;
                *preserve_space = has_preserve_space(start);
            }
        }
        W_TR => {
            if let Some(ParseState::Table { current_row, .. }) = stack.last_mut() {
                *current_row = Some(TableRowBuilder { cells: Vec::new() });
            }
        }
        W_TC => {
            if let Some(ParseState::Table {
                current_row: Some(row),
                ..
            }) = stack.last_mut()
            {
                row.cells.push(TableCellBuilder {
                    text: String::new(),
                    column_span: 1,
                    row_span: 1,
                    v_merge: None,
                    in_tcpr: false,
                    blocks: Vec::new(),
                });
            }
        }
        W_TCPR => {
            if let Some(ParseState::Table {
                current_row: Some(row),
                ..
            }) = stack.last_mut()
                && let Some(cell) = row.cells.last_mut()
            {
                cell.in_tcpr = true;
            }
        }
        _ => {}
    }

    Ok(())
}

fn handle_empty(
    empty: &quick_xml::events::BytesStart,
    sink: &mut dyn ParseSink,
    stack: &mut [ParseState],
) -> Result<()> {
    let name = empty.name();
    let local = name.as_ref();

    match local {
        W_BR => {
            if br_is_page_break(empty) {
                sink.on_event(&DocumentEvent::PageBreak)?;
            }
        }
        W_NUMID => {
            if let Some(ParseState::Paragraph { num_id, .. }) = stack.last_mut() {
                *num_id = extract_val(empty).and_then(|v| v.parse::<u32>().ok());
            }
        }
        W_ILVL => {
            if let Some(ParseState::Paragraph { ilvl, .. }) = stack.last_mut() {
                *ilvl = extract_val(empty).and_then(|v| v.parse::<u8>().ok());
            }
        }
        W_PSTYLE => {
            if let Some(ParseState::Paragraph { heading_level, .. }) = stack.last_mut() {
                *heading_level = extract_val(empty).and_then(|v| parse_heading_level(&v));
            }
        }
        W_B => {
            if let Some(ParseState::Paragraph {
                in_rpr, run_bold, ..
            }) = stack.last_mut()
                && *in_rpr
            {
                *run_bold = extract_bool_attr(empty).unwrap_or(true);
            }
        }
        W_I => {
            if let Some(ParseState::Paragraph {
                in_rpr, run_italic, ..
            }) = stack.last_mut()
                && *in_rpr
            {
                *run_italic = extract_bool_attr(empty).unwrap_or(true);
            }
        }
        W_STRIKE => {
            if let Some(ParseState::Paragraph {
                in_rpr, run_strike, ..
            }) = stack.last_mut()
                && *in_rpr
            {
                *run_strike = extract_bool_attr(empty).unwrap_or(true);
            }
        }
        A_BLIP => {
            if let Some(ParseState::Drawing { pending_rid, .. }) = stack.last_mut() {
                for attr in empty.attributes().flatten() {
                    if attr.key.as_ref() == R_EMBED {
                        *pending_rid = attr
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .ok()
                            .map(Cow::into_owned);
                        break;
                    }
                }
            }
        }
        W_GRIDSPAN => {
            // <w:gridSpan w:val="N"/> -- horizontal merge across N columns.
            if let Some(ParseState::Table {
                current_row: Some(row),
                ..
            }) = stack.last_mut()
                && let Some(cell) = row.cells.last_mut()
                && cell.in_tcpr
                && let Some(val) = extract_val(empty)
                && let Ok(n) = val.parse::<u32>()
                && n > 1
            {
                cell.column_span = n;
            }
        }
        W_VMERGE => {
            // <w:vMerge w:val="restart"/> or <w:vMerge/> or <w:vMerge w:val="continue"/>
            // restart = first cell in vertical merge (row_span=1)
            // no val / continue = continuation cell merged into cell above (row_span=0)
            if let Some(ParseState::Table {
                current_row: Some(row),
                ..
            }) = stack.last_mut()
                && let Some(cell) = row.cells.last_mut()
                && cell.in_tcpr
            {
                if let Some("restart") = extract_val(empty).as_deref() {
                    cell.v_merge = Some(VMerge::Restart);
                    cell.row_span = 1;
                } else {
                    // No val or val="continue" => continuation cell.
                    cell.v_merge = Some(VMerge::Continue);
                    cell.row_span = 0;
                }
            }
        }
        WP_DOC_PR => {
            if let Some(ParseState::Drawing { pending_alt, .. }) = stack.last_mut() {
                for attr in empty.attributes().flatten() {
                    if attr.key.as_ref() == b"descr" {
                        let val = attr
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .ok()
                            .map(Cow::into_owned);
                        if val.as_deref().is_some_and(|s| !s.is_empty()) {
                            *pending_alt = val;
                        }
                        break;
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn handle_text(text: &quick_xml::events::BytesText, stack: &mut [ParseState]) -> Result<()> {
    // Accumulate text into the appropriate buffer depending on current state.
    // OOXML is always UTF-8, so we can decode the raw bytes directly.
    let decoded = std::str::from_utf8(text.as_ref()).unwrap_or("").to_owned();
    if let Some(state) = stack.last_mut() {
        match state {
            ParseState::Paragraph {
                in_text: true,
                text_buf,
                ..
            } => {
                text_buf.push_str(&decoded);
            }
            ParseState::Table {
                current_row: Some(row),
                ..
            } => {
                if let Some(cell) = row.cells.last_mut() {
                    cell.text.push_str(&decoded);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Context for `handle_end`, grouping references to shared parse state.
struct ParseContext<'a> {
    archive: Option<&'a mut zip::ZipArchive<File>>,
    relationships: Option<&'a Relationships>,
    numbering: Option<&'a super::numbering::Numbering>,
    /// Flat list of accumulated items with their indentation levels.
    /// Each entry is `(item, ilvl)` where `ilvl` comes from `<w:ilvl>`.
    list_items: &'a mut Vec<(DocumentListItem, u8)>,
    first_list_num_id: &'a mut Option<u32>,
    first_list_ilvl: &'a mut u8,
    /// SSRF guard for hyperlink URL validation.
    ssrf: &'a SsrfGuard,
}

fn handle_end(
    end: &quick_xml::events::BytesEnd,
    sink: &mut dyn ParseSink,
    stack: &mut Vec<ParseState>,
    ctx: &mut ParseContext<'_>,
) -> Result<()> {
    let name = end.name();
    let local = name.as_ref();

    match local {
        W_P => {
            // Only pop and emit if a Paragraph state is on top. When inside
            // a table cell, no Paragraph state was pushed so we skip.
            if matches!(stack.last(), Some(ParseState::Paragraph { .. }))
                && let Some(ParseState::Paragraph {
                    runs,
                    heading_level,
                    has_num_pr,
                    num_id,
                    ilvl,
                    ..
                }) = stack.pop()
            {
                if let Some(level) = heading_level {
                    // Headings always flush the list first, then emit as heading.
                    flush_list(
                        sink,
                        ctx.list_items,
                        ctx.first_list_num_id,
                        ctx.first_list_ilvl,
                        ctx.numbering,
                        ctx.relationships,
                        ctx.ssrf,
                    )?;
                    sink.on_event(&DocumentEvent::Heading { level, runs })?;
                } else if has_num_pr {
                    // List item -- accumulate into the current list.
                    // Record the first item's numId/ilvl for list-level lookup.
                    let item_ilvl = ilvl.unwrap_or(0);
                    if ctx.list_items.is_empty() {
                        *ctx.first_list_num_id = num_id;
                        *ctx.first_list_ilvl = item_ilvl;
                    }
                    if !runs.is_empty() {
                        ctx.list_items.push((
                            DocumentListItem {
                                blocks: vec![DocumentBlock::Paragraph(runs)],
                                nested: None,
                            },
                            item_ilvl,
                        ));
                    }
                } else {
                    // Non-list paragraph -- flush any accumulated list first.
                    flush_list(
                        sink,
                        ctx.list_items,
                        ctx.first_list_num_id,
                        ctx.first_list_ilvl,
                        ctx.numbering,
                        ctx.relationships,
                        ctx.ssrf,
                    )?;
                    if !runs.is_empty() {
                        // Skip empty paragraphs (e.g. those left after math flush).
                        sink.on_event(&DocumentEvent::Paragraph(runs))?;
                    }
                }
            }
        }
        W_R => {
            if let Some(ParseState::Paragraph {
                in_run,
                text_buf,
                run_bold,
                run_italic,
                run_strike,
                runs,
                in_hyperlink,
                hyperlink_rid,
                ..
            }) = stack.last_mut()
            {
                if *in_run && !text_buf.is_empty() {
                    let hyperlink = if *in_hyperlink {
                        hyperlink_rid.as_ref().map(|rid| {
                            // Resolve rId to actual URL via relationships.
                            // Fallback: use raw rId if not resolved.
                            match ctx
                                .relationships
                                .and_then(|rels| rels.resolve_hyperlink(rid))
                            {
                                Some(resolved_url) => {
                                    // SSRF guard: only check resolved URLs.
                                    if ctx.ssrf.check_url(resolved_url).is_ok() {
                                        resolved_url.to_owned()
                                    } else {
                                        // Blocked URL: keep raw rId as fallback.
                                        rid.clone()
                                    }
                                }
                                None => rid.clone(),
                            }
                        })
                    } else {
                        None
                    };
                    runs.push(DocumentTextRun {
                        text: std::mem::take(text_buf),
                        bold: *run_bold,
                        italic: *run_italic,
                        strikethrough: *run_strike,
                        hyperlink,
                    });
                }
                *in_run = false;
                *run_bold = false;
                *run_italic = false;
                *run_strike = false;
            }
        }
        W_HYPERLINK => {
            // Closing `</w:hyperlink>` clears the hyperlink state so subsequent
            // runs in the same paragraph are not tagged.
            if let Some(ParseState::Paragraph {
                in_hyperlink,
                hyperlink_rid,
                ..
            }) = stack.last_mut()
            {
                *in_hyperlink = false;
                *hyperlink_rid = None;
            }
        }
        W_T => {
            if let Some(ParseState::Paragraph { in_text, .. }) = stack.last_mut() {
                *in_text = false;
            }
        }
        W_PPR => {
            if let Some(ParseState::Paragraph { in_ppr, .. }) = stack.last_mut() {
                *in_ppr = false;
            }
        }
        W_RPR => {
            if let Some(ParseState::Paragraph { in_rpr, .. }) = stack.last_mut() {
                *in_rpr = false;
            }
        }
        W_TCPR => {
            if let Some(ParseState::Table {
                current_row: Some(row),
                ..
            }) = stack.last_mut()
                && let Some(cell) = row.cells.last_mut()
            {
                cell.in_tcpr = false;
            }
        }
        W_TBL => {
            if let Some(ParseState::Table { rows, .. }) = stack.pop() {
                let table = easydoc_core::DocumentTable { rows };
                if inside_table(stack) {
                    // Nested table -- store as a block in the parent cell.
                    if let Some(ParseState::Table {
                        current_row: Some(row),
                        ..
                    }) = stack.last_mut()
                        && let Some(cell) = row.cells.last_mut()
                    {
                        cell.blocks.push(DocumentBlock::Table(table));
                    }
                } else {
                    // Top-level table -- flush any pending list, then emit.
                    flush_list(
                        sink,
                        ctx.list_items,
                        ctx.first_list_num_id,
                        ctx.first_list_ilvl,
                        ctx.numbering,
                        ctx.relationships,
                        ctx.ssrf,
                    )?;
                    sink.on_event(&DocumentEvent::Table(table))?;
                }
            }
        }
        W_TR => {
            if let Some(ParseState::Table {
                current_row, rows, ..
            }) = stack.last_mut()
                && let Some(row_builder) = current_row.take()
            {
                let cells = row_builder
                    .cells
                    .into_iter()
                    .map(|c| {
                        let mut blocks = Vec::new();
                        let trimmed = c.text.trim().to_owned();
                        if !trimmed.is_empty() {
                            blocks.push(easydoc_core::DocumentBlock::Paragraph(vec![
                                DocumentTextRun {
                                    text: trimmed,
                                    ..DocumentTextRun::default()
                                },
                            ]));
                        }
                        blocks.extend(c.blocks);
                        DocumentTableCell {
                            blocks,
                            column_span: c.column_span,
                            row_span: c.row_span,
                        }
                    })
                    .collect();
                rows.push(DocumentTableRow {
                    cells,
                    is_header: false,
                });
            }
        }
        W_DRAWING => {
            // Pop the Drawing state and attempt to extract real image data.
            if let Some(ParseState::Drawing {
                pending_rid,
                pending_alt,
            }) = stack.pop()
            {
                let (data, extension) = if let (Some(rid), Some(arch), Some(rels)) = (
                    pending_rid.as_ref(),
                    ctx.archive.as_deref_mut(),
                    ctx.relationships,
                ) {
                    if let Some(part_path) = rels.resolve(rid) {
                        match read_zip_part(arch, part_path) {
                            Ok(bytes) => (Some(bytes), extension_from_filename(part_path)),
                            Err(_) => (None, None),
                        }
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };

                let alt_text = pending_alt.or_else(|| Some("[image]".to_owned()));

                sink.on_event(&DocumentEvent::Image(DocumentImage {
                    alt_text,
                    data,
                    extension,
                }))?;
            }
        }
        _ => {}
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Returns `true` if the stack contains a `Table` state (i.e. we are inside a
/// `<w:tbl>` element).
fn inside_table(stack: &[ParseState]) -> bool {
    stack.iter().any(|s| matches!(s, ParseState::Table { .. }))
}

/// Extracts the `w:val` attribute value from a start tag.
fn extract_val(tag: &quick_xml::events::BytesStart) -> Option<String> {
    for attr in tag.attributes().flatten() {
        if attr.key.as_ref() == W_VAL {
            return attr
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(std::borrow::Cow::into_owned);
        }
    }
    None
}

/// Extracts a boolean attribute: `w:val="true"` / `w:val="false"`.
/// If no `w:val` is present, returns `Some(true)` (OOXML convention).
fn extract_bool_attr(tag: &quick_xml::events::BytesStart) -> Option<bool> {
    match extract_val(tag) {
        Some(v) => {
            let lower = v.to_lowercase();
            if lower == "false" || lower == "0" {
                Some(false)
            } else {
                Some(true)
            }
        }
        None => Some(true),
    }
}

/// Checks `xml:space="preserve"` on a `<w:t>` tag.
fn has_preserve_space(tag: &quick_xml::events::BytesStart) -> bool {
    for attr in tag.attributes().flatten() {
        if attr.key.as_ref() == b"xml:space" {
            return attr
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .is_some_and(|v| v.as_ref() == "preserve");
        }
    }
    false
}

/// Parses a heading level from a style name like `"Heading1"` -> `1`.
fn parse_heading_level(style: &str) -> Option<u8> {
    let trimmed = style.trim();
    // Handle both "Heading1" and "heading 1" variants.
    let lower = trimmed.to_lowercase();
    let digits = lower
        .strip_prefix("heading")
        .or_else(|| lower.strip_prefix("heading "));
    digits
        .and_then(|d| d.trim().parse::<u8>().ok())
        .filter(|&l| (1..=6).contains(&l))
}

/// Checks if a `<w:br>` element is a page break.
fn br_is_page_break(tag: &quick_xml::events::BytesStart) -> bool {
    for attr in tag.attributes().flatten() {
        if attr.key.as_ref() == W_TYPE {
            return attr
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .is_some_and(|v| v.as_ref() == "page");
        }
    }
    false
}

/// Searches for `word/document.xml` case-insensitively in a ZIP archive.
fn find_word_document_xml(archive: &mut zip::ZipArchive<File>) -> Result<String> {
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| DocError::Zip(e.to_string()))?;
        if entry.name().to_lowercase() == "word/document.xml" {
            return Ok(entry.name().to_owned());
        }
    }
    Err(DocError::Format(
        "word/document.xml not found in DOCX archive".to_owned(),
    ))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use easydoc_core::ContentCollector;

    /// Helper: wraps raw OOXML XML into a minimal valid DOCX ZIP archive in
    /// memory and returns the bytes.
    fn make_docx_xml(xml: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let w = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(w);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file("word/document.xml", options).unwrap();
            zip.write_all(xml).unwrap();
            zip.finish().unwrap();
        }
        buf
    }

    /// Writes a docx zip to a temp file and returns the path.
    fn write_temp_docx(xml: &[u8]) -> tempfile::NamedTempFile {
        let data = make_docx_xml(xml);
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut tmp, &data).unwrap();
        tmp
    }

    #[test]
    fn empty_document_emits_start_end() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body/>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        assert!(content.blocks.is_empty());
    }

    #[test]
    fn simple_paragraph() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>Hello World</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        assert_eq!(content.blocks.len(), 1);
        match &content.blocks[0] {
            easydoc_core::DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs.len(), 1);
                assert_eq!(runs[0].text, "Hello World");
            }
            _ => panic!("expected Paragraph"),
        }
    }

    #[test]
    fn heading_detection() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Title</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        assert_eq!(content.blocks.len(), 1);
        match &content.blocks[0] {
            easydoc_core::DocumentBlock::Heading { level, runs } => {
                assert_eq!(*level, 1);
                assert_eq!(runs[0].text, "Title");
            }
            _ => panic!("expected Heading"),
        }
    }

    #[test]
    fn bold_run() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:rPr><w:b/></w:rPr><w:t>Bold</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        match &content.blocks[0] {
            easydoc_core::DocumentBlock::Paragraph(runs) => {
                assert!(runs[0].bold);
                assert!(!runs[0].italic);
            }
            _ => panic!("expected Paragraph"),
        }
    }

    #[test]
    fn page_break() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:br w:type="page"/></w:r>
      <w:r><w:t>After break</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        // PageBreak + Paragraph
        assert_eq!(content.blocks.len(), 2);
        assert!(matches!(
            content.blocks[0],
            easydoc_core::DocumentBlock::PageBreak
        ));
    }

    #[test]
    fn simple_table() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>B2</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        assert_eq!(content.blocks.len(), 1);
        match &content.blocks[0] {
            easydoc_core::DocumentBlock::Table(table) => {
                assert_eq!(table.rows.len(), 2);
                assert_eq!(table.rows[0].cells.len(), 2);
                // Check cell text
                let cell_text: String = table.rows[0].cells[0]
                    .blocks
                    .iter()
                    .filter_map(|b| match b {
                        easydoc_core::DocumentBlock::Paragraph(runs) => {
                            Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(cell_text, "A1");
            }
            _ => panic!("expected Table"),
        }
    }

    #[test]
    fn mixed_content_paragraph_and_table() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Before</w:t></w:r></w:p>
    <w:tbl>
      <w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr>
    </w:tbl>
    <w:p><w:r><w:t>After</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        assert_eq!(content.blocks.len(), 3);
        assert!(matches!(
            content.blocks[0],
            easydoc_core::DocumentBlock::Paragraph(_)
        ));
        assert!(matches!(
            content.blocks[1],
            easydoc_core::DocumentBlock::Table(_)
        ));
        assert!(matches!(
            content.blocks[2],
            easydoc_core::DocumentBlock::Paragraph(_)
        ));
    }

    #[test]
    fn from_reader_basic() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Direct</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        assert_eq!(content.blocks.len(), 1);
    }

    #[test]
    fn parse_heading_level_variants() {
        assert_eq!(parse_heading_level("Heading1"), Some(1));
        assert_eq!(parse_heading_level("Heading2"), Some(2));
        assert_eq!(parse_heading_level("heading3"), Some(3));
        assert_eq!(parse_heading_level("heading 4"), Some(4));
        assert_eq!(parse_heading_level("Heading7"), None);
        assert_eq!(parse_heading_level("Normal"), None);
        assert_eq!(parse_heading_level("Title"), None);
    }

    #[test]
    fn drawing_emits_placeholder_image() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:drawing><wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"/></w:drawing></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        let has_image = content
            .blocks
            .iter()
            .any(|b| matches!(b, easydoc_core::DocumentBlock::Image(_)));
        assert!(has_image, "expected an Image block from drawing");
    }

    #[test]
    fn italic_and_strikethrough() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:rPr><w:i/><w:strike/></w:rPr><w:t>Fancy</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        match &content.blocks[0] {
            easydoc_core::DocumentBlock::Paragraph(runs) => {
                assert!(runs[0].italic);
                assert!(runs[0].strikethrough);
                assert!(!runs[0].bold);
            }
            _ => panic!("expected Paragraph"),
        }
    }

    #[test]
    fn multiple_runs_in_paragraph() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>Hello </w:t></w:r>
      <w:r><w:rPr><w:b/></w:rPr><w:t>World</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();
        match &content.blocks[0] {
            easydoc_core::DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs.len(), 2);
                assert_eq!(runs[0].text, "Hello ");
                assert!(!runs[0].bold);
                assert_eq!(runs[1].text, "World");
                assert!(runs[1].bold);
            }
            _ => panic!("expected Paragraph"),
        }
    }

    /// A minimal 1x1 white PNG (67 bytes).
    const MINIMAL_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8,
        0xcf, 0xc0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xe2, 0x21, 0xbc, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    /// Builds a DOCX ZIP containing `word/document.xml`, `word/_rels/document.xml.rels`,
    /// and `word/media/image1.png` with the given image bytes.
    fn make_docx_with_image(xml: &[u8], rels_xml: &[u8], image_bytes: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let w = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(w);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            zip.start_file("word/document.xml", options).unwrap();
            zip.write_all(xml).unwrap();

            zip.start_file("word/_rels/document.xml.rels", options)
                .unwrap();
            zip.write_all(rels_xml).unwrap();

            zip.start_file("word/media/image1.png", options).unwrap();
            zip.write_all(image_bytes).unwrap();

            zip.finish().unwrap();
        }
        buf
    }

    /// Writes a full docx ZIP (with image) to a temp file and returns the path.
    fn write_temp_docx_with_image(
        xml: &[u8],
        rels_xml: &[u8],
        image_bytes: &[u8],
    ) -> tempfile::NamedTempFile {
        let data = make_docx_with_image(xml, rels_xml, image_bytes);
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut tmp, &data).unwrap();
        tmp
    }

    #[test]
    fn from_reader_drawing_emits_placeholder_no_data() {
        // from_reader path has no ZIP, so image data must be None.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <w:body>
    <w:p>
      <w:r>
        <w:drawing>
          <wp:inline>
            <wp:docPr id="1" name="Picture 1" descr="My photo"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
                  <pic:blipFill>
                    <a:blip r:embed="rId5"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </w:drawing>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let img = content
            .blocks
            .iter()
            .find_map(|b| match b {
                easydoc_core::DocumentBlock::Image(img) => Some(img),
                _ => None,
            })
            .expect("expected an Image block");

        // No ZIP archive => data is None.
        assert!(img.data.is_none());
        // Alt text should come from wp:docPr descr.
        assert_eq!(img.alt_text.as_deref(), Some("My photo"));
    }

    #[test]
    fn from_path_drawing_extracts_real_image_data() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <w:body>
    <w:p>
      <w:r>
        <w:drawing>
          <wp:inline>
            <wp:docPr id="1" name="Picture 1" descr="A tiny image"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
                  <pic:blipFill>
                    <a:blip r:embed="rId5"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </w:drawing>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Target="styles.xml" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles"/>
  <Relationship Id="rId5" Target="media/image1.png" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"/>
</Relationships>"#;

        let tmp = write_temp_docx_with_image(xml, rels_xml, MINIMAL_PNG);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let img = content
            .blocks
            .iter()
            .find_map(|b| match b {
                easydoc_core::DocumentBlock::Image(img) => Some(img),
                _ => None,
            })
            .expect("expected an Image block");

        // Image data should be the real PNG bytes.
        assert_eq!(img.data.as_deref(), Some(MINIMAL_PNG));
        // Extension should be inferred from the media path.
        assert_eq!(img.extension.as_deref(), Some("png"));
        // Alt text from wp:docPr descr.
        assert_eq!(img.alt_text.as_deref(), Some("A tiny image"));
    }

    #[test]
    fn from_path_drawing_without_rels_emits_placeholder() {
        // ZIP has document.xml but no rels file => image data is None.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <w:body>
    <w:p>
      <w:r>
        <w:drawing>
          <wp:inline>
            <wp:docPr id="1" name="Pic"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
                  <pic:blipFill>
                    <a:blip r:embed="rId5"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </w:drawing>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let tmp = write_temp_docx(xml);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let img = content
            .blocks
            .iter()
            .find_map(|b| match b {
                easydoc_core::DocumentBlock::Image(img) => Some(img),
                _ => None,
            })
            .expect("expected an Image block");

        // No rels file => data is None.
        assert!(img.data.is_none());
        // wp:docPr name is an object label, not alt text; descr is absent,
        // so the fallback "[image]" is used.
        assert_eq!(img.alt_text.as_deref(), Some("[image]"));
    }

    #[test]
    fn from_path_drawing_alt_from_name_when_no_descr() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <w:body>
    <w:p>
      <w:r>
        <w:drawing>
          <wp:inline>
            <wp:docPr id="1" name="Diagram"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
                  <pic:blipFill>
                    <a:blip r:embed="rId5"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </w:drawing>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId5" Target="media/image1.png" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"/>
</Relationships>"#;

        let tmp = write_temp_docx_with_image(xml, rels_xml, MINIMAL_PNG);
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let img = content
            .blocks
            .iter()
            .find_map(|b| match b {
                easydoc_core::DocumentBlock::Image(img) => Some(img),
                _ => None,
            })
            .expect("expected an Image block");

        // No descr attribute, but name="Diagram" is present.  Since we prefer
        // descr and fall back to "[image]", the alt should be "[image]".
        // (wp:docPr name is the object label, not the alt text.)
        assert_eq!(img.alt_text.as_deref(), Some("[image]"));
    }

    #[test]
    fn from_path_drawing_jpeg_extension() {
        use std::io::Write;
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <w:body>
    <w:p>
      <w:r>
        <w:drawing>
          <wp:inline>
            <wp:docPr id="1" name="Pic"/>
            <a:graphic>
              <a:graphicData>
                <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
                  <pic:blipFill>
                    <a:blip r:embed="rId5"/>
                  </pic:blipFill>
                </pic:pic>
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </w:drawing>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId5" Target="media/photo.jpeg" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"/>
</Relationships>"#;

        // Build a ZIP with the media entry matching the rels target.
        let zip_bytes = {
            let mut buf = Vec::new();
            {
                let w = std::io::Cursor::new(&mut buf);
                let mut zip = zip::ZipWriter::new(w);
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored);

                zip.start_file("word/document.xml", options).unwrap();
                zip.write_all(xml).unwrap();

                zip.start_file("word/_rels/document.xml.rels", options)
                    .unwrap();
                zip.write_all(rels_xml).unwrap();

                // Match the rels target: word/media/photo.jpeg
                zip.start_file("word/media/photo.jpeg", options).unwrap();
                zip.write_all(MINIMAL_PNG).unwrap();

                zip.finish().unwrap();
            }
            buf
        };

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut tmp, &zip_bytes).unwrap();
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let img = content
            .blocks
            .iter()
            .find_map(|b| match b {
                easydoc_core::DocumentBlock::Image(img) => Some(img),
                _ => None,
            })
            .expect("expected an Image block");

        assert_eq!(img.data.as_deref(), Some(MINIMAL_PNG));
        assert_eq!(img.extension.as_deref(), Some("jpeg"));
    }

    // -----------------------------------------------------------------------
    // Merge cell tests
    // -----------------------------------------------------------------------

    #[test]
    fn cell_without_merge_has_default_span() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let easydoc_core::DocumentBlock::Table(table) = &content.blocks[0] else {
            panic!("expected Table")
        };
        assert_eq!(table.rows[0].cells.len(), 2);
        assert_eq!(table.rows[0].cells[0].column_span, 1);
        assert_eq!(table.rows[0].cells[0].row_span, 1);
        assert_eq!(table.rows[0].cells[1].column_span, 1);
        assert_eq!(table.rows[0].cells[1].row_span, 1);
    }

    #[test]
    fn gridspan_horizontal_merge_two_columns() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:tcPr><w:gridSpan w:val="2"/></w:tcPr>
          <w:p><w:r><w:t>Merged</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>B2</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let easydoc_core::DocumentBlock::Table(table) = &content.blocks[0] else {
            panic!("expected Table")
        };
        // Row 0: one cell spanning 2 columns.
        assert_eq!(table.rows[0].cells.len(), 1);
        assert_eq!(table.rows[0].cells[0].column_span, 2);
        assert_eq!(table.rows[0].cells[0].row_span, 1);
        // Row 1: two normal cells.
        assert_eq!(table.rows[1].cells.len(), 2);
        assert_eq!(table.rows[1].cells[0].column_span, 1);
        assert_eq!(table.rows[1].cells[1].column_span, 1);
    }

    #[test]
    fn gridspan_horizontal_merge_three_columns() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:tcPr><w:gridSpan w:val="3"/></w:tcPr>
          <w:p><w:r><w:t>Wide</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let easydoc_core::DocumentBlock::Table(table) = &content.blocks[0] else {
            panic!("expected Table")
        };
        assert_eq!(table.rows[0].cells.len(), 1);
        assert_eq!(table.rows[0].cells[0].column_span, 3);
    }

    #[test]
    fn vmerge_restart_sets_row_span_one() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:tcPr><w:vMerge w:val="restart"/></w:tcPr>
          <w:p><w:r><w:t>Start</w:t></w:r></w:p>
        </w:tc>
        <w:tc><w:p><w:r><w:t>Right</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let easydoc_core::DocumentBlock::Table(table) = &content.blocks[0] else {
            panic!("expected Table")
        };
        // restart cell: row_span = 1 (self), column_span = 1 (default).
        assert_eq!(table.rows[0].cells[0].row_span, 1);
        assert_eq!(table.rows[0].cells[0].column_span, 1);
    }

    #[test]
    fn vmerge_continue_sets_row_span_zero() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:tcPr><w:vMerge w:val="restart"/></w:tcPr>
          <w:p><w:r><w:t>Start</w:t></w:r></w:p>
        </w:tc>
        <w:tc><w:p><w:r><w:t>R1</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:tcPr><w:vMerge w:val="continue"/></w:tcPr>
          <w:p/>
        </w:tc>
        <w:tc><w:p><w:r><w:t>R2</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let easydoc_core::DocumentBlock::Table(table) = &content.blocks[0] else {
            panic!("expected Table")
        };
        // Row 0, cell 0: restart => row_span = 1.
        assert_eq!(table.rows[0].cells[0].row_span, 1);
        // Row 1, cell 0: continue => row_span = 0 (merged into cell above).
        assert_eq!(table.rows[1].cells[0].row_span, 0);
    }

    #[test]
    fn vmerge_no_val_treated_as_continue() {
        // OOXML spec: <w:vMerge/> without val is treated as "continue".
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:tcPr><w:vMerge w:val="restart"/></w:tcPr>
          <w:p><w:r><w:t>Top</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:tcPr><w:vMerge/></w:tcPr>
          <w:p/>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:tcPr><w:vMerge/></w:tcPr>
          <w:p/>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let easydoc_core::DocumentBlock::Table(table) = &content.blocks[0] else {
            panic!("expected Table")
        };
        // Row 0: restart => row_span = 1.
        assert_eq!(table.rows[0].cells[0].row_span, 1);
        // Row 1: no val => continue => row_span = 0.
        assert_eq!(table.rows[1].cells[0].row_span, 0);
        // Row 2: no val => continue => row_span = 0.
        assert_eq!(table.rows[2].cells[0].row_span, 0);
    }

    #[test]
    fn mixed_gridspan_and_vmerge() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:tcPr>
            <w:gridSpan w:val="2"/>
            <w:vMerge w:val="restart"/>
          </w:tcPr>
          <w:p><w:r><w:t>Big</w:t></w:r></w:p>
        </w:tc>
        <w:tc><w:p><w:r><w:t>C</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:tcPr>
            <w:gridSpan w:val="2"/>
            <w:vMerge w:val="continue"/>
          </w:tcPr>
          <w:p/>
        </w:tc>
        <w:tc><w:p><w:r><w:t>D</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let easydoc_core::DocumentBlock::Table(table) = &content.blocks[0] else {
            panic!("expected Table")
        };
        // Row 0, cell 0: gridSpan=2 + vMerge=restart => column_span=2, row_span=1.
        assert_eq!(table.rows[0].cells[0].column_span, 2);
        assert_eq!(table.rows[0].cells[0].row_span, 1);
        // Row 1, cell 0: gridSpan=2 + vMerge=continue => column_span=2, row_span=0.
        assert_eq!(table.rows[1].cells[0].column_span, 2);
        assert_eq!(table.rows[1].cells[0].row_span, 0);
    }

    #[test]
    fn gridspan_val_one_is_noop() {
        // gridSpan=1 means no horizontal merge; column_span should remain 1.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:tcPr><w:gridSpan w:val="1"/></w:tcPr>
          <w:p><w:r><w:t>X</w:t></w:r></w:p>
        </w:tc>
        <w:tc><w:p><w:r><w:t>Y</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let mut collector = ContentCollector::new();
        reader.read_events(&mut collector).unwrap();
        let content = collector.into_content();

        let easydoc_core::DocumentBlock::Table(table) = &content.blocks[0] else {
            panic!("expected Table")
        };
        assert_eq!(table.rows[0].cells[0].column_span, 1);
        assert_eq!(table.rows[0].cells[1].column_span, 1);
    }

    // -----------------------------------------------------------------------
    // OMML math tests
    // -----------------------------------------------------------------------

    #[test]
    fn inline_math_in_paragraph() {
        // Single <m:oMath> inside a <w:p> with surrounding text.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>
    <w:p>
      <w:r><w:t>Formula: </w:t></w:r>
      <m:oMath><m:r><m:t>x</m:t></m:r></m:oMath>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        // Expect: Paragraph(["Formula: "]), Math (inline)
        assert_eq!(blocks.len(), 2);
        match &blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs.len(), 1);
                assert_eq!(runs[0].text, "Formula: ");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
        match &blocks[1] {
            DocumentBlock::Math {
                omml,
                latex,
                display,
            } => {
                let xml_str = omml.as_ref().expect("omml should be Some");
                assert!(xml_str.contains("<m:oMath>"), "omml = {xml_str}");
                assert!(xml_str.contains("</m:oMath>"), "omml = {xml_str}");
                assert!(
                    xml_str.contains("<m:r><m:t>x</m:t></m:r>"),
                    "omml = {xml_str}"
                );
                assert!(latex.is_none());
                assert!(!display, "inline math should have display=false");
            }
            other => panic!("expected Math, got {other:?}"),
        }
    }

    #[test]
    fn display_math_with_omathpara() {
        // <m:oMathPara> wrapping an <m:oMath> -- block-level display math.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>
    <w:p><w:r><w:t>Before</w:t></w:r></w:p>
    <m:oMathPara><m:oMath><m:r><m:t>E=mc^2</m:t></m:r></m:oMath></m:oMathPara>
    <w:p><w:r><w:t>After</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        // Expect: Paragraph, Math (display), Paragraph
        assert_eq!(blocks.len(), 3, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::Paragraph(runs) => assert_eq!(runs[0].text, "Before"),
            other => panic!("expected Paragraph, got {other:?}"),
        }
        match &blocks[1] {
            DocumentBlock::Math { omml, display, .. } => {
                let xml_str = omml.as_ref().expect("omml should be Some");
                assert!(xml_str.contains("<m:oMathPara>"), "omml = {xml_str}");
                assert!(xml_str.contains("</m:oMathPara>"), "omml = {xml_str}");
                assert!(*display, "oMathPara should have display=true");
            }
            other => panic!("expected Math, got {other:?}"),
        }
        match &blocks[2] {
            DocumentBlock::Paragraph(runs) => assert_eq!(runs[0].text, "After"),
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn mixed_text_math_text_in_paragraph() {
        // Text before, math in the middle, text after -- all in one <w:p>.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>
    <w:p>
      <w:r><w:t>Let </w:t></w:r>
      <m:oMath><m:r><m:t>y</m:t></m:r></m:oMath>
      <w:r><w:t> be the result.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        // Expect: Paragraph(["Let "]), Math, Paragraph([" be the result."])
        assert_eq!(blocks.len(), 3, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::Paragraph(runs) => assert_eq!(runs[0].text, "Let "),
            other => panic!("expected Paragraph, got {other:?}"),
        }
        assert!(matches!(
            &blocks[1],
            DocumentBlock::Math { display: false, .. }
        ));
        match &blocks[2] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs[0].text, " be the result.");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn nested_math_structure() {
        // <m:oMath> with nested <m:f> (fraction) structure.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>
    <m:oMath>
      <m:f>
        <m:num><m:r><m:t>a</m:t></m:r></m:num>
        <m:den><m:r><m:t>b</m:t></m:r></m:den>
      </m:f>
    </m:oMath>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::Math { omml, display, .. } => {
                let xml_str = omml.as_ref().expect("omml should be Some");
                // Verify the nested structure is preserved in the XML.
                assert!(xml_str.contains("<m:f>"), "omml = {xml_str}");
                assert!(xml_str.contains("</m:f>"), "omml = {xml_str}");
                assert!(xml_str.contains("<m:num>"), "omml = {xml_str}");
                assert!(xml_str.contains("<m:den>"), "omml = {xml_str}");
                assert!(xml_str.contains("<m:t>a</m:t>"), "omml = {xml_str}");
                assert!(xml_str.contains("<m:t>b</m:t>"), "omml = {xml_str}");
                assert!(!display, "standalone oMath should have display=false");
            }
            other => panic!("expected Math, got {other:?}"),
        }
    }

    #[test]
    fn block_level_math_without_omathpara() {
        // <m:oMath> directly in <w:body> (no wrapping <m:oMathPara>).
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>
    <w:p><w:r><w:t>See equation:</w:t></w:r></w:p>
    <m:oMath><m:r><m:t>x+1=0</m:t></m:r></m:oMath>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 2, "blocks = {blocks:?}");
        assert!(matches!(&blocks[0], DocumentBlock::Paragraph(_)));
        match &blocks[1] {
            DocumentBlock::Math { omml, display, .. } => {
                let xml_str = omml.as_ref().expect("omml should be Some");
                assert!(xml_str.contains("<m:oMath>"));
                assert!(
                    !display,
                    "bare oMathPara-less math should have display=false"
                );
            }
            other => panic!("expected Math, got {other:?}"),
        }
    }

    #[test]
    fn multiple_math_in_one_paragraph() {
        // Two <m:oMath> formulas in a single paragraph.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
  <w:body>
    <w:p>
      <m:oMath><m:r><m:t>a</m:t></m:r></m:oMath>
      <w:r><w:t> + </w:t></w:r>
      <m:oMath><m:r><m:t>b</m:t></m:r></m:oMath>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        // Expect: Math("a"), Paragraph([" + "]), Math("b")
        assert_eq!(blocks.len(), 3, "blocks = {blocks:?}");
        assert!(matches!(
            &blocks[0],
            DocumentBlock::Math { display: false, .. }
        ));
        match &blocks[1] {
            DocumentBlock::Paragraph(runs) => assert_eq!(runs[0].text, " + "),
            other => panic!("expected Paragraph, got {other:?}"),
        }
        assert!(matches!(
            &blocks[2],
            DocumentBlock::Math { display: false, .. }
        ));
    }

    // -----------------------------------------------------------------------
    // List detection tests (<w:numPr>)
    // -----------------------------------------------------------------------

    #[test]
    fn single_list_item() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Item one</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert_eq!(list.items.len(), 1);
                match &list.items[0].blocks[0] {
                    DocumentBlock::Paragraph(runs) => {
                        assert_eq!(runs[0].text, "Item one");
                    }
                    other => panic!("expected Paragraph inside list item, got {other:?}"),
                }
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn consecutive_list_items_merged() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>First</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Second</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Third</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        // All three list items should be merged into a single List block.
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert_eq!(list.items.len(), 3);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn list_followed_by_paragraph_flushes() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>List item</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Normal paragraph</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 2, "blocks = {blocks:?}");
        assert!(matches!(&blocks[0], DocumentBlock::List(_)));
        match &blocks[1] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs[0].text, "Normal paragraph");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn two_separate_lists() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>A</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Separator</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="2"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>B</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 3, "blocks = {blocks:?}");
        assert!(matches!(&blocks[0], DocumentBlock::List(_)));
        assert!(matches!(&blocks[1], DocumentBlock::Paragraph(_)));
        assert!(matches!(&blocks[2], DocumentBlock::List(_)));
    }

    #[test]
    fn list_at_document_end_flushes() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Last item</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        assert!(matches!(&blocks[0], DocumentBlock::List(_)));
    }

    // -----------------------------------------------------------------------
    // Hyperlink parsing tests (<w:hyperlink>)
    // -----------------------------------------------------------------------

    #[test]
    fn hyperlink_sets_run_field() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p>
      <w:hyperlink r:id="rId5">
        <w:r><w:t>Click here</w:t></w:r>
      </w:hyperlink>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs.len(), 1);
                assert_eq!(runs[0].text, "Click here");
                assert_eq!(runs[0].hyperlink.as_deref(), Some("rId5"));
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn hyperlink_mixed_with_normal_runs() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p>
      <w:r><w:t>Normal </w:t></w:r>
      <w:hyperlink r:id="rId3">
        <w:r><w:t>link text</w:t></w:r>
      </w:hyperlink>
      <w:r><w:t> after</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs.len(), 3);
                assert_eq!(runs[0].text, "Normal ");
                assert!(runs[0].hyperlink.is_none());
                assert_eq!(runs[1].text, "link text");
                assert_eq!(runs[1].hyperlink.as_deref(), Some("rId3"));
                assert_eq!(runs[2].text, " after");
                assert!(runs[2].hyperlink.is_none());
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn hyperlink_with_bold_run() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p>
      <w:hyperlink r:id="rId10">
        <w:r>
          <w:rPr><w:b/></w:rPr>
          <w:t>Bold link</w:t>
        </w:r>
      </w:hyperlink>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        match &blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs[0].text, "Bold link");
                assert!(runs[0].bold);
                assert_eq!(runs[0].hyperlink.as_deref(), Some("rId10"));
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn no_hyperlink_field_when_not_in_hyperlink() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>No link</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        match &blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                assert!(runs[0].hyperlink.is_none());
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Nested table tests (<w:tbl> inside <w:tc>)
    // -----------------------------------------------------------------------

    #[test]
    fn nested_table_in_cell() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:p><w:r><w:t>Outer</w:t></w:r></w:p>
          <w:tbl>
            <w:tr>
              <w:tc><w:p><w:r><w:t>Inner A</w:t></w:r></w:p></w:tc>
              <w:tc><w:p><w:r><w:t>Inner B</w:t></w:r></w:p></w:tc>
            </w:tr>
          </w:tbl>
        </w:tc>
        <w:tc><w:p><w:r><w:t>Right</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::Table(outer) => {
                assert_eq!(outer.rows.len(), 1);
                assert_eq!(outer.rows[0].cells.len(), 2);
                // First cell should have: Paragraph("Outer") + Table(inner)
                let cell0 = &outer.rows[0].cells[0];
                assert!(cell0.blocks.len() >= 2, "cell0.blocks = {:?}", cell0.blocks);
                assert!(matches!(&cell0.blocks[0], DocumentBlock::Paragraph(_)));
                assert!(matches!(&cell0.blocks[1], DocumentBlock::Table(_)));
                if let DocumentBlock::Table(inner) = &cell0.blocks[1] {
                    assert_eq!(inner.rows.len(), 1);
                    assert_eq!(inner.rows[0].cells.len(), 2);
                }
                // Second cell is normal.
                let cell1 = &outer.rows[0].cells[1];
                assert_eq!(cell1.blocks.len(), 1);
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn nested_table_only_in_cell() {
        // Cell has only a nested table, no text before it.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:tbl>
            <w:tr>
              <w:tc><w:p><w:r><w:t>Deep</w:t></w:r></w:p></w:tc>
            </w:tr>
          </w:tbl>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::Table(outer) => {
                let cell0 = &outer.rows[0].cells[0];
                assert_eq!(cell0.blocks.len(), 1);
                assert!(matches!(&cell0.blocks[0], DocumentBlock::Table(_)));
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn flat_table_still_works() {
        // Regression: make sure normal tables without nesting still work.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc><w:p><w:r><w:t>X</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>Y</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            DocumentBlock::Table(table) => {
                assert_eq!(table.rows.len(), 1);
                assert_eq!(table.rows[0].cells.len(), 2);
                // Cells should have text as Paragraph blocks.
                let cell_text = |cell: &easydoc_core::DocumentTableCell| -> String {
                    cell.blocks
                        .iter()
                        .filter_map(|b| match b {
                            DocumentBlock::Paragraph(runs) => {
                                Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                            }
                            _ => None,
                        })
                        .collect()
                };
                assert_eq!(cell_text(&table.rows[0].cells[0]), "X");
                assert_eq!(cell_text(&table.rows[0].cells[1]), "Y");
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Combined feature tests
    // -----------------------------------------------------------------------

    #[test]
    fn list_then_hyperlink_then_table() {
        // All three features in one document.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Item</w:t></w:r>
    </w:p>
    <w:p>
      <w:hyperlink r:id="rId7">
        <w:r><w:t>Link</w:t></w:r>
      </w:hyperlink>
    </w:p>
    <w:tbl>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 3, "blocks = {blocks:?}");
        assert!(matches!(&blocks[0], DocumentBlock::List(_)));
        match &blocks[1] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs[0].hyperlink.as_deref(), Some("rId7"));
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
        assert!(matches!(&blocks[2], DocumentBlock::Table(_)));
    }

    // -----------------------------------------------------------------------
    // End-to-end: numbering integration
    // -----------------------------------------------------------------------

    /// Builds a DOCX ZIP containing `word/document.xml` and `word/numbering.xml`.
    fn make_docx_with_numbering(doc_xml: &[u8], numbering_xml: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let w = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(w);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            zip.start_file("word/document.xml", options).unwrap();
            zip.write_all(doc_xml).unwrap();

            zip.start_file("word/numbering.xml", options).unwrap();
            zip.write_all(numbering_xml).unwrap();

            zip.finish().unwrap();
        }
        buf
    }

    /// Builds a DOCX ZIP with document.xml, rels, and numbering.xml.
    fn make_docx_with_rels_and_numbering(
        doc_xml: &[u8],
        rels_xml: &[u8],
        numbering_xml: &[u8],
    ) -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let w = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(w);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            zip.start_file("word/document.xml", options).unwrap();
            zip.write_all(doc_xml).unwrap();

            zip.start_file("word/_rels/document.xml.rels", options)
                .unwrap();
            zip.write_all(rels_xml).unwrap();

            zip.start_file("word/numbering.xml", options).unwrap();
            zip.write_all(numbering_xml).unwrap();

            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn e2e_ordered_list_from_numbering_xml() {
        let doc_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>First</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Second</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Third</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let numbering_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

        let zip_data = make_docx_with_numbering(doc_xml, numbering_xml);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut &tmp, &zip_data).unwrap();
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let blocks = reader.read_blocks().unwrap();

        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert!(list.ordered, "list should be ordered (decimal fmt)");
                assert_eq!(list.start_number, Some(1));
                assert_eq!(list.items.len(), 3);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn e2e_bullet_list_remains_unordered() {
        let doc_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Bullet A</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Bullet B</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let numbering_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="&#x2022;"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

        let zip_data = make_docx_with_numbering(doc_xml, numbering_xml);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut &tmp, &zip_data).unwrap();
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let blocks = reader.read_blocks().unwrap();

        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert!(!list.ordered, "bullet list should be unordered");
                assert_eq!(list.start_number, None);
                assert_eq!(list.items.len(), 2);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn e2e_numbering_missing_numid_falls_back_to_unordered() {
        // numId="99" does not exist in numbering.xml => fallback to unordered.
        let doc_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="99"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Unknown numId</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let numbering_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

        let zip_data = make_docx_with_numbering(doc_xml, numbering_xml);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut &tmp, &zip_data).unwrap();
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let blocks = reader.read_blocks().unwrap();

        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert!(!list.ordered, "unknown numId should fallback to unordered");
                assert_eq!(list.start_number, None);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn e2e_no_numbering_xml_falls_back_to_unordered() {
        // No numbering.xml in the ZIP at all => fallback to unordered.
        let doc_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Item</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        // Use the basic make_docx_xml helper (no numbering.xml).
        let zip_data = make_docx_xml(doc_xml);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut &tmp, &zip_data).unwrap();
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let blocks = reader.read_blocks().unwrap();

        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert!(!list.ordered, "no numbering.xml => unordered fallback");
                assert_eq!(list.start_number, None);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn e2e_ordered_list_with_start_value() {
        let doc_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="2"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Item five</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="2"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Item six</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let numbering_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="5"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="2">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

        let zip_data = make_docx_with_numbering(doc_xml, numbering_xml);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut &tmp, &zip_data).unwrap();
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let blocks = reader.read_blocks().unwrap();

        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert!(list.ordered);
                assert_eq!(list.start_number, Some(5));
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // End-to-end: hyperlink resolution via relationships
    // -----------------------------------------------------------------------

    #[test]
    fn e2e_hyperlink_resolves_to_url() {
        let doc_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p>
      <w:hyperlink r:id="rId10">
        <w:r><w:t>Visit example</w:t></w:r>
      </w:hyperlink>
    </w:p>
  </w:body>
</w:document>"#;

        let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId10" Target="https://example.com"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"
                TargetMode="External"/>
</Relationships>"#;

        let zip_data = make_docx_with_image(doc_xml, rels_xml, &[]);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut &tmp, &zip_data).unwrap();
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let blocks = reader.read_blocks().unwrap();

        match &blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs.len(), 1);
                assert_eq!(runs[0].text, "Visit example");
                assert_eq!(
                    runs[0].hyperlink.as_deref(),
                    Some("https://example.com"),
                    "hyperlink should be resolved to URL, not raw rId"
                );
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn e2e_hyperlink_fallback_to_rid_when_no_rels() {
        // No rels file => hyperlink stays as raw rId.
        let doc_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p>
      <w:hyperlink r:id="rId5">
        <w:r><w:t>No rels</w:t></w:r>
      </w:hyperlink>
    </w:p>
  </w:body>
</w:document>"#;

        let zip_data = make_docx_xml(doc_xml);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut &tmp, &zip_data).unwrap();
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let blocks = reader.read_blocks().unwrap();

        match &blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs[0].hyperlink.as_deref(), Some("rId5"));
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    #[test]
    fn e2e_hyperlink_in_list_items_resolves() {
        // List items containing hyperlinks should resolve both numbering and
        // hyperlink relationships.
        let doc_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>See </w:t></w:r>
      <w:hyperlink r:id="rId20">
        <w:r><w:t>Rust lang</w:t></w:r>
      </w:hyperlink>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Plain item</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId20" Target="https://rust-lang.org"
                Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"
                TargetMode="External"/>
</Relationships>"#;

        let numbering_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

        let zip_data = make_docx_with_rels_and_numbering(doc_xml, rels_xml, numbering_xml);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut &tmp, &zip_data).unwrap();
        let mut reader = DocxSaxReader::from_path(tmp.path()).unwrap();
        let blocks = reader.read_blocks().unwrap();

        // Should be a single List block.
        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert!(list.ordered, "should be ordered (decimal)");
                assert_eq!(list.items.len(), 2);

                // First item: Paragraph with two runs, second run is hyperlink.
                match &list.items[0].blocks[0] {
                    DocumentBlock::Paragraph(runs) => {
                        assert_eq!(runs.len(), 2);
                        assert_eq!(runs[0].text, "See ");
                        assert!(runs[0].hyperlink.is_none());
                        assert_eq!(runs[1].text, "Rust lang");
                        assert_eq!(runs[1].hyperlink.as_deref(), Some("https://rust-lang.org"),);
                    }
                    other => panic!("expected Paragraph, got {other:?}"),
                }

                // Second item: plain text, no hyperlink.
                match &list.items[1].blocks[0] {
                    DocumentBlock::Paragraph(runs) => {
                        assert_eq!(runs[0].text, "Plain item");
                        assert!(runs[0].hyperlink.is_none());
                    }
                    other => panic!("expected Paragraph, got {other:?}"),
                }
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Nested list tests (ilvl-based nesting)
    // -----------------------------------------------------------------------

    #[test]
    fn two_level_list_nests_ilvl_1_in_ilvl_0() {
        // ilvl 0, 0, 1 => top-level items=2, first item has nested with 1 item.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Top A</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Top B</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Nested under B</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::List(list) => {
                // Two top-level items: "Top A" and "Top B".
                assert_eq!(list.items.len(), 2, "items = {:?}", list.items);

                // First item: "Top A", no nested.
                assert!(list.items[0].nested.is_none());

                // Second item: "Top B", has nested with 1 item.
                let nested = list.items[1]
                    .nested
                    .as_ref()
                    .expect("Top B should have nested list");
                assert_eq!(nested.items.len(), 1);
                match &nested.items[0].blocks[0] {
                    DocumentBlock::Paragraph(runs) => {
                        assert_eq!(runs[0].text, "Nested under B");
                    }
                    other => panic!("expected Paragraph, got {other:?}"),
                }
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn three_level_list_nests_correctly() {
        // ilvl 0, 1, 2, 0 => top-level items=2, first has nested chain 0->1->2.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Level 0</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Level 1</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="2"/></w:numPr></w:pPr>
      <w:r><w:t>Level 2</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Second top</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::List(list) => {
                // Two top-level items.
                assert_eq!(list.items.len(), 2);

                // First top-level item: "Level 0".
                let item0 = &list.items[0];
                let nested1 = item0.nested.as_ref().expect("Level 0 should have nested");
                assert_eq!(nested1.items.len(), 1);

                // Nested level 1: "Level 1".
                let nested2 = nested1.items[0]
                    .nested
                    .as_ref()
                    .expect("Level 1 should have nested");
                assert_eq!(nested2.items.len(), 1);

                // Nested level 2: "Level 2".
                match &nested2.items[0].blocks[0] {
                    DocumentBlock::Paragraph(runs) => {
                        assert_eq!(runs[0].text, "Level 2");
                    }
                    other => panic!("expected Paragraph, got {other:?}"),
                }

                // Second top-level item: "Second top", no nesting.
                assert!(list.items[1].nested.is_none());
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn flat_list_with_multiple_ilvl_0() {
        // 5 items all at ilvl=0 => top-level items=5, none nested.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>A</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>B</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>C</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>D</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>E</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert_eq!(list.items.len(), 5);
                for item in &list.items {
                    assert!(item.nested.is_none(), "flat items should not have nested");
                }
                // Verify text ordering.
                let texts: Vec<&str> = list
                    .items
                    .iter()
                    .map(|item| match &item.blocks[0] {
                        DocumentBlock::Paragraph(runs) => runs[0].text.as_str(),
                        _ => panic!("expected Paragraph"),
                    })
                    .collect();
                assert_eq!(texts, vec!["A", "B", "C", "D", "E"]);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn list_breaks_at_non_list_paragraph() {
        // ilvl 0 + ilvl 1 + normal paragraph + ilvl 0 => two separate lists.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>List 1 top</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>List 1 nested</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Separator paragraph</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>List 2 top</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        // Expect: List, Paragraph, List.
        assert_eq!(blocks.len(), 3, "blocks = {blocks:?}");

        // First list: 1 top-level item with nested.
        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert_eq!(list.items.len(), 1);
                let nested = list.items[0]
                    .nested
                    .as_ref()
                    .expect("first list item should have nested");
                assert_eq!(nested.items.len(), 1);
            }
            other => panic!("expected List, got {other:?}"),
        }

        // Separator paragraph.
        match &blocks[1] {
            DocumentBlock::Paragraph(runs) => {
                assert_eq!(runs[0].text, "Separator paragraph");
            }
            other => panic!("expected Paragraph, got {other:?}"),
        }

        // Second list: 1 top-level item, no nested.
        match &blocks[2] {
            DocumentBlock::List(list) => {
                assert_eq!(list.items.len(), 1);
                assert!(list.items[0].nested.is_none());
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn ilvl_decrease_creates_separate_branch() {
        // ilvl 0, 1, 0 => top-level items=2, first has nested with 1 item.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Branch A</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Branch A child</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Branch B</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert_eq!(list.items.len(), 2);

                // First branch: "Branch A" with nested "Branch A child".
                let nested = list.items[0]
                    .nested
                    .as_ref()
                    .expect("Branch A should have nested");
                assert_eq!(nested.items.len(), 1);
                match &nested.items[0].blocks[0] {
                    DocumentBlock::Paragraph(runs) => {
                        assert_eq!(runs[0].text, "Branch A child");
                    }
                    other => panic!("expected Paragraph, got {other:?}"),
                }

                // Second branch: "Branch B", no nested.
                assert!(list.items[1].nested.is_none());
                match &list.items[1].blocks[0] {
                    DocumentBlock::Paragraph(runs) => {
                        assert_eq!(runs[0].text, "Branch B");
                    }
                    other => panic!("expected Paragraph, got {other:?}"),
                }
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn multiple_siblings_at_nested_level() {
        // ilvl 0, 1, 1, 0 => first top-level has 2 nested children.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Parent</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Child 1</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Child 2</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr>
      <w:r><w:t>Sibling</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
        let mut reader = DocxSaxReader::from_reader(&xml[..]);
        let blocks = reader.read_blocks().unwrap();
        assert_eq!(blocks.len(), 1, "blocks = {blocks:?}");
        match &blocks[0] {
            DocumentBlock::List(list) => {
                assert_eq!(list.items.len(), 2);

                // "Parent" has 2 nested children.
                let nested = list.items[0]
                    .nested
                    .as_ref()
                    .expect("Parent should have nested");
                assert_eq!(nested.items.len(), 2);

                match &nested.items[0].blocks[0] {
                    DocumentBlock::Paragraph(runs) => assert_eq!(runs[0].text, "Child 1"),
                    other => panic!("expected Paragraph, got {other:?}"),
                }
                match &nested.items[1].blocks[0] {
                    DocumentBlock::Paragraph(runs) => assert_eq!(runs[0].text, "Child 2"),
                    other => panic!("expected Paragraph, got {other:?}"),
                }

                // "Sibling" has no nested.
                assert!(list.items[1].nested.is_none());
            }
            other => panic!("expected List, got {other:?}"),
        }
    }
}
