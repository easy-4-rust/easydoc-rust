//! Auto-column-width strategy.

/// Strategy for automatic column width calculation.
///
/// Calculates column widths based on the longest cell content in each column.
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
