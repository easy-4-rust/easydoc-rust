//! Table extraction from DOCX/DOC files via `office_oxide` IR.

use std::path::Path;

use easydoc_core::{CellData, DocError, DocxRow, Result, RowData};
use office_oxide::ir::{DocumentIR, Element, InlineContent};

/// Extracts all tables from a document and deserialises each into `Vec<T>`.
///
/// Uses `office_oxide`'s IR (intermediate representation) to find tables,
/// then converts each row via the [`DocxRow`] trait.
///
/// # Errors
///
/// Returns I/O, format, or conversion errors.
pub fn extract_tables<T: DocxRow>(path: &Path) -> Result<Vec<Vec<T>>> {
    let doc = office_oxide::Document::open(path)
        .map_err(|e| DocError::Document(format!("failed to open document: {e}")))?;
    let ir = doc.to_ir();
    extract_tables_from_ir(&ir)
}

/// Extracts tables from an already-parsed IR.
fn extract_tables_from_ir<T: DocxRow>(ir: &DocumentIR) -> Result<Vec<Vec<T>>> {
    let mut all_tables: Vec<Vec<T>> = Vec::new();

    for section in &ir.sections {
        for element in &section.elements {
            if let Element::Table(table) = element {
                let mut rows: Vec<T> = Vec::new();
                let mut header_skipped = false;

                for row in &table.rows {
                    // Skip header row (first row marked as header)
                    if row.is_header && !header_skipped {
                        header_skipped = true;
                        continue;
                    }

                    // Extract cell text values
                    let cells: Vec<CellData> = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let text = cell_text(cell);
                            CellData::new(text)
                        })
                        .collect();

                    let row_data = RowData::new(cells);
                    match T::from_row(&row_data) {
                        Ok(item) => rows.push(item),
                        Err(e) => {
                            // Skip row on conversion error
                            eprintln!("warning: skipping row: {e}");
                        }
                    }
                }

                if !rows.is_empty() {
                    all_tables.push(rows);
                }
            }
        }
    }

    Ok(all_tables)
}

/// Extracts all text from a table cell by flattening paragraph content.
fn cell_text(cell: &office_oxide::ir::TableCell) -> String {
    let mut text = String::new();
    for element in &cell.content {
        if let Element::Paragraph(para) = element {
            if !text.is_empty() {
                text.push('\n');
            }
            for inline in &para.content {
                if let InlineContent::Text(span) = inline {
                    text.push_str(&span.text);
                }
            }
        }
    }
    text
}
