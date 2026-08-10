/// 带常用富文本属性的文本片段。
///
/// 对应 OOXML `<w:r>` 元素，包含 `<w:rPr>` 属性（粗体/斜体/删除线等）。
/// 无直接 Java 对应，是 easydoc-rust 自创的语义模型。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentTextRun {
    /// 原始文本。
    pub text: String,
    /// 是否加粗。
    pub bold: bool,
    /// 是否斜体。
    pub italic: bool,
    /// 是否删除线。
    pub strikethrough: bool,
    /// 可选超链接地址。
    pub hyperlink: Option<String>,
}
