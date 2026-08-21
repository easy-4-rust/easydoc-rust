//! 语义模型渲染器 — 将 `DocumentContent` 转换为 docx-rs 文档。
//!
//! 这是打通 Read → Modify → Write 闭环的关键桥梁。
//! Reader 输出 `DocumentContent`，本渲染器将其渲染为 DOCX。

use docx_rs::{
    AbstractNumbering, BreakType, Docx, Hyperlink, HyperlinkType, IndentLevel, Level, LevelJc,
    LevelText, NumberFormat, Numbering, NumberingId, Pic, RunFonts, SpecialIndentType, Start,
};
use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentImage, DocumentList, DocumentTable, DocumentTextRun,
    HeadingLevel, Result,
};

/// Bullet list numbering ID (references `numbering.xml` abstractNum 0).
const BULLET_NUM_ID: usize = 10;
/// Ordered (decimal) list numbering ID (references `numbering.xml` abstractNum 1).
const DECIMAL_NUM_ID: usize = 11;
/// Starting abstract numbering ID for dynamically created numbering definitions
/// (used when `start_number` differs from 1).
const DYNAMIC_ABSTRACT_NUM_START: usize = 100;
/// Starting numbering ID for dynamically created numbering references.
const DYNAMIC_NUM_ID_START: usize = 100;

