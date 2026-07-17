//! Document, paragraph, and text style definitions.

use crate::{Length, Pt};

/// Paragraph alignment.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Alignment {
    /// Align to the leading edge.
    #[default]
    Left,
    /// Centre the paragraph.
    Center,
    /// Align to the trailing edge.
    Right,
    /// Justify both edges.
    Justified,
}

/// Font slots used by `WordprocessingML`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FontFamily {
    /// Font used for ASCII characters.
    pub ascii: Option<String>,
    /// Font used for East Asian characters.
    pub east_asia: Option<String>,
    /// Font used for high ANSI characters.
    pub high_ansi: Option<String>,
    /// Font used for complex scripts.
    pub complex_script: Option<String>,
}

impl FontFamily {
    /// Uses one family for every Word font slot.
    #[must_use]
    pub fn all(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            ascii: Some(name.clone()),
            east_asia: Some(name.clone()),
            high_ansi: Some(name.clone()),
            complex_script: Some(name),
        }
    }
}

/// Character-level formatting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyle {
    /// Font family slots.
    pub font: Option<FontFamily>,
    /// Font size.
    pub size: Option<Pt>,
    /// RGB colour without a leading hash.
    pub color: Option<String>,
    /// Bold state.
    pub bold: Option<bool>,
    /// Italic state.
    pub italic: Option<bool>,
    /// Underline state.
    pub underline: Option<bool>,
}

impl TextStyle {
    /// Applies local values over inherited values.
    #[must_use]
    pub fn overlay(&self, local: &Self) -> Self {
        Self {
            font: local.font.clone().or_else(|| self.font.clone()),
            size: local.size.or(self.size),
            color: local.color.clone().or_else(|| self.color.clone()),
            bold: local.bold.or(self.bold),
            italic: local.italic.or(self.italic),
            underline: local.underline.or(self.underline),
        }
    }

    /// Sets the font family.
    #[must_use]
    pub fn font(mut self, family: impl Into<String>) -> Self {
        self.font = Some(FontFamily::all(family));
        self
    }

    /// Sets the font size.
    #[must_use]
    pub const fn size(mut self, size: Pt) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets the RGB colour.
    #[must_use]
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into().trim_start_matches('#').to_uppercase());
        self
    }

    /// Enables bold formatting.
    #[must_use]
    pub const fn bold(mut self) -> Self {
        self.bold = Some(true);
        self
    }

    /// Enables italic formatting.
    #[must_use]
    pub const fn italic(mut self) -> Self {
        self.italic = Some(true);
        self
    }

    /// Enables single underline formatting.
    #[must_use]
    pub const fn underline(mut self) -> Self {
        self.underline = Some(true);
        self
    }
}

/// Paragraph-level formatting.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphStyle {
    /// Paragraph alignment.
    pub alignment: Option<Alignment>,
    /// Leading indentation.
    pub left_indent: Option<Length>,
    /// First-line indentation.
    pub first_line_indent: Option<Length>,
    /// Keep this paragraph with the next one.
    pub keep_next: Option<bool>,
    /// Character defaults for runs in this paragraph.
    pub text: TextStyle,
}

impl ParagraphStyle {
    /// Applies local values over inherited values.
    #[must_use]
    pub fn overlay(&self, local: &Self) -> Self {
        Self {
            alignment: local.alignment.or(self.alignment),
            left_indent: local.left_indent.or(self.left_indent),
            first_line_indent: local.first_line_indent.or(self.first_line_indent),
            keep_next: local.keep_next.or(self.keep_next),
            text: self.text.overlay(&local.text),
        }
    }

    /// Sets paragraph alignment.
    #[must_use]
    pub const fn align(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// Sets character defaults for the paragraph.
    #[must_use]
    pub fn text(mut self, style: TextStyle) -> Self {
        self.text = style;
        self
    }
}

/// A named document style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    /// Paragraph-level values.
    pub paragraph: ParagraphStyle,
}

impl Style {
    /// Creates a paragraph style.
    #[must_use]
    pub fn paragraph(style: ParagraphStyle) -> Self {
        Self { paragraph: style }
    }
}

/// Page dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageSize {
    /// Page width.
    pub width: Length,
    /// Page height.
    pub height: Length,
}

impl PageSize {
    /// ISO A4 portrait page.
    pub const A4: Self = Self {
        width: Length::from_twips(11_906),
        height: Length::from_twips(16_838),
    };
}

/// Page margins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Margins {
    /// Top margin.
    pub top: Length,
    /// Right margin.
    pub right: Length,
    /// Bottom margin.
    pub bottom: Length,
    /// Left margin.
    pub left: Length,
}

impl Margins {
    /// Creates four equal margins.
    #[must_use]
    pub fn all(value: Length) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

impl Default for Margins {
    fn default() -> Self {
        Self::all(Length::mm(25.4))
    }
}

/// Document-wide defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentConfig {
    /// Default font family.
    pub default_font: FontFamily,
    /// Default text size.
    pub default_font_size: Pt,
    /// Page size.
    pub page_size: PageSize,
    /// Page margins.
    pub margins: Margins,
}

impl Default for DocumentConfig {
    fn default() -> Self {
        Self {
            default_font: FontFamily::all("Calibri"),
            default_font_size: Pt::default(),
            page_size: PageSize::A4,
            margins: Margins::default(),
        }
    }
}
