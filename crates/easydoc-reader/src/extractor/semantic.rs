use std::path::Path;

use easydoc_core::{
    DocError, DocumentBlock, DocumentContent, DocumentImage, DocumentList, DocumentListItem,
    DocumentMeta, DocumentTable, DocumentTableCell, DocumentTableRow, DocumentTextRun, Result,
};
use office_oxide::ir::{Element, InlineContent};

/// 将 DOC/DOCX 解析为 easydoc 的后端无关语义模型。
pub(crate) fn extract_document(path: &Path) -> Result<DocumentContent> {
    let document = office_oxide::Document::open(path)
        .map_err(|error| DocError::Document(format!("failed to open document: {error}")))?;
    let ir = document.to_ir();
    let metadata = DocumentMeta {
        title: ir.metadata.title,
        author: ir.metadata.author,
        subject: ir.metadata.subject.or(ir.metadata.description),
        keywords: (!ir.metadata.keywords.is_empty()).then(|| ir.metadata.keywords.join(", ")),
        ..DocumentMeta::default()
    };
    let blocks = ir
        .sections
        .iter()
        .flat_map(|section| section.elements.iter())
        .filter_map(convert_element)
        .collect();
    Ok(DocumentContent { metadata, blocks })
}

fn convert_elements(elements: &[Element]) -> Vec<DocumentBlock> {
    elements.iter().filter_map(convert_element).collect()
}

fn convert_element(element: &Element) -> Option<DocumentBlock> {
    match element {
        Element::Heading(heading) => Some(DocumentBlock::Heading {
            level: heading.level.clamp(1, 6),
            runs: convert_inline(&heading.content),
        }),
        Element::Paragraph(paragraph) => {
            Some(DocumentBlock::Paragraph(convert_inline(&paragraph.content)))
        }
        Element::Table(table) => Some(DocumentBlock::Table(DocumentTable {
            rows: table
                .rows
                .iter()
                .map(|row| DocumentTableRow {
                    cells: row
                        .cells
                        .iter()
                        .map(|cell| DocumentTableCell {
                            blocks: convert_elements(&cell.content),
                            column_span: cell.col_span.max(1),
                            row_span: cell.row_span.max(1),
                        })
                        .collect(),
                    is_header: row.is_header,
                })
                .collect(),
        })),
        Element::List(list) => Some(DocumentBlock::List(convert_list(list))),
        Element::Image(image) => Some(DocumentBlock::Image(DocumentImage {
            alt_text: image.alt_text.clone(),
            data: image.data.clone(),
            extension: image
                .format
                .as_ref()
                .map(|format| format.extension().to_owned()),
        })),
        Element::ThematicBreak => Some(DocumentBlock::ThematicBreak),
        Element::TextBox(text_box) => {
            Some(DocumentBlock::TextBox(convert_elements(&text_box.content)))
        }
        Element::PageBreak => Some(DocumentBlock::PageBreak),
        Element::ColumnBreak => Some(DocumentBlock::ColumnBreak),
        Element::Footnote(note) => Some(DocumentBlock::Footnote {
            id: note.id,
            blocks: convert_elements(&note.content),
        }),
        Element::Endnote(note) => Some(DocumentBlock::Endnote {
            id: note.id,
            blocks: convert_elements(&note.content),
        }),
        Element::CodeBlock(code) => Some(DocumentBlock::CodeBlock {
            language: code.language.clone(),
            code: code.content.clone(),
        }),
        _ => None,
    }
}

fn convert_inline(content: &[InlineContent]) -> Vec<DocumentTextRun> {
    content
        .iter()
        .map(|inline| match inline {
            InlineContent::Text(span) => DocumentTextRun {
                text: span.text.clone(),
                bold: span.bold,
                italic: span.italic,
                strikethrough: span.strikethrough,
                hyperlink: span.hyperlink.clone(),
            },
            InlineContent::LineBreak => DocumentTextRun {
                text: "\n".to_owned(),
                ..DocumentTextRun::default()
            },
            InlineContent::FootnoteRef(reference) => DocumentTextRun {
                text: format!("[^{}]", reference.note_id),
                ..DocumentTextRun::default()
            },
            InlineContent::EndnoteRef(reference) => DocumentTextRun {
                text: format!("[^endnote-{}]", reference.note_id),
                ..DocumentTextRun::default()
            },
            _ => DocumentTextRun::default(),
        })
        .collect()
}

fn convert_list(list: &office_oxide::ir::List) -> DocumentList {
    DocumentList {
        ordered: list.ordered,
        start_number: list.start_number,
        items: list
            .items
            .iter()
            .map(|item| DocumentListItem {
                blocks: convert_elements(&item.content),
                nested: item
                    .nested
                    .as_ref()
                    .map(|nested| Box::new(convert_list(nested))),
            })
            .collect(),
    }
}
