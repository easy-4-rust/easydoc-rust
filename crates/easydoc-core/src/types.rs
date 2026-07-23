//! Core data types for document content.
//!
//! These types form the universal intermediate representation between
//! typed Rust values and document cell/paragraph content, analogous to
//! `CellValue` in `easyexcel-rust`.

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

/// Universal document value — the bridge between Rust types and DOCX content.
///
/// Every `DocConverter` reads from or writes to this enum, exactly as
/// `CellValue` mediates between Rust types and Excel cell data.
#[derive(Debug, Clone)]
pub enum DocValue {
    /// Plain text string.
    String(String),
    /// Boolean value.
    Bool(bool),
    /// Signed 64-bit integer.
    Int(i64),
    /// 64-bit floating-point number.
    Float(f64),
    /// UTC date-time.
    DateTime(DateTime<Utc>),
    /// Date only (no time component).
    Date(NaiveDate),
    /// Date and time without timezone.
    NaiveDateTime(NaiveDateTime),
    /// Empty / null value.
    Empty,
    /// Rich text (formatted runs).
    RichText(Vec<RichRun>),
    /// Image data (raw bytes + metadata).
    Image(ImageData),
}

/// A single formatted text run within a rich-text cell.
#[derive(Debug, Clone)]
pub struct RichRun {
    /// The text content.
    pub text: String,
    /// Whether this run is bold.
    pub bold: bool,
    /// Whether this run is italic.
    pub italic: bool,
    /// Font size in half-points (e.g. 24 = 12pt).
    pub size: Option<u32>,
    /// RGB color (e.g. `0xFF0000` for red).
    pub color: Option<u32>,
    /// Font family name.
    pub font: Option<String>,
}

/// Image payload with metadata.
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Raw image bytes.
    pub bytes: Vec<u8>,
    /// File extension / MIME hint (e.g. "png", "jpg").
    pub extension: String,
    /// Desired width in EMU (English Metric Units) or pixels.
    pub width: Option<u32>,
    /// Desired height in EMU or pixels.
    pub height: Option<u32>,
    /// Alt-text description.
    pub alt_text: Option<String>,
}

/// Data for a single table cell.
///
/// Carries the converted value together with optional formatting overrides.
#[derive(Debug, Clone)]
pub struct CellData {
    /// The cell's converted value.
    pub value: DocValue,
    /// Optional horizontal alignment override.
    pub alignment: Option<HorizontalAlignment>,
    /// Column span for merged cells (1 = normal).
    pub col_span: u32,
    /// Row span for merged cells (1 = normal).
    pub row_span: u32,
}

impl CellData {
    /// Creates a new cell from any value that can be converted into a [`DocValue`].
    pub fn new(value: impl Into<DocValue>) -> Self {
        Self {
            value: value.into(),
            alignment: None,
            col_span: 1,
            row_span: 1,
        }
    }

    /// Sets horizontal alignment for this cell.
    #[must_use]
    pub fn alignment(mut self, alignment: HorizontalAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
}

/// Data for a single table row.
#[derive(Debug, Clone)]
pub struct RowData {
    /// Cells in the row, in column order.
    pub cells: Vec<CellData>,
    /// Row height hint (in twips, 1/20 of a point).
    pub height: Option<u32>,
}

impl RowData {
    /// Creates a row from cells.
    #[must_use]
    pub fn new(cells: Vec<CellData>) -> Self {
        Self {
            cells,
            height: None,
        }
    }
}

/// Full table data extracted from a document during reading.
#[derive(Debug, Clone)]
pub struct TableData {
    /// Header row (if available).
    pub headers: Option<Vec<String>>,
    /// Data rows.
    pub rows: Vec<Vec<String>>,
}

/// Paragraph alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    /// Align left.
    Left,
    /// Align center.
    Center,
    /// Align right.
    Right,
    /// Align both / justify.
    Both,
}

/// Heading level for structured documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingLevel {
    /// Heading 1 (largest).
    H1,
    /// Heading 2.
    H2,
    /// Heading 3.
    H3,
    /// Heading 4.
    H4,
    /// Heading 5.
    H5,
    /// Heading 6 (smallest).
    H6,
}

/// Action to take when a read error occurs.
///
/// Returned by [`DocReadListener::on_error`](crate::traits::DocReadListener::on_error).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    /// Skip the error and continue reading.
    Continue,
    /// Skip the current construct (paragraph / table row) and continue.
    Skip,
    /// Stop reading immediately.
    Stop,
}

// ---------------------------------------------------------------------------
// From impls — convenience conversions into DocValue
// ---------------------------------------------------------------------------

impl From<String> for DocValue {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for DocValue {
    fn from(v: &str) -> Self {
        Self::String(v.to_owned())
    }
}

impl From<bool> for DocValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i64> for DocValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<i32> for DocValue {
    fn from(v: i32) -> Self {
        Self::Int(i64::from(v))
    }
}

impl From<u32> for DocValue {
    fn from(v: u32) -> Self {
        Self::Int(i64::from(v))
    }
}

impl From<f64> for DocValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<DateTime<Utc>> for DocValue {
    fn from(v: DateTime<Utc>) -> Self {
        Self::DateTime(v)
    }
}

impl From<NaiveDate> for DocValue {
    fn from(v: NaiveDate) -> Self {
        Self::Date(v)
    }
}

impl From<NaiveDateTime> for DocValue {
    fn from(v: NaiveDateTime) -> Self {
        Self::NaiveDateTime(v)
    }
}

impl<T: Into<DocValue>> From<Option<T>> for DocValue {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(inner) => inner.into(),
            None => Self::Empty,
        }
    }
}
