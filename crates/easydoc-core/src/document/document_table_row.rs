use crate::DocumentTableCell;

/// 语义表格中的一行。
///
/// 对应 OOXML `<w:tr>` 元素。
/// 无直接 Java 对应，是 easydoc-rust 自创的语义模型。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentTableRow {
    /// 行内单元格。
    pub cells: Vec<DocumentTableCell>,
    /// 是否为表头行。
    pub is_header: bool,
}
