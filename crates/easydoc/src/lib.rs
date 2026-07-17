//! Ergonomic DOCX generation for Rust.
//!
//! `easydoc` combines a Hutool-style writer facade with a backend-neutral
//! document model and an explicit, fallible `finish` operation.

use std::path::{Path, PathBuf};

pub use easydoc_core::{
    Alignment, Block, Cell, Document, DocumentConfig, Error, FontFamily, Image, Inline, Length,
    Margins, PageSize, Paragraph, ParagraphStyle, Pt, Px, Result, Row, Style, Table, TextRun,
    TextStyle,
};
use easydoc_docx::DocxRenderer;

/// Static entry point for fluent document creation.
#[derive(Clone, Copy, Debug, Default)]
pub struct EasyDoc;

impl EasyDoc {
    /// Starts a DOCX writer builder.
    #[must_use]
    pub fn write(path: impl Into<PathBuf>) -> DocxWriterBuilder {
        DocxWriterBuilder::new(path)
    }
}

/// Configures a [`DocxWriter`] before content is added.
#[derive(Clone, Debug)]
pub struct DocxWriterBuilder {
    path: PathBuf,
    config: DocumentConfig,
}

impl DocxWriterBuilder {
    /// Creates a writer builder for a destination path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            config: DocumentConfig::default(),
        }
    }

    /// Sets the default font across all Word font slots.
    #[must_use]
    pub fn default_font(mut self, family: impl Into<String>) -> Self {
        self.config.default_font = FontFamily::all(family);
        self
    }

    /// Sets the default font size.
    #[must_use]
    pub const fn default_font_size(mut self, size: Pt) -> Self {
        self.config.default_font_size = size;
        self
    }

    /// Sets the page size.
    #[must_use]
    pub const fn page_size(mut self, page_size: PageSize) -> Self {
        self.config.page_size = page_size;
        self
    }

    /// Sets page margins.
    #[must_use]
    pub const fn margins(mut self, margins: Margins) -> Self {
        self.config.margins = margins;
        self
    }

    /// Builds the writer without creating the output file yet.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidInput`] when the destination is invalid.
    pub fn build(self) -> Result<DocxWriter> {
        DocxWriter::with_config(self.path, self.config)
    }
}

/// Stateful, ergonomic DOCX writer.
///
/// The file is only created by [`Self::finish`]. Dropping a writer never hides
/// an output error or emits a partial document.
#[derive(Debug)]
pub struct DocxWriter {
    path: PathBuf,
    document: Document,
}

impl DocxWriter {
    /// Creates a writer with default configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidInput`] when the destination is invalid.
    pub fn create(path: impl Into<PathBuf>) -> Result<Self> {
        Self::with_config(path, DocumentConfig::default())
    }

    /// Creates a writer with explicit document configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidInput`] when the destination is invalid.
    pub fn with_config(path: impl Into<PathBuf>, config: DocumentConfig) -> Result<Self> {
        let path = path.into();
        validate_destination(&path)?;
        Ok(Self {
            path,
            document: Document {
                config,
                ..Document::default()
            },
        })
    }

    /// Creates a writer around an existing document model.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidInput`] when the destination is invalid.
    pub fn from_document(path: impl Into<PathBuf>, document: Document) -> Result<Self> {
        let path = path.into();
        validate_destination(&path)?;
        Ok(Self { path, document })
    }

    /// Registers or replaces a named style.
    pub fn register_style(&mut self, name: impl Into<String>, style: Style) -> &mut Self {
        self.document.register_style(name, style);
        self
    }

    /// Adds a heading using built-in level defaults.
    pub fn add_heading(&mut self, text: impl Into<String>, level: u8) -> &mut Self {
        let level = level.clamp(1, 6);
        let size = Pt(28.0 - f32::from(level) * 2.0);
        let style = ParagraphStyle::default().text(TextStyle::default().size(size).bold());
        self.document.push(Paragraph::from_text(text).format(style));
        self
    }

    /// Adds a paragraph.
    pub fn add_paragraph(&mut self, paragraph: Paragraph) -> &mut Self {
        self.document.push(paragraph);
        self
    }

    /// Adds a simple text paragraph.
    pub fn add_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.add_paragraph(Paragraph::from_text(text))
    }

    /// Adds a table.
    pub fn add_table(&mut self, table: Table) -> &mut Self {
        self.document.push(table);
        self
    }

    /// Adds a centred, standalone image.
    pub fn add_image(&mut self, image: Image) -> &mut Self {
        self.document.push(image);
        self
    }

    /// Adds a page break.
    pub fn add_page_break(&mut self) -> &mut Self {
        self.document.blocks.push(Block::PageBreak);
        self
    }

    /// Returns the backend-neutral document model.
    #[must_use]
    pub const fn document(&self) -> &Document {
        &self.document
    }

    /// Returns mutable access to the document model.
    pub const fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// Renders and explicitly completes the DOCX output.
    ///
    /// # Errors
    ///
    /// Returns an I/O, style resolution, or DOCX packaging error.
    pub fn finish(self) -> Result<()> {
        DocxRenderer::render_to_path(&self.document, self.path)
    }
}

fn validate_destination(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(Error::InvalidInput(
            "destination path must not be empty".to_owned(),
        ));
    }
    if path
        .extension()
        .is_some_and(|extension| extension != "docx")
    {
        return Err(Error::InvalidInput(
            "destination must use the .docx extension".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;

    use zip::ZipArchive;

    use super::*;

    #[test]
    fn writer_produces_a_docx_with_fluent_operations() {
        let directory = std::env::temp_dir().join(format!("easydoc-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("report.docx");

        let mut writer = EasyDoc::write(&path)
            .default_font("宋体")
            .default_font_size(Pt(12.0))
            .build()
            .unwrap();
        writer
            .add_heading("年度经营报告", 1)
            .add_text("以下为本年度经营数据。")
            .add_table(
                Table::new()
                    .push_row(Row::new([Cell::text("项目"), Cell::text("数量")]))
                    .push_row(Row::new([Cell::text("订单"), Cell::text("120")]))
                    .first_row_as_header(),
            );
        writer.finish().unwrap();

        let mut archive = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        assert!(xml.contains("年度经营报告"));
        assert!(xml.contains("以下为本年度经营数据。"));

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_non_docx_destinations() {
        assert!(DocxWriter::create("report.pdf").is_err());
    }
}
