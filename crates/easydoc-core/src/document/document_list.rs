use crate::DocumentListItem;

/// 有序或无序的语义列表。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentList {
    /// 是否为有序列表。
    pub ordered: bool,
    /// 有序列表起始编号。
    pub start_number: Option<u32>,
    /// 列表项。
    pub items: Vec<DocumentListItem>,
}
