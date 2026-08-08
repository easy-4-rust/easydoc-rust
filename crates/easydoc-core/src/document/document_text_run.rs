/// 带常用富文本属性的文本片段。
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
