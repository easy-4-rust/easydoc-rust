//! 视图模式枚举定义。

/// 选择如何将 [`DocumentContent`](easydoc_core::DocumentContent) 渲染为字符串。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ViewMode {
    /// 纯文本：段落以换行连接，表格以逗号分隔。
    Plain,
    /// 结构化标注（例如 `[段落 3]`、`[表格 1: 2行x3列]`）。
    Annotated,
    /// 仅标题，Markdown 风格。
    Outline {
        /// 要包含的最大标题级别（1--6）。超过此级别的标题将被省略。
        max_level: u8,
    },
    /// 聚合统计：段落/表格/图片/字数计数。
    Stats,
}
