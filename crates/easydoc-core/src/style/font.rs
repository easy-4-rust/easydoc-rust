use super::color::Color;

/// Font configuration for text runs.
///
/// Used by both paragraph text and table cell content.
#[derive(Debug, Clone)]
pub struct FontConfig {
    /// Font family name (e.g. "Arial", "Times New Roman", "宋体").
    pub name: Option<String>,
    /// Font size in half-points (e.g. 24 = 12pt).
    pub size: Option<u32>,
    /// Whether the text is bold.
    pub bold: bool,
    /// Whether the text is italic.
    pub italic: bool,
    /// Whether the text is underlined.
    pub underline: bool,
    /// Text color. `None` means auto/inherit.
    pub color: Option<Color>,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            name: None,
            size: Some(22), // 11pt default
            bold: false,
            italic: false,
            underline: false,
            color: Some(Color::BLACK),
        }
    }
}

impl FontConfig {
    /// Creates default font configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a bold font.
    #[must_use]
    pub fn bold() -> Self {
        Self {
            bold: true,
            ..Default::default()
        }
    }

    /// Creates a font for table headers (bold, white on blue).
    #[must_use]
    pub fn header() -> Self {
        Self {
            bold: true,
            size: Some(22),
            color: Some(Color::WHITE),
            ..Default::default()
        }
    }

    /// Sets the font family.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the font size in half-points.
    #[must_use]
    pub fn size(mut self, size: u32) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets bold on or off.
    #[must_use]
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Sets italic on or off.
    #[must_use]
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Sets underline on or off.
    #[must_use]
    pub fn with_underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    /// Sets the text color.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}
