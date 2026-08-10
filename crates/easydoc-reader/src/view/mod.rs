//! 面向 LLM 的文档视图模式。
//!
//! 提供四种视图模式，将 [`DocumentContent`] 转换为针对不同场景优化的纯文本表示：
//!
//! - [`ViewMode::Plain`] -- 纯文本，段落以换行分隔。
//! - [`ViewMode::Annotated`] -- 结构化标注（`[段落 3]`、`[表格 2: 3行x4列]` 等）。
//! - [`ViewMode::Outline`] -- 仅标题，Markdown 风格 `#` / `##`。
//! - [`ViewMode::Stats`] -- 块计数和字数统计。
//!
//! 无直接 Java 对应，是 easydoc-rust 自创的辅助功能。

mod annotated;
mod outline;
mod plain;
mod stats;

use easydoc_core::{DocumentContent, Result};

/// Selects how a [`DocumentContent`] is rendered into a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewMode {
    /// Bare text: paragraphs joined by newlines, tables by commas.
    Plain,
    /// Annotated with structural markers (e.g. `[段落 3]`, `[表格 1: 2行x3列]`).
    Annotated,
    /// Headings only, Markdown-style.
    Outline {
        /// Maximum heading level to include (1--6). Levels deeper than this are
        /// omitted.
        max_level: u8,
    },
    /// Aggregate statistics: paragraph / table / image / word counts.
    Stats,
}

/// Renders a [`DocumentContent`] into a string according to the chosen
/// [`ViewMode`].
///
/// # Errors
///
/// Returns an error if rendering fails (currently infallible, but the signature
/// allows for future fallible rendering).
pub fn render_view(content: &DocumentContent, mode: &ViewMode) -> Result<String> {
    match mode {
        ViewMode::Plain => Ok(plain::render(content)),
        ViewMode::Annotated => Ok(annotated::render(content)),
        ViewMode::Outline { max_level } => Ok(outline::render(content, *max_level)),
        ViewMode::Stats => Ok(stats::render(content)),
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use easydoc_core::{
        DocumentBlock, DocumentTable, DocumentTableCell, DocumentTableRow, DocumentTextRun,
    };

    fn sample_content() -> DocumentContent {
        DocumentContent {
            blocks: vec![
                DocumentBlock::Heading {
                    level: 1,
                    runs: vec![DocumentTextRun {
                        text: "Title".into(),
                        ..DocumentTextRun::default()
                    }],
                },
                DocumentBlock::Paragraph(vec![DocumentTextRun {
                    text: "Hello World".into(),
                    ..DocumentTextRun::default()
                }]),
                DocumentBlock::Table(DocumentTable {
                    rows: vec![DocumentTableRow {
                        cells: vec![
                            DocumentTableCell {
                                blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                                    text: "A".into(),
                                    ..DocumentTextRun::default()
                                }])],
                                column_span: 1,
                                row_span: 1,
                            },
                            DocumentTableCell {
                                blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                                    text: "B".into(),
                                    ..DocumentTextRun::default()
                                }])],
                                column_span: 1,
                                row_span: 1,
                            },
                        ],
                        is_header: true,
                    }],
                }),
            ],
            ..DocumentContent::default()
        }
    }

    #[test]
    fn render_plain() {
        let content = sample_content();
        let result = render_view(&content, &ViewMode::Plain).unwrap();
        assert!(result.contains("Title"));
        assert!(result.contains("Hello World"));
    }

    #[test]
    fn render_annotated() {
        let content = sample_content();
        let result = render_view(&content, &ViewMode::Annotated).unwrap();
        assert!(result.contains("[标题1]"));
        assert!(result.contains("[段落"));
        assert!(result.contains("[表格"));
    }

    #[test]
    fn render_outline() {
        let content = sample_content();
        let result = render_view(&content, &ViewMode::Outline { max_level: 3 }).unwrap();
        assert!(result.contains("# Title"));
    }

    #[test]
    fn render_stats() {
        let content = sample_content();
        let result = render_view(&content, &ViewMode::Stats).unwrap();
        assert!(result.contains("段落数:"));
        assert!(result.contains("表格数:"));
    }

    #[test]
    fn view_mode_debug_clone_eq() {
        let mode = ViewMode::Annotated;
        let mode2 = mode.clone();
        assert_eq!(mode, mode2);
        assert!(format!("{mode:?}").contains("Annotated"));
    }

    #[test]
    fn outline_max_level_filters() {
        let content = DocumentContent {
            blocks: vec![
                DocumentBlock::Heading {
                    level: 1,
                    runs: vec![DocumentTextRun {
                        text: "H1".into(),
                        ..DocumentTextRun::default()
                    }],
                },
                DocumentBlock::Heading {
                    level: 2,
                    runs: vec![DocumentTextRun {
                        text: "H2".into(),
                        ..DocumentTextRun::default()
                    }],
                },
                DocumentBlock::Heading {
                    level: 3,
                    runs: vec![DocumentTextRun {
                        text: "H3".into(),
                        ..DocumentTextRun::default()
                    }],
                },
            ],
            ..DocumentContent::default()
        };
        let result = render_view(&content, &ViewMode::Outline { max_level: 2 }).unwrap();
        assert!(result.contains("# H1"));
        assert!(result.contains("## H2"));
        assert!(!result.contains("H3"));
    }
}
