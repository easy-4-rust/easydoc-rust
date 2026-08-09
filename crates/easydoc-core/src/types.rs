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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn doc_value_from_string() {
        let v: DocValue = "hello".into();
        assert!(matches!(v, DocValue::String(s) if s == "hello"));
    }

    #[test]
    fn doc_value_from_owned_string() {
        let v: DocValue = String::from("world").into();
        assert!(matches!(v, DocValue::String(s) if s == "world"));
    }

    #[test]
    fn doc_value_from_bool() {
        let v: DocValue = true.into();
        assert!(matches!(v, DocValue::Bool(true)));
    }

    #[test]
    fn doc_value_from_i64() {
        let v: DocValue = 42i64.into();
        assert!(matches!(v, DocValue::Int(42)));
    }

    #[test]
    fn doc_value_from_i32() {
        let v: DocValue = 7i32.into();
        assert!(matches!(v, DocValue::Int(7)));
    }

    #[test]
    fn doc_value_from_u32() {
        let v: DocValue = 99u32.into();
        assert!(matches!(v, DocValue::Int(99)));
    }

    #[test]
    fn doc_value_from_f64() {
        let v: DocValue = 3.14f64.into();
        assert!(matches!(v, DocValue::Float(f) if (f - 3.14).abs() < f64::EPSILON));
    }

    #[test]
    fn doc_value_from_datetime_utc() {
        let dt = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let v: DocValue = dt.into();
        assert!(matches!(v, DocValue::DateTime(_)));
    }

    #[test]
    fn doc_value_from_naive_date() {
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let v: DocValue = d.into();
        assert!(matches!(v, DocValue::Date(_)));
    }

    #[test]
    fn doc_value_from_naive_datetime() {
        let ndt = NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let v: DocValue = ndt.into();
        assert!(matches!(v, DocValue::NaiveDateTime(_)));
    }

    #[test]
    fn doc_value_from_option_some() {
        let v: DocValue = Some("test").into();
        assert!(matches!(v, DocValue::String(s) if s == "test"));
    }

    #[test]
    fn doc_value_from_option_none() {
        let v: DocValue = Option::<String>::None.into();
        assert!(matches!(v, DocValue::Empty));
    }

    #[test]
    fn cell_data_new_and_alignment() {
        let cell = CellData::new("hello").alignment(HorizontalAlignment::Center);
        assert!(matches!(cell.value, DocValue::String(_)));
        assert_eq!(cell.alignment, Some(HorizontalAlignment::Center));
        assert_eq!(cell.col_span, 1);
        assert_eq!(cell.row_span, 1);
    }

    #[test]
    fn row_data_new() {
        let cells = vec![CellData::new("a"), CellData::new("b")];
        let row = RowData::new(cells);
        assert_eq!(row.cells.len(), 2);
        assert!(row.height.is_none());
    }

    #[test]
    fn horizontal_alignment_debug_and_eq() {
        assert_eq!(HorizontalAlignment::Left, HorizontalAlignment::Left);
        assert_ne!(HorizontalAlignment::Left, HorizontalAlignment::Right);
        let _ = format!("{:?}", HorizontalAlignment::Both);
    }

    #[test]
    fn heading_level_variants() {
        assert_ne!(HeadingLevel::H1, HeadingLevel::H2);
        assert_ne!(HeadingLevel::H3, HeadingLevel::H6);
    }

    #[test]
    fn error_action_variants() {
        assert_eq!(ErrorAction::Continue, ErrorAction::Continue);
        assert_ne!(ErrorAction::Stop, ErrorAction::Skip);
    }

    #[test]
    fn rich_run_debug() {
        let run = RichRun {
            text: "hi".into(),
            bold: true,
            italic: false,
            size: Some(24),
            color: Some(0xFF0000),
            font: Some("Arial".into()),
        };
        let dbg = format!("{:?}", run);
        assert!(dbg.contains("hi"));
    }

    #[test]
    fn image_data_debug() {
        let img = ImageData {
            bytes: vec![0x89, 0x50],
            extension: "png".into(),
            width: Some(100),
            height: Some(200),
            alt_text: Some("test".into()),
        };
        assert!(format!("{:?}", img).contains("png"));
    }
}
