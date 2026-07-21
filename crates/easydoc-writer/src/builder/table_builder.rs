//! Quick table writer -- one-liner `Vec<Struct>` -> DOCX table.

use std::path::PathBuf;

use easydoc_core::style::TableStyle;
use easydoc_core::DocxRow;
use easydoc_core::Result;

use crate::executor::table_executor::TableWriteExecutor;

/// Fluent builder for writing a typed `Vec<T>` as a DOCX table.
///
/// Created via [`EasyDoc::write_table()`].
///
/// # Example
///
/// ```ignore
/// EasyDoc::write_table("users.docx", &users)
///     .title("User List")
///     .header_style(TableStyle::header())
///     .do_write()?;
/// ```
pub struct TableWriteBuilder<'a, T: DocxRow> {
    path: PathBuf,
    data: &'a [T],
    title: Option<String>,
    style: TableStyle,
    need_header: bool,
}

impl<'a, T: DocxRow> TableWriteBuilder<'a, T> {
    /// Creates a new table write builder.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, data: &'a [T]) -> Self {
        Self {
            path: path.into(),
            data,
            title: None,
            style: TableStyle::default(),
            need_header: true,
        }
    }

    /// Sets a document title (heading above the table).
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Controls whether the header row is emitted.
    #[must_use]
    pub fn need_header(mut self, need: bool) -> Self {
        self.need_header = need;
        self
    }

    /// Sets the table style.
    #[must_use]
    pub fn header_style(mut self, style: TableStyle) -> Self {
        self.style = style;
        self
    }

    /// Enables zebra striping on the table.
    #[must_use]
    pub fn banded_rows(mut self, enabled: bool) -> Self {
        self.style.banded_rows = enabled;
        self
    }

    /// Executes the write and saves the document to disk.
    ///
    /// # Errors
    ///
    /// Returns I/O or conversion errors.
    pub fn do_write(self) -> Result<()> {
        let executor = TableWriteExecutor::new(
            self.path,
            self.data,
            self.title,
            self.style,
            self.need_header,
        );
        executor.execute()
    }

    /// Executes the write and returns the document as bytes.
    ///
    /// Useful for in-memory generation without touching the filesystem.
    /// Corresponds to Hutool's pattern of writing to a `ByteArrayOutputStream`.
    ///
    /// # Errors
    ///
    /// Returns ZIP or conversion errors.
    pub fn do_write_to_bytes(self) -> Result<Vec<u8>> {
        let executor = TableWriteExecutor::new(
            self.path,
            self.data,
            self.title,
            self.style,
            self.need_header,
        );
        executor.execute_to_bytes()
    }

    /// Executes the write to a generic writer implementing `Write + Seek`.
    ///
    /// Corresponds to Hutool's `flush(OutputStream)`.
    ///
    /// # Errors
    ///
    /// Returns I/O, ZIP, or conversion errors.
    pub fn do_write_to_writer<W: std::io::Write + std::io::Seek>(self, writer: W) -> Result<()> {
        let executor = TableWriteExecutor::new(
            self.path,
            self.data,
            self.title,
            self.style,
            self.need_header,
        );
        executor.execute_to_writer(writer)
    }
}
