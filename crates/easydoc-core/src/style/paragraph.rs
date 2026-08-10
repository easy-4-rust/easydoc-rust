use crate::types::HorizontalAlignment;

/// 段落级格式化。
///
/// 对应 Java: `com.alibaba.excel.write.metadata.style.WriteCellStyle` 中的段落属性
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_paragraph_style() {
        let s = ParagraphStyle::default();
        assert!(s.alignment.is_none());
        assert!(s.first_line_indent.is_none());
        assert!(s.space_after.is_none());
        assert!(s.line_spacing.is_none());
    }

    #[test]
    fn new_equals_default() {
        let s = ParagraphStyle::new();
        assert!(s.alignment.is_none());
    }

    #[test]
    fn builder_chain() {
        let s = ParagraphStyle::new()
            .alignment(HorizontalAlignment::Center)
            .first_line_indent(480)
            .space_after(200)
            .line_spacing(360);
        assert_eq!(s.alignment, Some(HorizontalAlignment::Center));
        assert_eq!(s.first_line_indent, Some(480));
        assert_eq!(s.space_after, Some(200));
        assert_eq!(s.line_spacing, Some(360));
    }
}
