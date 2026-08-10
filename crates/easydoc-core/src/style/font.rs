use super::color::Color;

/// 文本片段的字体配置。
///
/// 用于段落文本和表格单元格内容。
///
/// 对应 Java: `com.alibaba.excel.write.metadata.style.WriteFont`
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_config() {
        let f = FontConfig::default();
        assert!(!f.bold);
        assert!(!f.italic);
        assert!(!f.underline);
        assert_eq!(f.size, Some(22));
        assert_eq!(f.color, Some(Color::BLACK));
        assert!(f.name.is_none());
    }

    #[test]
    fn new_equals_default() {
        assert_eq!(FontConfig::new().size, FontConfig::default().size);
    }

    #[test]
    fn bold_font() {
        let f = FontConfig::bold();
        assert!(f.bold);
        assert!(!f.italic);
    }

    #[test]
    fn header_font() {
        let f = FontConfig::header();
        assert!(f.bold);
        assert_eq!(f.color, Some(Color::WHITE));
        assert_eq!(f.size, Some(22));
    }

    #[test]
    fn builder_chain() {
        let f = FontConfig::new()
            .name("Arial")
            .size(28)
            .with_bold(true)
            .with_italic(true)
            .with_underline(true)
            .color(Color::RED);
        assert_eq!(f.name.as_deref(), Some("Arial"));
        assert_eq!(f.size, Some(28));
        assert!(f.bold);
        assert!(f.italic);
        assert!(f.underline);
        assert_eq!(f.color, Some(Color::RED));
    }
}
