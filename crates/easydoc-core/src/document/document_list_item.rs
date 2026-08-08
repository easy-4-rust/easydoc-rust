use crate::{DocumentBlock, DocumentList};

/// 列表项及其可选子列表。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentListItem {
    /// 列表项的块级内容。
    pub blocks: Vec<DocumentBlock>,
    /// 嵌套子列表。
    pub nested: Option<Box<DocumentList>>,
}
