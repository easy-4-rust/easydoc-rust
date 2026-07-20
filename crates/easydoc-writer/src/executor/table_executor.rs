//! Quick table write executor — renders `Vec<T>` directly into a DOCX table.

use std::fs::File;
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

    /// Executes the write.
    ///
    /// # Errors
    ///
    /// Returns I/O or conversion errors.
    pub fn execute(self) -> Result<()> {
        let file = File::create(&self.path)?;
        let mut docx = Docx::new();

        // Optional title heading
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

        // Build rows
        let mut rows: Vec<docx_rs::TableRow> = Vec::new();

        // Header row
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

        // Data rows
        for item in self.data.iter() {
            let cells = item.to_row()?;
            let data_cells: Vec<docx_rs::TableCell> = cells
                .iter()
                .map(|cell| {
                    let text = match &cell.value {
                        easydoc_core::DocValue::String(s) => s.clone(),
                        easydoc_core::DocValue::Int(n) => n.to_string(),
                        easydoc_core::DocValue::Float(n) => n.to_string(),
                        easydoc_core::DocValue::Bool(b) => b.to_string(),
                        easydoc_core::DocValue::Empty => String::new(),
                        other => format!("{other:?}"),
                    };
                    docx_rs::TableCell::new().add_paragraph(
                        docx_rs::Paragraph::new()
                            .add_run(docx_rs::Run::new().add_text(text)),
                    )
                })
                .collect();
            rows.push(docx_rs::TableRow::new(data_cells));
        }

        docx = docx.add_table(docx_rs::Table::new(rows));
        docx.build().pack(file).map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(())
    }
}
