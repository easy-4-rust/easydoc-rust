//! Annotated view renderer -- most LLM-friendly format.
//!
//! Prefixes each block with a structural tag such as `[段落 3]`, `[标题1]`,
//! `[表格 2: 3行x4列]`, `[图片]`, etc.

use std::fmt::Write;

use easydoc_core::{DocumentBlock, DocumentContent, DocumentTextRun};

/// Renders the document with structural annotations.
pub fn render(content: &DocumentContent) -> String {
    let mut out = String::new();
    let mut para_idx: usize = 0;
    let mut table_idx: usize = 0;
    let mut image_idx: usize = 0;

    for block in &content.blocks {
        render_annotated_block(
            &mut out,
            block,
            &mut para_idx,
            &mut table_idx,
            &mut image_idx,
            0,
        );
    }
    out
}

fn render_annotated_block(
    out: &mut String,
    block: &DocumentBlock,
    para_idx: &mut usize,
    table_idx: &mut usize,
    image_idx: &mut usize,
    indent: usize,
) {
    let prefix = "  ".repeat(indent);
    match block {
        DocumentBlock::Heading { level, runs } => {
            let _ = write!(out, "{prefix}[标题{level}] ");
            out.push_str(&runs_text(runs));
            out.push('\n');
        }
        DocumentBlock::Paragraph(runs) => {
            *para_idx += 1;
            let _ = write!(out, "{prefix}[段落 {para_idx}] ");
            out.push_str(&runs_text(runs));
            out.push('\n');
        }
        DocumentBlock::Table(table) => {
            *table_idx += 1;
            let rows = table.rows.len();
            let cols = table.rows.first().map_or(0, |r| r.cells.len());
            let _ = writeln!(out, "{prefix}[表格 {table_idx}: {rows}行x{cols}列]");
            for (ri, row) in table.rows.iter().enumerate() {
                let row_num = ri + 1;
                let _ = write!(out, "{prefix}  行{row_num}: ");
                let cell_texts: Vec<String> = row
                    .cells
                    .iter()
                    .map(|c| blocks_text_flat(&c.blocks))
                    .collect();
                out.push_str(&cell_texts.join(" | "));
                out.push('\n');
            }
        }
        DocumentBlock::List(list) => {
            let list_type = if list.ordered {
                "有序列表"
            } else {
                "无序列表"
            };
            let count = list.items.len();
            let _ = writeln!(out, "{prefix}[{list_type} {count}项]");
            for (i, item) in list.items.iter().enumerate() {
                let bullet = if list.ordered {
                    format!("  {}. ", list.start_number.unwrap_or(1) + i as u32)
                } else {
                    "  - ".to_owned()
                };
                out.push_str(&prefix);
                out.push_str(&bullet);
                out.push_str(&blocks_text_flat(&item.blocks));
                out.push('\n');
                if let Some(nested) = &item.nested {
                    let nested_block = DocumentBlock::List((**nested).clone());
                    render_annotated_block(
                        out,
                        &nested_block,
                        para_idx,
                        table_idx,
                        image_idx,
                        indent + 1,
                    );
                }
            }
        }
        DocumentBlock::Image(image) => {
            *image_idx += 1;
            let alt = image.alt_text.as_deref().unwrap_or("无描述");
            let _ = writeln!(out, "{prefix}[图片 {image_idx}] {alt}");
        }
        DocumentBlock::PageBreak => {
            let _ = writeln!(out, "{prefix}[分页]");
        }
        DocumentBlock::ColumnBreak => {
            let _ = writeln!(out, "{prefix}[分栏]");
        }
        DocumentBlock::CodeBlock { language, code } => {
            let lang = language.as_deref().unwrap_or("text");
            let _ = writeln!(out, "{prefix}[代码块 {lang}]");
            for line in code.lines() {
                let _ = writeln!(out, "{prefix}  {line}");
            }
        }
        DocumentBlock::ThematicBreak => {
            let _ = writeln!(out, "{prefix}[分隔线]");
        }
        DocumentBlock::TextBox(blocks)
        | DocumentBlock::Footnote { blocks, .. }
        | DocumentBlock::Endnote { blocks, .. }
        | DocumentBlock::Section { blocks, .. } => {
            let label = match block {
                DocumentBlock::TextBox(_) => "文本框",
                DocumentBlock::Footnote { id, .. } => {
                    let _ = writeln!(out, "{prefix}[脚注 {id}]");
                    ""
                }
                DocumentBlock::Endnote { id, .. } => {
                    let _ = writeln!(out, "{prefix}[尾注 {id}]");
                    ""
                }
                DocumentBlock::Section { section_type, .. } => {
                    let stype = section_type.as_deref().unwrap_or("default");
                    let _ = writeln!(out, "{prefix}[分区 {stype}]");
                    ""
                }
                _ => unreachable!(),
            };
            if !label.is_empty() {
                let _ = writeln!(out, "{prefix}[{label}]");
            }
            for b in blocks {
                render_annotated_block(out, b, para_idx, table_idx, image_idx, indent + 1);
            }
        }
        _ => {}
    }
}

fn runs_text(runs: &[DocumentTextRun]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
}

fn blocks_text_flat(blocks: &[DocumentBlock]) -> String {
    let mut out = String::new();
    for b in blocks {
        match b {
            DocumentBlock::Paragraph(runs) | DocumentBlock::Heading { runs, .. } => {
                out.push_str(&runs_text(runs));
            }
            _ => {}
        }
    }
    out
}
