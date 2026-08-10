//! Statistics view renderer.

use easydoc_core::{DocumentBlock, DocumentContent, DocumentTextRun};

/// Renders aggregate statistics about the document content.
pub fn render(content: &DocumentContent) -> String {
    let mut paragraphs: usize = 0;
    let mut tables: usize = 0;
    let mut images: usize = 0;
    let mut headings: usize = 0;
    let mut word_count: usize = 0;

    for block in &content.blocks {
        count_block(
            block,
            &mut paragraphs,
            &mut tables,
            &mut images,
            &mut headings,
            &mut word_count,
        );
    }

    format!(
        "段落数: {paragraphs}\n标题数: {headings}\n表格数: {tables}\n图片数: {images}\n字数: {word_count}",
    )
}

fn count_block(
    block: &DocumentBlock,
    paragraphs: &mut usize,
    tables: &mut usize,
    images: &mut usize,
    headings: &mut usize,
    word_count: &mut usize,
) {
    match block {
        DocumentBlock::Heading { runs, .. } => {
            *headings += 1;
            *word_count += count_runs_words(runs);
        }
        DocumentBlock::Paragraph(runs) => {
            *paragraphs += 1;
            *word_count += count_runs_words(runs);
        }
        DocumentBlock::Table(table) => {
            *tables += 1;
            for row in &table.rows {
                for cell in &row.cells {
                    for b in &cell.blocks {
                        count_block(b, paragraphs, tables, images, headings, word_count);
                    }
                }
            }
        }
        DocumentBlock::List(list) => {
            for item in &list.items {
                for b in &item.blocks {
                    count_block(b, paragraphs, tables, images, headings, word_count);
                }
                if let Some(nested) = &item.nested {
                    let nested_block = DocumentBlock::List((**nested).clone());
                    count_block(
                        &nested_block,
                        paragraphs,
                        tables,
                        images,
                        headings,
                        word_count,
                    );
                }
            }
        }
        DocumentBlock::Image(_) => {
            *images += 1;
        }
        DocumentBlock::CodeBlock { code, .. } => {
            *paragraphs += 1;
            *word_count += code.split_whitespace().count();
        }
        DocumentBlock::TextBox(blocks)
        | DocumentBlock::Footnote { blocks, .. }
        | DocumentBlock::Endnote { blocks, .. }
        | DocumentBlock::Section { blocks, .. } => {
            for b in blocks {
                count_block(b, paragraphs, tables, images, headings, word_count);
            }
        }
        _ => {}
    }
}

/// Counts "words" in a sequence of text runs.
///
/// Uses whitespace splitting. For CJK text this is an approximation.
fn count_runs_words(runs: &[DocumentTextRun]) -> usize {
    runs.iter().flat_map(|r| r.text.split_whitespace()).count()
}
