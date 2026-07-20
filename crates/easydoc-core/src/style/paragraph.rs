use crate::types::HorizontalAlignment;

/// Paragraph-level formatting.
#[derive(Debug, Clone, Default)]
pub struct ParagraphStyle {
    /// Horizontal text alignment.
    pub alignment: Option<HorizontalAlignment>,
    /// First-line indent in twips.
    pub first_line_indent: Option<i32>,
    /// Left indent in twips.
    pub left_indent: Option<i32>,
    /// Right indent in twips.
    pub right_indent: Option<i32>,
    /// Space before paragraph in twips.
    pub space_before: Option<u32>,
    /// Space after paragraph in twips.
    pub space_after: Option<u32>,
    /// Line spacing (e.g. 240 = single, 360 = 1.5, 480 = double).
    pub line_spacing: Option<u32>,
}

impl ParagraphStyle {
    /// Creates a new paragraph style with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets horizontal alignment.
    #[must_use]
    pub fn alignment(mut self, alignment: HorizontalAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// Sets first-line indent.
    #[must_use]
    pub fn first_line_indent(mut self, indent: i32) -> Self {
        self.first_line_indent = Some(indent);
        self
    }

    /// Sets spacing after the paragraph.
    #[must_use]
    pub fn space_after(mut self, space: u32) -> Self {
        self.space_after = Some(space);
        self
    }

    /// Sets line spacing in twips (240 = single, 360 = 1.5, 480 = double).
    #[must_use]
    pub fn line_spacing(mut self, spacing: u32) -> Self {
        self.line_spacing = Some(spacing);
        self
    }
}
