//! Parsed OOXML cell width type.

/// Parsed OOXML cell width: a numeric value paired with its unit type.
///
/// Returned by [`super::parse_width`] and consumed by `docx_rs::TableCell::width`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParsedWidth {
    /// Width value in the target unit (twips for `Dxa`, percentage * 50 for `Pct`).
    pub value: usize,
    /// OOXML width type.
    pub width_type: docx_rs::WidthType,
}
