//! Backend-neutral document tree.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::{DocumentConfig, Error, ParagraphStyle, Px, Result, Style, TextStyle};

/// A complete document and its named styles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    /// Document-wide configuration.
    pub config: DocumentConfig,
    /// Named paragraph styles.
    pub styles: BTreeMap<String, Style>,
    /// Top-level document content.
    pub blocks: Vec<Block>,
}

impl Document {
    /// Creates an empty document with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers or replaces a named style.
    pub fn register_style(&mut self, name: impl Into<String>, style: Style) {
        self.styles.insert(name.into(), style);
    }

    /// Appends a block.
    pub fn push(&mut self, block: impl Into<Block>) {
        self.blocks.push(block.into());
    }

    /// Resolves a named style.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownStyle`] when the style is not registered.
    pub fn style(&self, name: &str) -> Result<&Style> {
        self.styles
            .get(name)
            .ok_or_else(|| Error::UnknownStyle(name.to_owned()))
    }
}

/// Top-level or table-cell document content.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    /// A paragraph.
    Paragraph(Paragraph),
    /// A table.
    Table(Table),
    /// A standalone image.
    Image(Image),
    /// A page break.
    PageBreak,
}

impl From<Paragraph> for Block {
    fn from(value: Paragraph) -> Self {
        Self::Paragraph(value)
    }
}

impl From<Table> for Block {
    fn from(value: Table) -> Self {
        Self::Table(value)
    }
}

impl From<Image> for Block {
    fn from(value: Image) -> Self {
        Self::Image(value)
    }
}

/// A paragraph containing inline content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Paragraph {
    /// Optional named style.
    pub style_name: Option<String>,
    /// Local paragraph style overrides.
    pub style: ParagraphStyle,
    /// Inline children.
    pub children: Vec<Inline>,
}

impl Paragraph {
    /// Creates an empty paragraph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a paragraph with one text run.
    #[must_use]
    pub fn from_text(text: impl Into<String>) -> Self {
        Self::new().push(TextRun::new(text))
    }

    /// Selects a named style.
    #[must_use]
    pub fn style(mut self, name: impl Into<String>) -> Self {
        self.style_name = Some(name.into());
        self
    }

    /// Applies local paragraph formatting.
    #[must_use]
    pub fn format(mut self, style: ParagraphStyle) -> Self {
        self.style = style;
        self
    }

    /// Appends inline content.
    #[must_use]
    pub fn push(mut self, child: impl Into<Inline>) -> Self {
        self.children.push(child.into());
        self
    }
}

/// Inline paragraph content.
#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    /// Styled text.
    Text(TextRun),
    /// An inline image.
    Image(Image),
    /// A line break.
    LineBreak,
}

impl From<TextRun> for Inline {
    fn from(value: TextRun) -> Self {
        Self::Text(value)
    }
}

impl From<Image> for Inline {
    fn from(value: Image) -> Self {
        Self::Image(value)
    }
}

/// A run of text with uniform formatting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextRun {
    /// Text content.
    pub text: String,
    /// Local text style.
    pub style: TextStyle,
}

impl TextRun {
    /// Creates an unstyled text run.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::default(),
        }
    }

    /// Applies text formatting.
    #[must_use]
    pub fn format(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
}

/// Image bytes and rendered dimensions.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    /// Image file name used inside the DOCX package.
    pub filename: String,
    /// Encoded image bytes.
    pub data: Vec<u8>,
    /// Rendered width.
    pub width: Px,
    /// Rendered height.
    pub height: Px,
}

impl Image {
    /// Loads an image from disk.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the image cannot be read.
    pub fn from_path(path: impl AsRef<Path>, width: Px, height: Px) -> Result<Self> {
        let path = path.as_ref();
        let data = fs::read(path).map_err(|source| Error::io(path, source))?;
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("image.bin")
            .to_owned();
        Ok(Self {
            filename,
            data,
            width,
            height,
        })
    }
}

/// A Word table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    /// Table rows.
    pub rows: Vec<Row>,
    /// Whether the first row is a header row.
    pub first_row_as_header: bool,
}

impl Table {
    /// Creates an empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a row.
    #[must_use]
    pub fn push_row(mut self, row: Row) -> Self {
        self.rows.push(row);
        self
    }

    /// Marks the first row as a logical header.
    #[must_use]
    pub const fn first_row_as_header(mut self) -> Self {
        self.first_row_as_header = true;
        self
    }
}

/// A table row.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    /// Row cells.
    pub cells: Vec<Cell>,
}

impl Row {
    /// Creates a row from cells.
    #[must_use]
    pub fn new(cells: impl IntoIterator<Item = Cell>) -> Self {
        Self {
            cells: cells.into_iter().collect(),
        }
    }
}

/// A table cell containing block-level content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    /// Cell contents.
    pub blocks: Vec<Block>,
    /// Number of grid columns occupied by this cell.
    pub colspan: usize,
}

impl Cell {
    /// Creates a text-only cell.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            blocks: vec![Paragraph::from_text(text).into()],
            colspan: 1,
        }
    }

    /// Creates a cell from arbitrary blocks.
    #[must_use]
    pub fn new(blocks: impl IntoIterator<Item = Block>) -> Self {
        Self {
            blocks: blocks.into_iter().collect(),
            colspan: 1,
        }
    }

    /// Sets horizontal cell span.
    #[must_use]
    pub fn colspan(mut self, columns: usize) -> Self {
        self.colspan = columns.max(1);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Alignment, ParagraphStyle};

    #[test]
    fn resolves_named_styles() {
        let mut document = Document::new();
        document.register_style(
            "title",
            Style::paragraph(ParagraphStyle::default().align(Alignment::Center)),
        );
        assert_eq!(
            document.style("title").unwrap().paragraph.alignment,
            Some(Alignment::Center)
        );
        assert!(document.style("missing").is_err());
    }
}
