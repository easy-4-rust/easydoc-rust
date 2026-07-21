//! Zebra striping strategy for alternating row colors.

use easydoc_core::style::Color;

/// Applies alternating row background colors to a table.
#[derive(Debug, Clone)]
pub struct BandedRowsStrategy {
    /// Color for even-numbered rows (0, 2, 4…).
    pub even_color: Color,
    /// Color for odd-numbered rows (1, 3, 5…). `None` = transparent.
    pub odd_color: Option<Color>,
}

impl Default for BandedRowsStrategy {
    fn default() -> Self {
        Self {
            even_color: Color::rgb(242, 242, 242),
            odd_color: None,
        }
    }
}

impl BandedRowsStrategy {
    /// Creates a new banded rows strategy.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the background color for a given row index.
    #[must_use]
    pub fn color_for_row(&self, row_index: usize) -> Option<Color> {
        if row_index.is_multiple_of(2) {
            Some(self.even_color)
        } else {
            self.odd_color
        }
    }
}
