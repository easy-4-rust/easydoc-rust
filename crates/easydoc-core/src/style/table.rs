use super::color::Color;
use super::font::FontConfig;

/// Table-level formatting configuration.
#[derive(Debug, Clone)]
pub struct TableStyle {
    /// Style for the header row.
    pub header_font: FontConfig,
    /// Style for content rows.
    pub content_font: FontConfig,
    /// Background color for header cells.
    pub header_background: Option<Color>,
    /// Whether to apply alternating row colors (banded/zebra).
    pub banded_rows: bool,
    /// Background color for even rows (when banded).
    pub even_row_background: Option<Color>,
    /// Background color for odd rows (when banded).
    pub odd_row_background: Option<Color>,
    /// Whether to auto-fit column widths to content.
    pub auto_width: bool,
    /// Table border visibility.
    pub borders: bool,
}

impl Default for TableStyle {
    fn default() -> Self {
        Self {
            header_font: FontConfig::header(),
            content_font: FontConfig::default(),
            header_background: Some(Color::HEADER_BLUE),
            banded_rows: false,
            even_row_background: Some(Color::rgb(242, 242, 242)),
            odd_row_background: None, // white default
            auto_width: false,
            borders: true,
        }
    }
}

impl TableStyle {
    /// Creates a default table style.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a header-only style (bold white text on accent blue).
    #[must_use]
    pub fn header() -> Self {
        Self::default()
    }

    /// Creates a simple style without borders or banding.
    #[must_use]
    pub fn simple() -> Self {
        Self {
            borders: false,
            header_background: None,
            header_font: FontConfig::bold(),
            ..Default::default()
        }
    }

    /// Enables zebra striping.
    #[must_use]
    pub fn banded_rows(mut self, enabled: bool) -> Self {
        self.banded_rows = enabled;
        self
    }

    /// Enables auto column width.
    #[must_use]
    pub fn auto_width(mut self, enabled: bool) -> Self {
        self.auto_width = enabled;
        self
    }

    /// Enables table borders.
    #[must_use]
    pub fn borders(mut self, enabled: bool) -> Self {
        self.borders = enabled;
        self
    }

    /// Sets header background color.
    #[must_use]
    pub fn header_background(mut self, color: Color) -> Self {
        self.header_background = Some(color);
        self
    }
}
