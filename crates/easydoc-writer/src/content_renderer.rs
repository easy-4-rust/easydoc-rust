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
