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
    /// 文档分区（Section），包含页面布局属性和子块。
    Section {
        /// 分区内的块级内容。
        blocks: Vec<DocumentBlock>,
        /// 可选的分区类型标识（如 nextPage, continuous 等）。
        section_type: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_variant_roundtrip() {
        let section = DocumentBlock::Section {
            blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                text: "hello".into(),
                ..DocumentTextRun::default()
            }])],
            section_type: Some("nextPage".into()),
        };
        match &section {
            DocumentBlock::Section {
                blocks,
                section_type,
            } => {
                assert_eq!(blocks.len(), 1);
                assert_eq!(section_type.as_deref(), Some("nextPage"));
            }
            _ => panic!("expected Section"),
        }
    }

    #[test]
    fn document_block_is_non_exhaustive() {
        // Verify _ => wildcard works for forward compat
        let block = DocumentBlock::ThematicBreak;
        let desc = match block {
            DocumentBlock::Heading { .. } => "heading",
            DocumentBlock::Paragraph(_) => "paragraph",
            DocumentBlock::Table(_) => "table",
            DocumentBlock::List(_) => "list",
            DocumentBlock::Image(_) => "image",
            DocumentBlock::ThematicBreak => "break",
            DocumentBlock::PageBreak => "page",
            DocumentBlock::ColumnBreak => "column",
            DocumentBlock::CodeBlock { .. } => "code",
            DocumentBlock::TextBox(_) => "textbox",
            DocumentBlock::Footnote { .. } => "footnote",
            DocumentBlock::Endnote { .. } => "endnote",
            DocumentBlock::Section { .. } => "section",
            _ => "unknown",
        };
        assert_eq!(desc, "break");
    }
}
