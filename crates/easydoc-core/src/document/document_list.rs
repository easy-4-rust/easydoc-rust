use crate::DocumentListItem;

/// 有序或无序的语义列表。
///
/// 对应 OOXML `<w:numPr>` 编号属性和 `<w:abstractNum>` 编号定义。
/// 无直接 Java 对应，是 easydoc-rust 自创的语义模型。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentList {
    /// 是否为有序列表。
    pub ordered: bool,
    /// 有序列表起始编号。
    pub start_number: Option<u32>,
    /// 列表项。
    pub items: Vec<DocumentListItem>,
}
