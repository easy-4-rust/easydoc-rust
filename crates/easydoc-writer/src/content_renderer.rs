//! 语义模型渲染器 — 将 `DocumentContent` 转换为 docx-rs 文档。
//!
//! 这是打通 Read → Modify → Write 闭环的关键桥梁。
//! Reader 输出 `DocumentContent`，本渲染器将其渲染为 DOCX。

use docx_rs::{BreakType, Docx, Pic, RunFonts};
use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentImage, DocumentList, DocumentTable, DocumentTextRun,
    HeadingLevel, Result,
};

/// 将核心语义模型渲染为 docx-rs 的 `Docx` 实例。
///
/// # 参数
/// - `content`: 完整的语义文档模型。
///
/// # 返回
/// 构建好的 `Docx` 实例，可进一步 `pack()` 为 DOCX 文件。
pub fn render_document_content(content: &DocumentContent) -> Result<Docx> {
    let mut docx = Docx::new();

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
            let mut p = docx_rs::Paragraph::new()
                .style(heading_style_name(*level))
                .outline_lvl(heading_outline_level(*level));
            for run in runs {
                let r = text_run_to_docx_run(run, true);
                p = p.add_run(r);
            }
            docx = docx.add_paragraph(p);
        }
        DocumentBlock::Paragraph(runs) => {
            let mut p = docx_rs::Paragraph::new();
            for run in runs {
                let r = text_run_to_docx_run(run, false);
                p = p.add_run(r);
            }
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
        DocumentBlock::Footnote { id: _, blocks } => {
            // 脚注内容渲染为缩进段落
            for inner in blocks {
                docx = render_block(docx, inner)?;
            }
        }
        DocumentBlock::Endnote { id: _, blocks } => {
            // 尾注内容渲染为缩进段落
            for inner in blocks {
                docx = render_block(docx, inner)?;
            }
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
            cells.push(tc);
        }
        rows.push(docx_rs::TableRow::new(cells));
    }

    docx = docx.add_table(docx_rs::Table::new(rows));
    Ok(docx)
}

/// 将单个块渲染为段落（用于表格单元格内）。
fn render_block_to_paragraph(block: &DocumentBlock) -> Result<docx_rs::Paragraph> {
    match block {
        DocumentBlock::Heading { level: _, runs } => {
            let mut p = docx_rs::Paragraph::new();
            for run in runs {
                let r = text_run_to_docx_run(run, true);
                p = p.add_run(r);
            }
            Ok(p)
        }
        DocumentBlock::Paragraph(runs) => {
            let mut p = docx_rs::Paragraph::new();
            for run in runs {
                let r = text_run_to_docx_run(run, false);
                p = p.add_run(r);
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

/// 渲染语义列表。
fn render_list(mut docx: Docx, list: &DocumentList) -> Result<Docx> {
    for (i, item) in list.items.iter().enumerate() {
        let bullet = if list.ordered {
            let start = list.start_number.unwrap_or(1);
            format!("{}. ", start + i as u32)
        } else {
            "• ".to_string()
        };

        let mut p = docx_rs::Paragraph::new();
        let bullet_run = docx_rs::Run::new().add_text(&bullet);
        p = p.add_run(bullet_run);

        for block in &item.blocks {
            if let DocumentBlock::Paragraph(runs) = block {
                for run in runs {
                    let r = text_run_to_docx_run(run, false);
                    p = p.add_run(r);
                }
            }
        }

        docx = docx.add_paragraph(p);

        // 递归渲染嵌套列表
        if let Some(nested) = &item.nested {
            docx = render_list(docx, nested)?;
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
    use easydoc_core::*;

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
