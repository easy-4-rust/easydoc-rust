//! Plain-text view renderer.

use easydoc_core::{DocumentBlock, DocumentContent, DocumentTextRun};

/// Renders the document as plain text.
///
/// Paragraphs are separated by blank lines. Table cells within a row are
/// separated by `, `, rows by newlines. Other block types are rendered as
/// best-effort text.
pub fn render(content: &DocumentContent) -> String {
    let mut out = String::new();
    for (i, block) in content.blocks.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        render_block(&mut out, block);
    }
    out
}

fn render_block(out: &mut String, block: &DocumentBlock) {
    match block {
        DocumentBlock::Heading { runs, .. } | DocumentBlock::Paragraph(runs) => {
            out.push_str(&runs_text(runs));
        }
        DocumentBlock::Table(table) => {
            for (ri, row) in table.rows.iter().enumerate() {
                if ri > 0 {
                    out.push('\n');
                }
                let cell_texts: Vec<String> =
                    row.cells.iter().map(|c| blocks_text(&c.blocks)).collect();
                out.push_str(&cell_texts.join(", "));
            }
        }
        DocumentBlock::List(list) => {
            for (i, item) in list.items.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                let prefix = if list.ordered {
                    format!("{}. ", list.start_number.unwrap_or(1) + i as u32)
                } else {
                    "- ".to_owned()
                };
                out.push_str(&prefix);
                out.push_str(&blocks_text(&item.blocks));
            }
        }
        DocumentBlock::Image(image) => {
            out.push_str(image.alt_text.as_deref().unwrap_or("[image]"));
        }
        DocumentBlock::PageBreak => out.push_str("--- page break ---"),
        DocumentBlock::CodeBlock { code, .. } => out.push_str(code),
        DocumentBlock::ThematicBreak => out.push_str("---"),
        DocumentBlock::TextBox(blocks)
        | DocumentBlock::Footnote { blocks, .. }
        | DocumentBlock::Endnote { blocks, .. }
        | DocumentBlock::Section { blocks, .. } => {
            for b in blocks {
                render_block(out, b);
            }
        }
        _ => {}
    }
}

fn runs_text(runs: &[DocumentTextRun]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
}

fn blocks_text(blocks: &[DocumentBlock]) -> String {
    let mut out = String::new();
    for b in blocks {
        render_block(&mut out, b);
    }
    out
}
