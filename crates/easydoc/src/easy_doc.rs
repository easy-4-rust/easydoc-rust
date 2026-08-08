use std::path::{Path, PathBuf};

use easydoc_core::{DocxRow, Result};
use easydoc_markdown::{MarkdownBuilder, MarkdownResult};
use easydoc_reader::DocReadBuilder;
use easydoc_writer::{DocBuilder, DocEditor, TableWriteBuilder};

/// Static factory — the single entry point for all `easydoc` operations.
///
/// Mirrors the `EasyExcel` factory pattern from `easyexcel-rust`:
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
    ) -> TableWriteBuilder<'_, T> {
        TableWriteBuilder::new(path, data)
    }

    /// Quick one-liner: creates a document and returns it as bytes.
    ///
    /// Corresponds to Hutool's pattern of writing to `ByteArrayOutputStream`.
    ///
    /// # Errors
    ///
    /// Returns ZIP or I/O errors.
    pub fn document_to_bytes(f: impl FnOnce(DocBuilder) -> DocBuilder) -> Result<Vec<u8>> {
        let builder = DocBuilder::new("memory.docx");
        f(builder).save_to_bytes()
    }

    /// Quick one-liner: writes a table and returns it as bytes.
    ///
    /// # Errors
    ///
    /// Returns ZIP or conversion errors.
    pub fn write_table_to_bytes<T: DocxRow>(data: &[T]) -> Result<Vec<u8>> {
        TableWriteBuilder::new("memory.docx", data).do_write_to_bytes()
    }

    /// Opens an existing DOCX file for editing.
    ///
    /// Corresponds to Hutool's `Word07Writer(File)` pattern that opens
    /// existing files. Uses `office_oxide`'s `EditableDocument` for text
    /// replacement and structural edits.
    ///
    /// # Errors
    ///
    /// Returns I/O or format errors if the file cannot be opened.
    pub fn edit(path: impl AsRef<Path>) -> Result<DocEditor> {
        DocEditor::open(path.as_ref())
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
        easydoc_template::fill_template_list(template.as_ref(), output.as_ref(), data, list_field)
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

    /// 创建 DOC/DOCX 到 Markdown 的转换构建器。
    #[must_use]
    pub fn markdown(path: impl Into<PathBuf>) -> MarkdownBuilder {
        MarkdownBuilder::new(path)
    }

    /// 使用默认选项快速把 DOC/DOCX 转换为 Markdown 文本。
    ///
    /// # Errors
    ///
    /// 源文档无法解析时返回错误。
    pub fn to_markdown(path: impl Into<PathBuf>) -> Result<String> {
        Ok(MarkdownBuilder::new(path).do_convert()?.markdown)
    }

    /// 使用默认选项转换并将 Markdown 原子写入目标文件。
    ///
    /// # Errors
    ///
    /// 转换或输出失败时返回错误。
    pub fn write_markdown(
        source: impl Into<PathBuf>,
        output: impl AsRef<Path>,
    ) -> Result<MarkdownResult> {
        MarkdownBuilder::new(source).write_to(output)
    }
}
