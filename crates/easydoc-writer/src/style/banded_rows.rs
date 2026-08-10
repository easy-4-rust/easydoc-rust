//! 斑马条纹策略（交替行颜色）。
//!
//! 对应 Java: `com.alibaba.excel.write.metadata.style.WriteCellStyle#tableStyle`

use easydoc_core::style::Color;

/// 为表格应用交替行背景色。
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_strategy() {
        let s = BandedRowsStrategy::default();
        assert_eq!(s.even_color, Color::rgb(242, 242, 242));
        assert!(s.odd_color.is_none());
    }

    #[test]
    fn new_equals_default() {
        let s = BandedRowsStrategy::new();
        assert_eq!(s.even_color, BandedRowsStrategy::default().even_color);
    }

    #[test]
    fn even_rows_use_even_color() {
        let s = BandedRowsStrategy::new();
        assert_eq!(s.color_for_row(0), Some(Color::rgb(242, 242, 242)));
        assert_eq!(s.color_for_row(2), Some(Color::rgb(242, 242, 242)));
        assert_eq!(s.color_for_row(4), Some(Color::rgb(242, 242, 242)));
    }

    #[test]
    fn odd_rows_use_odd_color() {
        let s = BandedRowsStrategy::new();
        assert_eq!(s.color_for_row(1), None);
        assert_eq!(s.color_for_row(3), None);
    }

    #[test]
    fn custom_odd_color() {
        let s = BandedRowsStrategy {
            odd_color: Some(Color::WHITE),
            ..Default::default()
        };
        assert_eq!(s.color_for_row(1), Some(Color::WHITE));
    }
}
