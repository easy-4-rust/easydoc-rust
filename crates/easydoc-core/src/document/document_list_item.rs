use crate::{DocumentBlock, DocumentList};

/// 列表项及其可选子列表。
///
/// 对应 OOXML `<w:numPr>` 中的单个编号段落。
/// 无直接 Java 对应，是 easydoc-rust 自创的语义模型。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentListItem {
    /// 列表项的块级内容。
    pub blocks: Vec<DocumentBlock>,
    /// 嵌套子列表。
    pub nested: Option<Box<DocumentList>>,
}
