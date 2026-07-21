//! Quick table write executor — renders `Vec<T>` directly into a DOCX table.

use std::fs::File;
use std::io::{Cursor, Seek, Write};
use std::path::PathBuf;

use docx_rs::Docx;
use easydoc_core::style::TableStyle;
use easydoc_core::{DocError, DocxRow, Result};

/// Executor for one-shot table writes.
pub struct TableWriteExecutor<'a, T: DocxRow> {
    path: PathBuf,
    data: &'a [T],
    title: Option<String>,
    style: TableStyle,
    need_header: bool,
}

impl<'a, T: DocxRow> TableWriteExecutor<'a, T> {
    /// Creates a new table write executor.
    pub(crate) fn new(
        path: PathBuf,
        data: &'a [T],
        title: Option<String>,
        style: TableStyle,
        need_header: bool,
    ) -> Self {
        Self {
            path,
            data,
            title,
            style,
            need_header,
        }
    }

    /// Builds the docx_rs document from stored data.
    fn build_docx(&self) -> Result<Docx> {
        let mut docx = Docx::new();

        if let Some(ref title) = self.title {
            docx = docx.add_paragraph(
                docx_rs::Paragraph::new().add_run(
                    docx_rs::Run::new()
                        .add_text(title.as_str())
                        .bold()
                        .size(28),
                ),
            );
        }

        let mut rows: Vec<docx_rs::TableRow> = Vec::new();

        if self.need_header {
            let schema = T::schema();
            let header_cells: Vec<docx_rs::TableCell> = schema
                .iter()
                .filter(|c| !c.ignored)
                .map(|col| {
                    let mut run = docx_rs::Run::new().add_text(col.name.as_str());
                    if self.style.header_font.bold {
                        run = run.bold();
                    }
                    docx_rs::TableCell::new()
                        .add_paragraph(docx_rs::Paragraph::new().add_run(run))
                })
                .collect();
            rows.push(docx_rs::TableRow::new(header_cells));
        }

        for item in self.data.iter() {
            let cells = item.to_row()?;
            let data_cells: Vec<docx_rs::TableCell> = cells.iter().map(|cell| {
                let text = doc_value_str(&cell.value);
                docx_rs::TableCell::new().add_paragraph(
                    docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text(text)),
                )
            }).collect();
            rows.push(docx_rs::TableRow::new(data_cells));
        }

        docx = docx.add_table(docx_rs::Table::new(rows));
        Ok(docx)
    }

    /// Executes the write to disk.
    pub fn execute(self) -> Result<()> {
        let file = File::create(&self.path)?;
        let docx = self.build_docx()?;
        docx.build().pack(file).map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(())
    }

    /// Executes the write to a generic writer.
    ///
    /// Corresponds to Hutool's `flush(OutputStream)` pattern.
    pub fn execute_to_writer<W: Write + Seek>(self, writer: W) -> Result<()> {
        let docx = self.build_docx()?;
        docx.build().pack(writer).map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(())
    }

    /// Executes the write and returns bytes.
    pub fn execute_to_bytes(self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let cursor = Cursor::new(&mut buf);
        let docx = self.build_docx()?;
        docx.build().pack(cursor).map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(buf)
    }
}

fn doc_value_str(value: &easydoc_core::DocValue) -> String {
    match value {
        easydoc_core::DocValue::String(s) => s.clone(),
        easydoc_core::DocValue::Int(n) => n.to_string(),
        easydoc_core::DocValue::Float(n) => n.to_string(),
        easydoc_core::DocValue::Bool(b) => b.to_string(),
        easydoc_core::DocValue::Empty => String::new(),
        other => format!("{other:?}"),
    }
}
