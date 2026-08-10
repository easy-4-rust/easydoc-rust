//! DOCX document writer for `easydoc-rust`.
//!
//! Wraps `docx-rs` behind a fluent builder API mirroring
//! `easyexcel-writer`'s patterns.

#![deny(unsafe_code)]

mod builder;
pub mod content_renderer;
mod doc_editor;
mod executor;
mod handler;
mod style;

pub use builder::doc_builder::DocBuilder;
pub use builder::table_builder::TableWriteBuilder;
pub use doc_editor::DocEditor;
pub use executor::table_executor::TableWriteExecutor;
pub use executor::write_executor::DocWriteExecutor;
pub use handler::DocWriteHandler;
pub use style::auto_width::AutoWidthStrategy;
pub use style::banded_rows::BandedRowsStrategy;

use std::path::PathBuf;

// Re-export key types for the facade
pub use easydoc_core::{
    CellData, Color, DocValue, DocumentBlock, DocumentContent, DocumentTextRun, DocxRow,
    FontConfig, HorizontalAlignment, ParagraphStyle, TableStyle,
};

// ---------------------------------------------------------------------------
// Paragraph builder
// ---------------------------------------------------------------------------

/// A paragraph composed of text runs.
#[derive(Clone)]
pub struct Paragraph {
    runs: Vec<Run>,
    style: Option<ParagraphStyle>,
}

impl Paragraph {
    /// Creates an empty paragraph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            style: None,
        }
    }

    /// Adds plain text to the paragraph.
    #[must_use]
    pub fn add_text(mut self, text: impl Into<String>) -> Self {
        self.runs.push(Run::text(text));
        self
    }

    /// Adds a pre-configured [`Run`] to the paragraph.
    #[must_use]
    pub fn add_run(mut self, run: Run) -> Self {
        self.runs.push(run);
        self
    }

    /// Sets paragraph alignment.
    #[must_use]
    pub fn alignment(mut self, alignment: HorizontalAlignment) -> Self {
        self.style.get_or_insert_default().alignment = Some(alignment);
        self
    }

    pub(crate) fn into_runs(self) -> Vec<Run> {
        self.runs
    }

    pub(crate) fn paragraph_style(&self) -> Option<&ParagraphStyle> {
        self.style.as_ref()
    }
}

impl Default for Paragraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Run builder
// ---------------------------------------------------------------------------

/// A formatted text run within a paragraph.
#[derive(Clone)]
pub struct Run {
    text: String,
    font: Option<FontConfig>,
}

impl Run {
    /// Creates a run with plain text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font: None,
        }
    }

    /// Creates a run with plain text (alias).
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(text)
    }

    /// Makes this run bold.
    #[must_use]
    pub fn bold(mut self) -> Self {
        self.font.get_or_insert_default().bold = true;
        self
    }

    /// Makes this run italic.
    #[must_use]
    pub fn italic(mut self) -> Self {
        self.font.get_or_insert_default().italic = true;
        self
    }

    /// Sets the font size in half-points (e.g. 24 = 12pt).
    #[must_use]
    pub fn size(mut self, size: u32) -> Self {
        self.font.get_or_insert_default().size = Some(size);
        self
    }

    /// Sets the text color.
    #[must_use]
    pub fn color(mut self, hex: u32) -> Self {
        self.font.get_or_insert_default().color = Some(Color::from_hex(hex));
        self
    }

    /// Sets the font family.
    #[must_use]
    pub fn font(mut self, name: impl Into<String>) -> Self {
        self.font.get_or_insert_default().name = Some(name.into());
        self
    }

    /// Underlines the text.
    #[must_use]
    pub fn underline(mut self) -> Self {
        self.font.get_or_insert_default().underline = true;
        self
    }

    pub(crate) fn run_text(&self) -> &str {
        &self.text
    }

    pub(crate) fn font_config(&self) -> Option<&FontConfig> {
        self.font.as_ref()
    }
}

// ---------------------------------------------------------------------------
// Table struct (from data)
// ---------------------------------------------------------------------------

/// A table constructed from typed data.
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<CellData>>,
    style: Option<TableStyle>,
}

impl Table {
    /// Creates a table from a slice of any `DocxRow`-implementing type.
    #[must_use]
    pub fn from_data<T: DocxRow>(data: &[T]) -> Self {
        let headers = T::schema()
            .iter()
            .filter(|c| !c.ignored)
            .map(|c| c.name.clone())
            .collect();

        let rows = data.iter().filter_map(|item| item.to_row().ok()).collect();

        Self {
            headers,
            rows,
            style: None,
        }
    }

