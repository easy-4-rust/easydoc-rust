//! 自动列宽策略。
//!
//! 对应 Java: `com.alibaba.excel.write.metadata.style.WriteCellStyle#autoSizeColumnStrategy`

/// 自动列宽计算策略。
///
/// 根据每列中最长的单元格内容计算列宽。
#[derive(Debug, Clone, Default)]
pub struct AutoWidthStrategy {
    /// Minimum column width in twips (default: ~1 character).
    pub min_width: u32,
    /// Maximum column width in twips (default: ~40 characters).
    pub max_width: u32,
}

impl AutoWidthStrategy {
    /// Creates a new auto-width strategy with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            min_width: 240,  // ~1 character at 11pt
            max_width: 9600, // ~40 characters at 11pt
        }
    }

    /// Calculates the width for a column based on its content.
    #[must_use]
    pub fn calculate_width(&self, max_content_length: usize) -> u32 {
        let char_width = 240; // approximate twips per character at 11pt
        let width = max_content_length as u32 * char_width;
        width.clamp(self.min_width, self.max_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_strategy() {
        let s = AutoWidthStrategy::default();
        assert_eq!(s.min_width, 0);
        assert_eq!(s.max_width, 0);
    }

    #[test]
    fn new_has_correct_defaults() {
        let s = AutoWidthStrategy::new();
        assert_eq!(s.min_width, 240);
        assert_eq!(s.max_width, 9600);
    }

    #[test]
    fn calculate_width_short_content() {
        let s = AutoWidthStrategy::new();
        let w = s.calculate_width(1);
        assert_eq!(w, 240); // clamped to min
    }

    #[test]
    fn calculate_width_medium_content() {
        let s = AutoWidthStrategy::new();
        let w = s.calculate_width(10);
        assert_eq!(w, 2400); // 10 * 240
    }

    #[test]
    fn calculate_width_long_content() {
        let s = AutoWidthStrategy::new();
        let w = s.calculate_width(100);
        assert_eq!(w, 9600); // clamped to max
    }

    #[test]
    fn calculate_width_zero_content() {
        let s = AutoWidthStrategy::new();
        let w = s.calculate_width(0);
        assert_eq!(w, 240); // clamped to min
    }

    #[test]
    fn custom_min_max() {
        let s = AutoWidthStrategy {
            min_width: 100,
            max_width: 5000,
        };
        assert_eq!(s.calculate_width(0), 100);
        assert_eq!(s.calculate_width(100), 5000);
    }
}
