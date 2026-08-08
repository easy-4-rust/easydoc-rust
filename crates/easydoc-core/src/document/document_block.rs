use crate::{DocumentImage, DocumentList, DocumentTable, DocumentTextRun};

/// 文档中的后端无关块级元素。
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DocumentBlock {
    /// 标题及其级别。
    Heading {
        /// 标题级别，范围一到六。
        level: u8,
        /// 标题富文本片段。
        runs: Vec<DocumentTextRun>,
    },
    /// 普通段落。
    Paragraph(Vec<DocumentTextRun>),
    /// 表格。
    Table(DocumentTable),
    /// 列表。
    List(DocumentList),
    /// 图片。
    Image(DocumentImage),
    /// 水平分隔线。
    ThematicBreak,
    /// 强制分页。
    PageBreak,
    /// 强制分栏。
    ColumnBreak,
    /// 预格式化代码块。
    CodeBlock {
        /// 可选语言标记。
        language: Option<String>,
        /// 代码文本。
        code: String,
    },
    /// 文本框中的块。
    TextBox(Vec<DocumentBlock>),
    /// 脚注。
    Footnote {
        /// 脚注标识。
        id: u32,
        /// 脚注内容。
        blocks: Vec<DocumentBlock>,
    },
    /// 尾注。
    Endnote {
        /// 尾注标识。
        id: u32,
        /// 尾注内容。
        blocks: Vec<DocumentBlock>,
    },
}
