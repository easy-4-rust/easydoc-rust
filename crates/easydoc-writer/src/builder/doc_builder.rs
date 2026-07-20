//! Document builder — the main entry point for building DOCX documents.

use std::path::PathBuf;

use easydoc_core::metadata::DocumentMeta;
use easydoc_core::types::HeadingLevel;
use easydoc_core::Result;

use crate::executor::write_executor::DocWriteExecutor;
use crate::{DocImage, Paragraph, Table};

/// Fluent builder for constructing complete DOCX documents.
///
/// Created via [`EasyDoc::document()`].
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
    Heading {
        text: String,
        level: HeadingLevel,
    },
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

    /// Builds and immediately saves the document.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be written.
    pub fn save(self) -> Result<()> {
        self.build()?.save()
    }
}
