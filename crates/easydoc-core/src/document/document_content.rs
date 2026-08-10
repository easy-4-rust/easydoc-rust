use crate::{DocumentBlock, DocumentMeta};

/// 与 DOC/DOCX 解析器实现无关的完整语义文档。
///
/// 无直接 Java 对应（Java `EasyExcel` 不处理 DOCX），是 easydoc-rust 自创的语义模型。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentContent {
    /// 文档元数据。
    pub metadata: DocumentMeta,
    /// 按源文档顺序排列的块级内容。
    pub blocks: Vec<DocumentBlock>,
}
