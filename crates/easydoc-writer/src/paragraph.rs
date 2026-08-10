//! 段落构建器。
//!
//! 对应 Java: `com.alibaba.excel.write.metadata.Row` 中的单元格文本内容

use easydoc_core::{HorizontalAlignment, ParagraphStyle};

use crate::run::Run;

/// 由文本片段组成的段落。
///
/// 对应 Java: `com.alibaba.excel.write.metadata.Row` 中的单元格文本内容
#[derive(Clone)]
pub struct Paragraph {
    runs: Vec<Run>,
    style: Option<ParagraphStyle>,
}

impl Paragraph {
    /// 创建空段落。
    #[must_use]
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            style: None,
        }
    }

    /// 向段落添加纯文本。
    #[must_use]
    pub fn add_text(mut self, text: impl Into<String>) -> Self {
        self.runs.push(Run::text(text));
        self
    }

    /// 向段落添加预配置的 [`Run`]。
    #[must_use]
    pub fn add_run(mut self, run: Run) -> Self {
        self.runs.push(run);
        self
    }

    /// 设置段落对齐方式。
    #[must_use]
    pub fn alignment(mut self, alignment: HorizontalAlignment) -> Self {
        self.style.get_or_insert_default().alignment = Some(alignment);
        self
    }

    pub(crate) fn into_runs(self) -> Vec<Run> {
        self.runs
    }

    pub(crate) fn paragraph_style(&self) -> Option<&ParagraphStyle> {
        self.style.as_ref()
    }
}

impl Default for Paragraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_new_is_empty() {
        let p = Paragraph::new();
        assert!(p.runs.is_empty());
        assert!(p.style.is_none());
    }

    #[test]
    fn paragraph_add_text() {
        let p = Paragraph::new().add_text("hello");
        assert_eq!(p.runs.len(), 1);
        assert_eq!(p.runs[0].run_text(), "hello");
    }

    #[test]
    fn paragraph_add_run() {
        let run = Run::new("bold").bold();
        let p = Paragraph::new().add_run(run);
        assert_eq!(p.runs.len(), 1);
        assert!(p.runs[0].font_config().unwrap().bold);
    }

    #[test]
    fn paragraph_alignment() {
        let p = Paragraph::new().alignment(HorizontalAlignment::Center);
        assert!(p.paragraph_style().is_some());
        assert_eq!(
            p.paragraph_style().unwrap().alignment,
            Some(HorizontalAlignment::Center)
        );
    }
}