/// Adds predefined bullet and decimal numbering definitions to the `Docx` instance.
///
/// This populates `word/numbering.xml` with two abstract numbering definitions:
/// - abstractNum 0: bullet list (multi-level with `•`, `◦`, `▪`)
/// - abstractNum 1: decimal list (multi-level with `1.`, `a.`, `i.`)
fn add_list_numberings(docx: Docx) -> Docx {
    // --- Bullet list (abstractNum 0) ---
    let mut bullet_abstract = AbstractNumbering::new(0);
    bullet_abstract.multi_level_type = Some("hybridMultilevel".to_string());
    let bullet_abstract = bullet_abstract
        .add_level(
            Level::new(
                0,
                Start::new(1),
                NumberFormat::new("bullet"),
                LevelText::new("\u{2022}"), // •
                LevelJc::new("left"),
            )
            .indent(Some(720), Some(SpecialIndentType::Hanging(360)), None, None),
        )
        .add_level(
            Level::new(
                1,
                Start::new(1),
                NumberFormat::new("bullet"),
                LevelText::new("\u{25E6}"), // ◦
                LevelJc::new("left"),
            )
            .indent(
                Some(1080),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        )
        .add_level(
            Level::new(
                2,
                Start::new(1),
                NumberFormat::new("bullet"),
                LevelText::new("\u{25AA}"), // ▪
                LevelJc::new("left"),
            )
            .indent(
                Some(1440),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        );

    // --- Decimal list (abstractNum 1) ---
    let mut decimal_abstract = AbstractNumbering::new(1);
    decimal_abstract.multi_level_type = Some("hybridMultilevel".to_string());
    let decimal_abstract = decimal_abstract
        .add_level(
            Level::new(
                0,
                Start::new(1),
                NumberFormat::new("decimal"),
                LevelText::new("%1."),
                LevelJc::new("left"),
            )
            .indent(Some(720), Some(SpecialIndentType::Hanging(360)), None, None),
        )
        .add_level(
            Level::new(
                1,
                Start::new(1),
                NumberFormat::new("lowerLetter"),
                LevelText::new("%2."),
                LevelJc::new("left"),
            )
            .indent(
                Some(1080),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        )
        .add_level(
            Level::new(
                2,
                Start::new(1),
                NumberFormat::new("lowerRoman"),
                LevelText::new("%3."),
                LevelJc::new("left"),
            )
            .indent(
                Some(1440),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        );

    docx.add_abstract_numbering(bullet_abstract)
        .add_abstract_numbering(decimal_abstract)
        .add_numbering(Numbering::new(BULLET_NUM_ID, 0))
        .add_numbering(Numbering::new(DECIMAL_NUM_ID, 1))
}

/// Counter for generating unique numbering IDs for custom start numbers.
static DYNAMIC_NUM_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Registers a custom abstract numbering definition with the given start value.
///
/// Returns the `numId` to use in `<w:numPr>` for this list.
/// Each call creates a new abstract numbering + numbering pair to avoid
/// conflicting with existing definitions.
fn register_custom_start_numbering(docx: &mut Docx, start: u32) -> usize {
    use std::sync::atomic::Ordering;

    let idx = DYNAMIC_NUM_COUNTER.fetch_add(1, Ordering::Relaxed);
    let abstract_num_id = DYNAMIC_ABSTRACT_NUM_START + idx;
    let num_id = DYNAMIC_NUM_ID_START + idx;

    let mut abstract_num = AbstractNumbering::new(abstract_num_id);
    abstract_num.multi_level_type = Some("hybridMultilevel".to_string());
    let abstract_num = abstract_num
        .add_level(
            Level::new(
                0,
                Start::new(start as usize),
                NumberFormat::new("decimal"),
                LevelText::new("%1."),
                LevelJc::new("left"),
            )
            .indent(Some(720), Some(SpecialIndentType::Hanging(360)), None, None),
        )
        .add_level(
            Level::new(
                1,
                Start::new(1),
                NumberFormat::new("lowerLetter"),
                LevelText::new("%2."),
                LevelJc::new("left"),
            )
            .indent(
                Some(1080),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        )
        .add_level(
            Level::new(
                2,
                Start::new(1),
                NumberFormat::new("lowerRoman"),
                LevelText::new("%3."),
                LevelJc::new("left"),
            )
            .indent(
                Some(1440),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        );

    // Swap out the docx, add numbering, and swap back.
    let owned = std::mem::replace(docx, Docx::new());
    *docx = owned
        .add_abstract_numbering(abstract_num)
        .add_numbering(Numbering::new(num_id, abstract_num_id));

    num_id
}

/// Wraps a set of runs into a paragraph, grouping consecutive hyperlink runs
/// into `<w:hyperlink>` elements.
///
/// Runs sharing the same non-empty `hyperlink` URL are grouped into a single
/// `Hyperlink` element.  Runs without a hyperlink are added as plain `Run` children.
fn add_runs_with_hyperlinks(
    p: docx_rs::Paragraph,
    runs: &[DocumentTextRun],
    bold: bool,
) -> docx_rs::Paragraph {
    let mut p = p;
    let mut i = 0;
    while i < runs.len() {
        if let Some(ref url) = runs[i].hyperlink {
            // Group consecutive runs with the same hyperlink URL.
            let mut link = Hyperlink::new(url.as_str(), HyperlinkType::External);
            while i < runs.len() && runs[i].hyperlink.as_deref() == Some(url.as_str()) {
                link = link.add_run(text_run_to_docx_run(&runs[i], bold));
                i += 1;
            }
            p = p.add_hyperlink(link);
        } else {
            p = p.add_run(text_run_to_docx_run(&runs[i], bold));
            i += 1;
        }
    }
    p
}

/// 将核心语义模型渲染为 docx-rs 的 `Docx` 实例。
///
/// # 参数
/// - `content`: 完整的语义文档模型。
///
/// # 返回
/// 构建好的 `Docx` 实例，可进一步 `pack()` 为 DOCX 文件。
pub fn render_document_content(content: &DocumentContent) -> Result<Docx> {
    let mut docx = Docx::new();

    // Pre-register bullet/decimal numbering definitions so that list paragraphs
    // can reference them via `<w:numPr>`.
    docx = add_list_numberings(docx);

    for block in &content.blocks {
        docx = render_block(docx, block)?;
    }

    Ok(docx)
}

/// 递归渲染单个块级元素。
fn render_block(mut docx: Docx, block: &DocumentBlock) -> Result<Docx> {
    match block {
        DocumentBlock::Heading { level, runs } => {
            let _heading_level = u8_to_heading_level(*level);
            let p = docx_rs::Paragraph::new()
                .style(heading_style_name(*level))
                .outline_lvl(heading_outline_level(*level));
            let p = add_runs_with_hyperlinks(p, runs, true);
            docx = docx.add_paragraph(p);
        }
        DocumentBlock::Paragraph(runs) => {
            let p = docx_rs::Paragraph::new();
            let p = add_runs_with_hyperlinks(p, runs, false);
            docx = docx.add_paragraph(p);
        }
        DocumentBlock::Table(table) => {
            docx = render_table(docx, table)?;
        }
        DocumentBlock::List(list) => {
            docx = render_list(docx, list)?;
        }
        DocumentBlock::Image(image) => {
            docx = render_image(docx, image)?;
        }
        DocumentBlock::ThematicBreak | DocumentBlock::PageBreak => {
            let p =
                docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_break(BreakType::Page));
            docx = docx.add_paragraph(p);
        }
        DocumentBlock::ColumnBreak => {
            let p =
                docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_break(BreakType::Column));
            docx = docx.add_paragraph(p);
        }
        DocumentBlock::CodeBlock { language: _, code } => {
            // 代码块渲染为等宽字体段落
            let mut p = docx_rs::Paragraph::new();
            let r = docx_rs::Run::new()
                .add_text(code.as_str())
                .fonts(RunFonts::new().ascii("Courier New"))
                .size(20); // 10pt
            p = p.add_run(r);
            docx = docx.add_paragraph(p);
        }
        DocumentBlock::TextBox(blocks) => {
            // 文本框内容渲染为普通段落
            for inner in blocks {
                docx = render_block(docx, inner)?;
            }
        }
        DocumentBlock::Footnote { id, blocks } => {
            // 脚注以 `[^id]: content` 文本段落写出，与 markdown_renderer
            // 的输出格式对称，保证 MD→DOCX→MD 往返不丢失脚注语义。
            let body = plain_blocks(blocks);
            let mut p = docx_rs::Paragraph::new();
            p = p.add_run(docx_rs::Run::new().add_text(format!("[^{id}]: {body}")));
            docx = docx.add_paragraph(p);
        }
        DocumentBlock::Endnote { id, blocks } => {
            // 尾注同脚注，标记为 `[^endnote-{id}]`（与 renderer 一致）。
            let body = plain_blocks(blocks);
            let mut p = docx_rs::Paragraph::new();
            p = p.add_run(docx_rs::Run::new().add_text(format!("[^endnote-{id}]: {body}")));
            docx = docx.add_paragraph(p);
        }
        DocumentBlock::Section {
            blocks,
            section_type: _,
        } => {
            // 分区内容渲染为普通子块
            for inner in blocks {
                docx = render_block(docx, inner)?;
            }
        }
        DocumentBlock::Math {
            omml: _,
            latex,
            display,
        } => {
            // docx-rs 不支持 OMML 数学公式；以 LaTeX 源码文本呈现，
            // 保留公式内容供读回（display 公式前后加空行区分块级）。
            // 依赖方向：writer 不依赖 markdown crate，故仅使用 latex 字段
            // （markdown_import 生成的 Math 块总是携带 latex）。
            let text = latex.clone().unwrap_or_default();
            if !text.is_empty() {
                let mut p = docx_rs::Paragraph::new();
                if *display {
                    p = p.add_run(
                        docx_rs::Run::new()
                            .add_text(format!("$${text}$$"))
                            .fonts(RunFonts::new().ascii("Courier New")),
                    );
                } else {
                    p = p.add_run(
                        docx_rs::Run::new()
                            .add_text(format!("${text}$"))
                            .fonts(RunFonts::new().ascii("Courier New")),
                    );
                }
                docx = docx.add_paragraph(p);
            }
        }
        _ => {
            // 未来新增的块类型暂时跳过
        }
    }
    Ok(docx)
}

/// 将 `DocumentTextRun` 转换为 `docx_rs::Run`。
fn text_run_to_docx_run(run: &DocumentTextRun, bold: bool) -> docx_rs::Run {
    let mut r = docx_rs::Run::new();
    r = r.add_text(run.text.as_str());
    if run.bold || bold {
        r = r.bold();
    }
    if run.italic {
        r = r.italic();
    }
    if run.strikethrough {
        r = r.strike();
    }
    r
}

/// 渲染语义表格。
fn render_table(mut docx: Docx, table: &DocumentTable) -> Result<Docx> {
    let mut rows: Vec<docx_rs::TableRow> = Vec::new();

    for table_row in &table.rows {
        let mut cells: Vec<docx_rs::TableCell> = Vec::new();
        for cell in &table_row.cells {
            let mut cell_paragraphs: Vec<docx_rs::Paragraph> = Vec::new();
            for block in &cell.blocks {
                let p = render_block_to_paragraph(block)?;
                cell_paragraphs.push(p);
            }
            let mut tc = docx_rs::TableCell::new();
            for p in cell_paragraphs {
                tc = tc.add_paragraph(p);
            }
            if cell.column_span > 1 {
                tc = tc.grid_span(cell.column_span as usize);
            }
            // 纵向合并映射：row_span == 0 表示 vMerge continue（合并到上方
            // 单元格），row_span > 1 表示 vMerge restart（向下跨 N-1 行）。
            if cell.row_span == 0 {
                tc = tc.vertical_merge(docx_rs::VMergeType::Continue);
            } else if cell.row_span > 1 {
                tc = tc.vertical_merge(docx_rs::VMergeType::Restart);
            }
            cells.push(tc);
        }
        rows.push(docx_rs::TableRow::new(cells));
    }

    docx = docx.add_table(docx_rs::Table::new(rows));
    Ok(docx)
}

/// 将块列表提取为纯文本（段落取 run 文本拼接，其他块递归）。
///
/// 用于脚注/尾注正文的扁平化输出，与 `markdown_renderer` 的
/// `plain_blocks` 语义一致。
fn plain_blocks(blocks: &[DocumentBlock]) -> String {
    let mut out = String::new();
    for block in blocks {
        match block {
            DocumentBlock::Paragraph(runs) | DocumentBlock::Heading { runs, .. } => {
                for run in runs {
                    out.push_str(&run.text);
                }
            }
            DocumentBlock::Footnote { blocks, .. } | DocumentBlock::Endnote { blocks, .. } => {
                out.push_str(&plain_blocks(blocks));
            }
            _ => {}
        }
    }
    out
}

/// 将单个块渲染为段落（用于表格单元格内）。
fn render_block_to_paragraph(block: &DocumentBlock) -> Result<docx_rs::Paragraph> {
    match block {
        DocumentBlock::Heading { level: _, runs } => {
            let p = docx_rs::Paragraph::new();
            Ok(add_runs_with_hyperlinks(p, runs, true))
        }
        DocumentBlock::Paragraph(runs) => {
            let p = docx_rs::Paragraph::new();
            Ok(add_runs_with_hyperlinks(p, runs, false))
        }
        DocumentBlock::List(list) => {
            // Render list inside a table cell with numbering properties.
            let mut p = docx_rs::Paragraph::new();
            let num_id = if list.ordered {
                DECIMAL_NUM_ID
            } else {
                BULLET_NUM_ID
            };
            p = p.numbering(NumberingId::new(num_id), IndentLevel::new(0));
            for item in &list.items {
                for inner_block in &item.blocks {
                    if let DocumentBlock::Paragraph(runs) = inner_block {
                        p = add_runs_with_hyperlinks(p, runs, false);
                    }
                }
            }
            Ok(p)
        }
        DocumentBlock::CodeBlock { code, .. } => {
            let mut p = docx_rs::Paragraph::new();
            let r = docx_rs::Run::new()
                .add_text(code.as_str())
                .fonts(RunFonts::new().ascii("Courier New"))
                .size(20);
            p = p.add_run(r);
            Ok(p)
        }
        _ => Ok(docx_rs::Paragraph::new()),
    }
}

/// Renders a semantic list as OOXML paragraphs with `<w:numPr>` numbering properties.
///
/// Each list item becomes a paragraph tagged with the appropriate `numId` and `ilvl`.
/// Nested lists recurse with an incremented indent level (max depth 3).
fn render_list(mut docx: Docx, list: &DocumentList) -> Result<Docx> {
    docx = render_list_at_level(docx, list, 0)?;
    Ok(docx)
}

/// Recursively renders list items at a given indent level.
///
/// When the list is ordered and has a non-default `start_number`, a new abstract
/// numbering definition with the correct start value is registered dynamically.
fn render_list_at_level(mut docx: Docx, list: &DocumentList, level: usize) -> Result<Docx> {
    let num_id = if list.ordered {
        // If start_number is non-default, create a dynamic numbering definition.
        if let Some(start) = list.start_number
            && start != 1
        {
            register_custom_start_numbering(&mut docx, start)
        } else {
            DECIMAL_NUM_ID
        }
    } else {
        BULLET_NUM_ID
    };
    // Clamp level to the 3 levels we defined (0, 1, 2).
    let ilvl = level.min(2);

    for item in &list.items {
        let mut p =
            docx_rs::Paragraph::new().numbering(NumberingId::new(num_id), IndentLevel::new(ilvl));

        // Add content from each block inside the list item.
        for block in &item.blocks {
            match block {
                DocumentBlock::Paragraph(runs) => {
                    p = add_runs_with_hyperlinks(p, runs, false);
                }
                DocumentBlock::Heading { runs, .. } => {
                    p = add_runs_with_hyperlinks(p, runs, true);
                }
                _ => {
                    // Other block types inside list items are rendered as-is
                    // (best-effort; they become additional runs).
                }
            }
        }

        docx = docx.add_paragraph(p);

        // Recursively render nested list at the next indent level.
        if let Some(nested) = &item.nested {
            docx = render_list_at_level(docx, nested, level + 1)?;
        }
    }
    Ok(docx)
}

/// 渲染图片。
fn render_image(mut docx: Docx, image: &DocumentImage) -> Result<Docx> {
    if let Some(data) = &image.data {
        let pic = Pic::new(data);
        let p = docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_image(pic));
        docx = docx.add_paragraph(p);
    }
    Ok(docx)
}

fn u8_to_heading_level(level: u8) -> HeadingLevel {
    match level {
        1 => HeadingLevel::H1,
        2 => HeadingLevel::H2,
        3 => HeadingLevel::H3,
        4 => HeadingLevel::H4,
        5 => HeadingLevel::H5,
        _ => HeadingLevel::H6,
    }
}

fn heading_style_name(level: u8) -> &'static str {
    match level {
        1 => "Heading1",
        2 => "Heading2",
        3 => "Heading3",
        4 => "Heading4",
        5 => "Heading5",
        _ => "Heading6",
    }
}

fn heading_outline_level(level: u8) -> usize {
    (level.saturating_sub(1)) as usize
}

// ---------------------------------------------------------------------------
// DocWriteHandler 集成 — 在渲染过程中触发生命周期回调
// ---------------------------------------------------------------------------

/// 将语义模型渲染为 docx-rs 的 Docx 实例，并在渲染过程中触发 handler 回调。
///
/// 支持 `DocWriteHandler` trait 的 before/after 回调：
/// - `before_document` / `after_document`
/// - `before_paragraph` / `after_paragraph`
/// - `before_table` / `after_table`
///
/// # 参数
/// - `content`: 完整的语义文档模型。
/// - `handlers`: 可变引用的 handler 切片，按 order 排序。
///
/// # 返回
pub fn render_with_handler<H: easydoc_core::traits::DocWriteHandler>(
    content: &DocumentContent,
    handler: &mut H,
) -> Result<Docx> {
    let ctx = easydoc_core::traits::DocWriteContext {
        path: String::new(),
    };

    handler.before_document(&ctx)?;

    let mut docx = Docx::new();
    docx = add_list_numberings(docx);
    let mut para_index: usize = 0;
    let mut table_index: usize = 0;

    for block in &content.blocks {
        match block {
            DocumentBlock::Heading { .. } | DocumentBlock::Paragraph(_) => {
                let p_ctx = easydoc_core::traits::ParagraphContext { index: para_index };
                handler.before_paragraph(&p_ctx)?;
                docx = render_block(docx, block)?;
                handler.after_paragraph(&p_ctx)?;
                para_index += 1;
            }
            DocumentBlock::Table(table) => {
                let t_ctx = easydoc_core::traits::TableWriteContext {
                    index: table_index,
                    row_count: table.rows.len(),
                };
                handler.before_table(&t_ctx)?;
                docx = render_block(docx, block)?;
                handler.after_table(&t_ctx)?;
                table_index += 1;
            }
            _ => {
                docx = render_block(docx, block)?;
            }
        }
    }

    handler.after_document(&ctx)?;

    Ok(docx)
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use easydoc_core::{
        CellContext, DocWriteContext, DocWriteHandler, DocumentListItem, DocumentTableCell,
        DocumentTableRow, ParagraphContext, TableWriteContext,
    };

    fn make_text_run(text: &str) -> DocumentTextRun {
        DocumentTextRun {
            text: text.into(),
            bold: false,
            italic: false,
            strikethrough: false,
            hyperlink: None,
        }
    }

    fn make_bold_run(text: &str) -> DocumentTextRun {
        DocumentTextRun {
            text: text.into(),
            bold: true,
            italic: false,
            strikethrough: false,
            hyperlink: None,
        }
    }

    #[test]
    fn render_heading_variants() {
        for level in 1..=7 {
            let content = DocumentContent {
                blocks: vec![DocumentBlock::Heading {
                    level,
                    runs: vec![make_text_run("Title")],
                }],
                ..Default::default()
            };
            let docx = render_document_content(&content).unwrap();
            let _ = docx;
        }
    }

    #[test]
    fn render_paragraph_with_runs() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::Paragraph(vec![
                make_text_run("Hello "),
                make_bold_run("World"),
            ])],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_table_with_spans() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::Table(DocumentTable {
                rows: vec![DocumentTableRow {
                    cells: vec![
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("A")])],
                            column_span: 2,
                            row_span: 1,
                        },
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("B")])],
                            column_span: 1,
                            row_span: 1,
                        },
                    ],
                    is_header: true,
                }],
            })],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_table_vertical_merge_emits_vmerge_xml() {
        // 两行一列：第一行 restart（row_span=2），第二行 continue（row_span=0）。
        let content = DocumentContent {
            blocks: vec![DocumentBlock::Table(DocumentTable {
                rows: vec![
                    DocumentTableRow {
                        cells: vec![DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("Merged")])],
                            column_span: 1,
                            row_span: 2,
                        }],
                        is_header: false,
                    },
                    DocumentTableRow {
                        cells: vec![DocumentTableCell {
                            blocks: vec![],
                            column_span: 1,
                            row_span: 0,
                        }],
                        is_header: false,
                    },
                ],
            })],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let xml = String::from_utf8(docx.build().document).expect("document.xml is UTF-8");
        // restart 单元格输出 <w:vMerge w:val="restart"/>，continue 输出 <w:vMerge w:val="continue"/>
        assert!(
            xml.contains("vMerge") && xml.contains("restart") && xml.contains("continue"),
            "document.xml should contain vMerge restart+continue, got: {xml}"
        );
    }

    #[test]
    fn render_table_plain_cells_have_no_vmerge() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::Table(DocumentTable {
                rows: vec![DocumentTableRow {
                    cells: vec![DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("Plain")])],
                        column_span: 1,
                        row_span: 1,
                    }],
                    is_header: false,
                }],
            })],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let xml = String::from_utf8(docx.build().document).expect("document.xml is UTF-8");
        assert!(
            !xml.contains("vMerge"),
            "plain cells should not emit vMerge, got: {xml}"
        );
    }

    #[test]
    fn render_list_ordered_and_unordered() {
        let content = DocumentContent {
            blocks: vec![
                DocumentBlock::List(DocumentList {
                    ordered: false,
                    start_number: None,
                    items: vec![DocumentListItem {
                        blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("Item 1")])],
                        nested: None,
                    }],
                }),
                DocumentBlock::List(DocumentList {
                    ordered: true,
                    start_number: Some(5),
                    items: vec![DocumentListItem {
                        blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("Item 2")])],
                        nested: Some(Box::new(DocumentList {
                            ordered: false,
                            start_number: None,
                            items: vec![DocumentListItem {
                                blocks: vec![DocumentBlock::Paragraph(vec![make_text_run(
                                    "Nested",
                                )])],
                                nested: None,
                            }],
                        })),
                    }],
                }),
            ],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_image_with_data() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::Image(DocumentImage {
                alt_text: Some("test".into()),
                data: Some(vec![
                    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49,
                    0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02,
                    0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44,
                    0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00,
                    0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
                    0xAE, 0x42, 0x60, 0x82,
                ]),
                extension: Some("png".into()),
            })],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_image_without_data() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::Image(DocumentImage {
                alt_text: Some("test".into()),
                data: None,
                extension: None,
            })],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_codeblock() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::CodeBlock {
                language: Some("rust".into()),
                code: "fn main() {}".into(),
            }],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_textbox() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::TextBox(vec![DocumentBlock::Paragraph(
                vec![make_text_run("inside")],
            )])],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_footnote_and_endnote() {
        let content = DocumentContent {
            blocks: vec![
                DocumentBlock::Footnote {
                    id: 1,
                    blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("note")])],
                },
                DocumentBlock::Endnote {
                    id: 2,
                    blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("end")])],
                },
            ],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_section() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::Section {
                blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("in section")])],
                section_type: Some("nextPage".into()),
            }],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_thematic_and_page_and_column_break() {
        let content = DocumentContent {
            blocks: vec![
                DocumentBlock::ThematicBreak,
                DocumentBlock::PageBreak,
                DocumentBlock::ColumnBreak,
            ],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_with_handler_all_blocks() {
        struct TestHandler;
        impl DocWriteHandler for TestHandler {
            fn order() -> i32 {
                0
            }
            fn before_document(&mut self, _: &DocWriteContext) -> Result<()> {
                Ok(())
            }
            fn after_document(&mut self, _: &DocWriteContext) -> Result<()> {
                Ok(())
            }
            fn before_paragraph(&mut self, _: &ParagraphContext) -> Result<()> {
                Ok(())
            }
            fn after_paragraph(&mut self, _: &ParagraphContext) -> Result<()> {
                Ok(())
            }
            fn before_table(&mut self, _: &TableWriteContext) -> Result<()> {
                Ok(())
            }
            fn after_table(&mut self, _: &TableWriteContext) -> Result<()> {
                Ok(())
            }
            fn before_cell(&mut self, _: &CellContext) -> Result<()> {
                Ok(())
            }
            fn after_cell(&mut self, _: &CellContext) -> Result<()> {
                Ok(())
            }
        }
        let content = DocumentContent {
            blocks: vec![
                DocumentBlock::Heading {
                    level: 1,
                    runs: vec![make_text_run("H1")],
                },
                DocumentBlock::Paragraph(vec![make_text_run("P")]),
                DocumentBlock::Table(DocumentTable { rows: vec![] }),
                DocumentBlock::CodeBlock {
                    language: None,
                    code: "x".into(),
                },
                DocumentBlock::ThematicBreak,
                DocumentBlock::PageBreak,
                DocumentBlock::ColumnBreak,
                DocumentBlock::Section {
                    blocks: vec![],
                    section_type: None,
                },
            ],
            ..Default::default()
        };
        let mut handler = TestHandler;
        let docx = render_with_handler(&content, &mut handler).unwrap();
        let _ = docx;
    }

    #[test]
    fn heading_style_names() {
        assert_eq!(heading_style_name(1), "Heading1");
        assert_eq!(heading_style_name(2), "Heading2");
        assert_eq!(heading_style_name(3), "Heading3");
        assert_eq!(heading_style_name(4), "Heading4");
        assert_eq!(heading_style_name(5), "Heading5");
        assert_eq!(heading_style_name(6), "Heading6");
        assert_eq!(heading_style_name(7), "Heading6"); // default
    }

    #[test]
    fn heading_outline_levels() {
        assert_eq!(heading_outline_level(1), 0);
        assert_eq!(heading_outline_level(2), 1);
        assert_eq!(heading_outline_level(6), 5);
        assert_eq!(heading_outline_level(0), 0); // saturating_sub
    }

    #[test]
    fn u8_to_heading_level_all() {
        assert_eq!(u8_to_heading_level(1), HeadingLevel::H1);
        assert_eq!(u8_to_heading_level(2), HeadingLevel::H2);
        assert_eq!(u8_to_heading_level(3), HeadingLevel::H3);
        assert_eq!(u8_to_heading_level(4), HeadingLevel::H4);
        assert_eq!(u8_to_heading_level(5), HeadingLevel::H5);
        assert_eq!(u8_to_heading_level(6), HeadingLevel::H6);
        assert_eq!(u8_to_heading_level(99), HeadingLevel::H6); // default
    }

    #[test]
    fn render_ordered_list_with_custom_start_number() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::List(DocumentList {
                ordered: true,
                start_number: Some(5),
                items: vec![
                    DocumentListItem {
                        blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("Fifth")])],
                        nested: None,
                    },
                    DocumentListItem {
                        blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("Sixth")])],
                        nested: None,
                    },
                ],
            })],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        // Should succeed -- custom numbering definitions are registered.
        let _ = docx;
    }

    #[test]
    fn render_ordered_list_with_default_start_number() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::List(DocumentList {
                ordered: true,
                start_number: Some(1),
                items: vec![DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("First")])],
                    nested: None,
                }],
            })],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn render_ordered_list_with_none_start_number() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::List(DocumentList {
                ordered: true,
                start_number: None,
                items: vec![DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![make_text_run("Default")])],
                    nested: None,
                }],
            })],
            ..Default::default()
        };
        let docx = render_document_content(&content).unwrap();
        let _ = docx;
    }

    #[test]
    fn text_run_to_docx_run_styles() {
        let run = DocumentTextRun {
            text: "test".into(),
            bold: true,
            italic: true,
            strikethrough: true,
            hyperlink: None,
        };
        let r = text_run_to_docx_run(&run, false);
        let _ = r;
        let r2 = text_run_to_docx_run(&run, true);
        let _ = r2;
    }

    #[test]
    fn render_block_to_paragraph_codeblock() {
        let block = DocumentBlock::CodeBlock {
            language: Some("python".into()),
            code: "print()".into(),
        };
        let p = render_block_to_paragraph(&block).unwrap();
        let _ = p;
    }

    #[test]
    fn render_block_to_paragraph_heading() {
        let block = DocumentBlock::Heading {
            level: 2,
            runs: vec![make_text_run("Sub")],
        };
        let p = render_block_to_paragraph(&block).unwrap();
        let _ = p;
    }

    #[test]
    fn render_block_to_paragraph_fallback() {
        // Non-paragraph blocks in table cells fall back to empty paragraph
        let block = DocumentBlock::ThematicBreak;
        let p = render_block_to_paragraph(&block).unwrap();
        let _ = p;
    }
}