    /// Sets the table style.
    #[must_use]
    pub fn header_style(mut self, style: TableStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Enables zebra striping.
    #[must_use]
    pub fn banded_rows(mut self, enabled: bool) -> Self {
        self.style.get_or_insert_default().banded_rows = enabled;
        self
    }

    /// Enables auto column width.
    #[must_use]
    pub fn auto_width(mut self) -> Self {
        self.style.get_or_insert_default().auto_width = true;
        self
    }

    pub(crate) fn headers(&self) -> &[String] {
        &self.headers
    }

    pub(crate) fn rows(&self) -> &[Vec<CellData>] {
        &self.rows
    }

    #[allow(dead_code)]
    pub(crate) fn table_style(&self) -> Option<&TableStyle> {
        self.style.as_ref()
    }
}

// ---------------------------------------------------------------------------
// Image builder
// ---------------------------------------------------------------------------

/// Configuration for inserting an image into a document.
pub struct DocImage {
    /// Path to the image file.
    pub path: PathBuf,
    /// Desired width in pixels (applied via `Pic::new_with_dimensions`).
    pub(crate) width: Option<u32>,
    /// Desired height in pixels.
    pub(crate) height: Option<u32>,
    alt_text: Option<String>,
}

impl DocImage {
    /// Creates an image from a file path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            width: None,
            height: None,
            alt_text: None,
        }
    }

    /// Sets the image width in pixels.
    #[must_use]
    pub fn width(mut self, w: u32) -> Self {
        self.width = Some(w);
        self
    }

    /// Sets the image height in pixels.
    #[must_use]
    pub fn height(mut self, h: u32) -> Self {
        self.height = Some(h);
        self
    }

    /// Sets alt text.
    #[must_use]
    pub fn alt_text(mut self, text: impl Into<String>) -> Self {
        self.alt_text = Some(text.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_new_is_empty() {
        let p = Paragraph::new();
        assert!(p.runs.is_empty());
        assert!(p.style.is_none());
    }

    #[test]
    fn paragraph_add_text() {
        let p = Paragraph::new().add_text("hello");
        assert_eq!(p.runs.len(), 1);
        assert_eq!(p.runs[0].run_text(), "hello");
    }

    #[test]
    fn paragraph_add_run() {
        let run = Run::new("bold").bold();
        let p = Paragraph::new().add_run(run);
        assert_eq!(p.runs.len(), 1);
        assert!(p.runs[0].font_config().unwrap().bold);
    }

    #[test]
    fn paragraph_alignment() {
        let p = Paragraph::new().alignment(HorizontalAlignment::Center);
        assert!(p.paragraph_style().is_some());
        assert_eq!(
            p.paragraph_style().unwrap().alignment,
            Some(HorizontalAlignment::Center)
        );
    }

    #[test]
    fn run_text_constructor() {
        let r = Run::text("test");
        assert_eq!(r.run_text(), "test");
        assert!(r.font_config().is_none());
    }

    #[test]
    fn run_builder_chain() {
        let r = Run::new("styled")
            .bold()
            .italic()
            .size(28)
            .color(0xFF0000)
            .font("Arial")
            .underline();
        let font = r.font_config().unwrap();
        assert!(font.bold);
        assert!(font.italic);
        assert_eq!(font.size, Some(28));
        assert_eq!(font.color, Some(Color::from_hex(0xFF0000)));
        assert_eq!(font.name.as_deref(), Some("Arial"));
        assert!(font.underline);
    }

    #[test]
    fn table_from_data_empty() {
        let users: Vec<TestUser> = vec![];
        let t = Table::from_data(&users);
        assert!(t.rows().is_empty());
        assert!(!t.headers().is_empty());
    }

    #[test]
    fn table_from_data_with_rows() {
        let users = vec![
            TestUser {
                name: "Alice".into(),
                age: 30,
                email: "a@b.com".into(),
            },
            TestUser {
                name: "Bob".into(),
                age: 25,
                email: "b@c.com".into(),
            },
        ];
        let t = Table::from_data(&users);
        assert_eq!(t.rows().len(), 2);
        assert_eq!(t.headers().len(), 3);
    }

    #[test]
    fn table_builder_methods() {
        let t = Table::from_data::<TestUser>(&[])
            .banded_rows(true)
            .auto_width();
        assert!(t.style.is_some());
        assert!(t.style.as_ref().unwrap().banded_rows);
        assert!(t.style.as_ref().unwrap().auto_width);
    }

    #[test]
    fn doc_image_builder() {
        let img = DocImage::new("/tmp/test.png")
            .width(100)
            .height(200)
            .alt_text("test image");
        assert_eq!(img.path, std::path::PathBuf::from("/tmp/test.png"));
        assert_eq!(img.width, Some(100));
        assert_eq!(img.height, Some(200));
    }

    // Helper struct for table tests
    #[derive(Debug, Clone)]
    struct TestUser {
        name: String,
        age: u32,
        email: String,
    }

    impl DocxRow for TestUser {
        fn schema() -> &'static [easydoc_core::metadata::TableColumn] {
            static SCHEMA: std::sync::LazyLock<Vec<easydoc_core::metadata::TableColumn>> =
                std::sync::LazyLock::new(|| {
                    vec![
                        easydoc_core::metadata::TableColumn::new("Name", "name", 0),
                        easydoc_core::metadata::TableColumn::new("Age", "age", 1),
                        easydoc_core::metadata::TableColumn::new("Email", "email", 2),
                    ]
                });
            &SCHEMA
        }

        fn from_row(_row: &easydoc_core::RowData) -> easydoc_core::Result<Self> {
            unimplemented!()
        }
        fn from_row_with_converters(
            _row: &easydoc_core::RowData,
            _registry: &easydoc_core::ConverterRegistry,
        ) -> easydoc_core::Result<Self> {
            unimplemented!()
        }
        fn to_row(&self) -> easydoc_core::Result<Vec<easydoc_core::CellData>> {
            Ok(vec![
                easydoc_core::CellData::new(self.name.clone()),
                easydoc_core::CellData::new(self.age as i64),
                easydoc_core::CellData::new(self.email.clone()),
            ])
        }
        fn to_row_with_converters(
            &self,
            _registry: &easydoc_core::ConverterRegistry,
        ) -> easydoc_core::Result<Vec<easydoc_core::CellData>> {
            self.to_row()
        }
    }
}
