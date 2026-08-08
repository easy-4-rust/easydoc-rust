//! Document builder — the main entry point for building DOCX documents.

use std::path::PathBuf;

use easydoc_core::Result;
use easydoc_core::metadata::DocumentMeta;
use easydoc_core::types::HeadingLevel;

use crate::executor::write_executor::DocWriteExecutor;
use crate::{DocImage, Paragraph, Table};

/// Fluent builder for constructing complete DOCX documents.
///
/// Created via the facade's `EasyDoc::document()` method.
///
/// # Example
///
/// ```ignore
/// EasyDoc::document("report.docx")
///     .title("Report")
///     .add_heading("Section 1", HeadingLevel::H1)
///     .add_paragraph(Paragraph::new().add_text("Content..."))
///     .add_table(Table::from_data(&rows))
///     .build()?
///     .save()?;
/// ```
pub struct DocBuilder {
    path: PathBuf,
    meta: DocumentMeta,
    elements: Vec<DocumentElement>,
}

/// A single element in the document — paragraph, table, image, etc.
pub(crate) enum DocumentElement {
    Heading { text: String, level: HeadingLevel },
    Paragraph(Paragraph),
    Table(Table),
    Image(DocImage),
    PageBreak,
}

impl DocBuilder {
    /// Creates a new document builder targeting the given output path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            meta: DocumentMeta::default(),
            elements: Vec::new(),
        }
    }

    /// Sets the document title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.meta = self.meta.title(title);
        self
    }

    /// Sets the document author.
    #[must_use]
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.meta = self.meta.author(author);
        self
    }

    /// Adds a heading paragraph.
    #[must_use]
    pub fn add_heading(mut self, text: impl Into<String>, level: HeadingLevel) -> Self {
        self.elements.push(DocumentElement::Heading {
            text: text.into(),
            level,
        });
        self
    }

    /// Adds a paragraph.
    #[must_use]
    pub fn add_paragraph(mut self, paragraph: Paragraph) -> Self {
        self.elements.push(DocumentElement::Paragraph(paragraph));
        self
    }

    /// Adds a table.
    #[must_use]
    pub fn add_table(mut self, table: Table) -> Self {
        self.elements.push(DocumentElement::Table(table));
        self
    }

    /// Adds an image.
    #[must_use]
    pub fn add_image(mut self, image: DocImage) -> Self {
        self.elements.push(DocumentElement::Image(image));
        self
    }

    /// Adds a page break.
    #[must_use]
    pub fn add_page_break(mut self) -> Self {
        self.elements.push(DocumentElement::PageBreak);
        self
    }

    /// Finalises the builder and returns a [`DocWriteExecutor`] ready to save.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be assembled.
    pub fn build(self) -> Result<DocWriteExecutor> {
        DocWriteExecutor::new(self.path, self.meta, self.elements)
    }

    /// Builds and immediately saves the document to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be written.
    pub fn save(self) -> Result<()> {
        self.build()?.save()
    }

    /// Builds and writes the document to a generic writer implementing `Write + Seek`.
    ///
    /// Corresponds to Hutool's `Word07Writer.flush(OutputStream)` pattern.
    /// Useful for writing to memory buffers, HTTP responses, etc.
    ///
    /// # Errors
    ///
    /// Returns an I/O or ZIP error.
    pub fn save_to_writer<W: std::io::Write + std::io::Seek>(self, writer: W) -> Result<()> {
        self.build()?.save_to_writer(writer)
    }

    /// Builds and returns the document as a `Vec<u8>`.
    ///
    /// Useful for in-memory generation without touching the filesystem.
    ///
    /// # Errors
    ///
    /// Returns a ZIP error if packaging fails.
    pub fn save_to_bytes(self) -> Result<Vec<u8>> {
        self.build()?.save_to_bytes()
    }
}
