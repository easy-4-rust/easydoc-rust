//! Document write executor — orchestrates the assembly of a complete DOCX file.

use std::fs::File;
use std::io::{Seek, Write};
use std::path::PathBuf;

use easydoc_core::metadata::DocumentMeta;
use easydoc_core::{DocError, Result};

use crate::builder::doc_builder::DocumentElement;

use docx_rs::{BreakType, Docx, Pic, RunFonts};

/// Executor for rendering a [`DocBuilder`] into a physical DOCX file.
///
/// Wraps `docx-rs` for the actual OOXML generation.
pub struct DocWriteExecutor {
    path: PathBuf,
    #[allow(dead_code)]
    meta: DocumentMeta,
    elements: Vec<DocumentElement>,
}

impl DocWriteExecutor {
    /// Creates a new executor from builder output.
    pub(crate) fn new(
        path: PathBuf,
        meta: DocumentMeta,
        elements: Vec<DocumentElement>,
    ) -> Result<Self> {
        Ok(Self {
            path,
            meta,
            elements,
        })
    }

    /// Builds the `docx_rs` document from stored elements.
    fn build_docx(&self) -> Result<Docx> {
        let mut docx = Docx::new();

        for element in &self.elements {
            match element {
                DocumentElement::Heading { text, level } => {
                    let run = docx_rs::Run::new().add_text(text.as_str()).bold().size(28);
                    let p = docx_rs::Paragraph::new().add_run(run);
                    docx = docx.add_paragraph(p);
                    let _ = level;
                }
                DocumentElement::Paragraph(para) => {
                    let mut p = docx_rs::Paragraph::new();
                    for run in para.clone().into_runs() {
                        let mut r = docx_rs::Run::new();
                        r = r.add_text(run.run_text());
                        if let Some(font) = run.font_config() {
                            if font.bold {
                                r = r.bold();
                            }
                            if font.italic {
                                r = r.italic();
                            }
                            if let Some(size) = font.size {
                                r = r.size(size as usize);
                            }
                            if let Some(color) = font.color {
                                r = r.color(format!("{:06X}", color.to_hex()));
                            }
                            if let Some(name) = &font.name {
                                r = r.fonts(RunFonts::new().ascii(name.as_str()));
                            }
                            if font.underline {
                                r = r.underline("single");
                            }
                        }
                        p = p.add_run(r);
                    }
                    if let Some(style) = para.paragraph_style()
                        && let Some(alignment) = style.alignment
                    {
                        p = p.align(convert_alignment(alignment));
                    }
                    docx = docx.add_paragraph(p);
                }
                DocumentElement::Table(table) => {
                    let mut rows: Vec<docx_rs::TableRow> = Vec::new();

                    // Header row
                    let header_cells: Vec<docx_rs::TableCell> = table
                        .headers()
                        .iter()
                        .map(|h| {
                            docx_rs::TableCell::new().add_paragraph(
                                docx_rs::Paragraph::new()
                                    .add_run(docx_rs::Run::new().add_text(h.as_str()).bold()),
                            )
                        })
                        .collect();
                    rows.push(docx_rs::TableRow::new(header_cells));

                    // Data rows
                    for row in table.rows() {
                        let cells: Vec<docx_rs::TableCell> = row
                            .iter()
                            .map(|cell| {
                                let text = doc_value_to_string(&cell.value);
                                docx_rs::TableCell::new().add_paragraph(
                                    docx_rs::Paragraph::new()
                                        .add_run(docx_rs::Run::new().add_text(text)),
                                )
                            })
                            .collect();
                        rows.push(docx_rs::TableRow::new(cells));
                    }

                    docx = docx.add_table(docx_rs::Table::new(rows));
                }
                DocumentElement::Image(image) => {
                    let bytes = std::fs::read(&image.path).map_err(|e| {
                        DocError::Document(format!(
                            "cannot read image {}: {e}",
                            image.path.display()
                        ))
                    })?;
                    let pic = if let (Some(w), Some(h)) = (image.width, image.height) {
                        Pic::new_with_dimensions(bytes, w, h)
                    } else {
                        Pic::new(&bytes)
                    };
                    docx = docx.add_paragraph(
                        docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_image(pic)),
                    );
                }
                DocumentElement::PageBreak => {
                    docx = docx.add_paragraph(
                        docx_rs::Paragraph::new()
                            .add_run(docx_rs::Run::new().add_break(BreakType::Page)),
                    );
                }
            }
        }

        Ok(docx)
    }

    /// Saves the assembled document to disk.
    ///
    /// # Errors
    ///
    /// Returns an I/O or ZIP error if the file cannot be written.
    pub fn save(self) -> Result<()> {
        let file = File::create(&self.path)?;
        let docx = self.build_docx()?;
        docx.build()
            .pack(file)
            .map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(())
    }

    /// Writes the assembled document to a generic writer.
    ///
    /// Corresponds to Hutool's `Word07Writer.flush(OutputStream)`.
    /// The writer must implement both `Write` and `Seek` (required by docx-rs).
    ///
    /// # Errors
    ///
    /// Returns an I/O or ZIP error.
    pub fn save_to_writer<W: Write + Seek>(self, writer: W) -> Result<()> {
        let docx = self.build_docx()?;
        docx.build()
            .pack(writer)
            .map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(())
    }

    /// Writes the assembled document to a `Vec<u8>` buffer.
    pub fn save_to_bytes(self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let cursor = std::io::Cursor::new(&mut buf);
        let docx = self.build_docx()?;
        docx.build()
            .pack(cursor)
            .map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(buf)
    }
}

fn doc_value_to_string(value: &easydoc_core::DocValue) -> String {
    match value {
        easydoc_core::DocValue::String(s) => s.clone(),
        easydoc_core::DocValue::Int(n) => n.to_string(),
        easydoc_core::DocValue::Float(n) => n.to_string(),
        easydoc_core::DocValue::Bool(b) => b.to_string(),
        easydoc_core::DocValue::Empty => String::new(),
        other => format!("{other:?}"),
    }
}

fn convert_alignment(
    alignment: easydoc_core::types::HorizontalAlignment,
) -> docx_rs::AlignmentType {
    match alignment {
        easydoc_core::types::HorizontalAlignment::Left => docx_rs::AlignmentType::Left,
        easydoc_core::types::HorizontalAlignment::Center => docx_rs::AlignmentType::Center,
        easydoc_core::types::HorizontalAlignment::Right => docx_rs::AlignmentType::Right,
        easydoc_core::types::HorizontalAlignment::Both => docx_rs::AlignmentType::Both,
    }
}
