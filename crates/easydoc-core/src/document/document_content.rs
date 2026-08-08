use crate::{DocumentBlock, DocumentMeta};

/// 与 DOC/DOCX 解析器实现无关的完整语义文档。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentContent {
    /// 文档元数据。
    pub metadata: DocumentMeta,
    /// 按源文档顺序排列的块级内容。
    pub blocks: Vec<DocumentBlock>,
}
