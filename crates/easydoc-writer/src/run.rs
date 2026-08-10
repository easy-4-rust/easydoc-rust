//! 段落内的格式化文本片段。
//!
//! 对应 Java: `com.alibaba.excel.write.metadata.Cell` 中的文本内容

use easydoc_core::{Color, FontConfig};

/// 段落内的格式化文本片段。
///
/// 对应 Java: `com.alibaba.excel.write.metadata.Cell` 中的文本内容
#[derive(Clone)]
pub struct Run {
    text: String,
    font: Option<FontConfig>,
}

impl Run {
    /// 创建包含纯文本的文本片段。
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font: None,
        }
    }

    /// 创建包含纯文本的文本片段（别名）。
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(text)
    }

    /// 设置为粗体。
    #[must_use]
    pub fn bold(mut self) -> Self {
        self.font.get_or_insert_default().bold = true;
        self
    }

    /// 设置为斜体。
    #[must_use]
    pub fn italic(mut self) -> Self {
        self.font.get_or_insert_default().italic = true;
        self
    }

    /// 设置字号（半磅单位，例如 24 = 12pt）。
    #[must_use]
    pub fn size(mut self, size: u32) -> Self {
        self.font.get_or_insert_default().size = Some(size);
        self
    }

    /// 设置文字颜色。
    #[must_use]
    pub fn color(mut self, hex: u32) -> Self {
        self.font.get_or_insert_default().color = Some(Color::from_hex(hex));
        self
    }

    /// 设置字体族。
    #[must_use]
    pub fn font(mut self, name: impl Into<String>) -> Self {
        self.font.get_or_insert_default().name = Some(name.into());
        self
    }

    /// 添加下划线。
    #[must_use]
    pub fn underline(mut self) -> Self {
        self.font.get_or_insert_default().underline = true;
        self
    }

    pub(crate) fn run_text(&self) -> &str {
        &self.text
    }

    pub(crate) fn font_config(&self) -> Option<&FontConfig> {
        self.font.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_text_constructor() {
        let r = Run::text("test");
        assert_eq!(r.run_text(), "test");
        assert!(r.font_config().is_none());
    }

    #[test]
    fn run_builder_chain() {
        let r = Run::new("styled")
            .bold()
            .italic()
            .size(28)
            .color(0xFF0000)
            .font("Arial")
            .underline();
        let font = r.font_config().unwrap();
        assert!(font.bold);
        assert!(font.italic);
        assert_eq!(font.size, Some(28));
        assert_eq!(font.color, Some(Color::from_hex(0xFF0000)));
        assert_eq!(font.name.as_deref(), Some("Arial"));
        assert!(font.underline);
    }
}
