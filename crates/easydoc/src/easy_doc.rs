use std::path::{Path, PathBuf};

use easydoc_core::{DocxRow, Result};
use easydoc_reader::DocReadBuilder;
use easydoc_writer::{DocBuilder, TableWriteBuilder};

/// Static factory — the single entry point for all `easydoc` operations.
///
/// Mirrors the `EasyExcel` factory pattern from `easyexcel-rs`:
/// every read, write, or template operation begins with a static method
/// returning a fluent builder.
///
/// # Examples
///
/// ```ignore
/// // Write a table
/// EasyDoc::write_table("output.docx", &data).do_write()?;
///
/// // Build a document
/// EasyDoc::document("report.docx")
///     .add_heading("Title", HeadingLevel::H1)
///     .build()?
///     .save()?;
///
/// // Read text
/// let text = EasyDoc::read_text("document.docx")?;
/// ```
pub struct EasyDoc;

impl EasyDoc {
    // ========================================================================
    // Write APIs
    // ========================================================================

    /// Creates a new document builder for building paragraphs, tables, and more.
    ///
    /// Returns a [`DocBuilder`] — the main document construction API.
    #[must_use]
    pub fn document(path: impl Into<PathBuf>) -> DocBuilder {
        DocBuilder::new(path)
    }

    /// Quick one-liner: writes a `Vec<Struct>` as a DOCX table.
    ///
    /// Requires `T: DocxRow` (implemented via `#[derive(DocxRow)]`).
    /// Returns a [`TableWriteBuilder`] for further configuration.
    #[must_use]
    pub fn write_table<T: DocxRow>(
        path: impl Into<PathBuf>,
        data: &[T],
    ) -> TableWriteBuilder<T> {
        TableWriteBuilder::new(path, data)
    }

    /// Fills scalar `{key}` placeholders in a DOCX template.
    ///
    /// The `data` map provides key → replacement value pairs.
    ///
    /// # Errors
    ///
    /// Returns I/O or template-processing errors.
    pub fn fill_template(
        template: impl AsRef<Path>,
        output: impl AsRef<Path>,
        data: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        easydoc_template::fill_template(template.as_ref(), output.as_ref(), data)
    }

    /// Fills a DOCX template with collection expansion (`{.field}` placeholders).
    ///
    /// Collection data is expanded into table rows.
    ///
    /// # Errors
    ///
    /// Returns I/O or template-processing errors.
    pub fn fill_template_list<T: serde::Serialize + std::fmt::Debug>(
        template: impl AsRef<Path>,
        output: impl AsRef<Path>,
        data: &[T],
        list_field: &str,
    ) -> Result<()> {
        easydoc_template::fill_template_list(
            template.as_ref(),
            output.as_ref(),
            data,
            list_field,
        )
    }

    // ========================================================================
    // Read APIs
    // ========================================================================

    /// Creates a streaming document reader.
    ///
    /// Auto-detects DOCX / DOC format from file extension and magic bytes.
    #[must_use]
    pub fn read(path: impl Into<PathBuf>) -> DocReadBuilder {
        DocReadBuilder::new(path)
    }

    /// Synchronously reads all plain text from a document.
    ///
    /// # Errors
    ///
    /// Returns I/O or format errors.
    pub fn read_text(path: impl AsRef<Path>) -> Result<String> {
        easydoc_reader::read_text(path.as_ref())
    }

    /// Synchronously reads all tables from a document, deserialising each
    /// into `Vec<T>` via the [`DocxRow`] trait.
    ///
    /// # Errors
    ///
    /// Returns I/O, format, or conversion errors.
    pub fn read_tables<T: DocxRow>(path: impl AsRef<Path>) -> Result<Vec<Vec<T>>> {
        easydoc_reader::read_tables::<T>(path.as_ref())
    }
}
