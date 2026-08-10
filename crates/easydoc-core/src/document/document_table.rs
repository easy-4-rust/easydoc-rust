use crate::DocumentTableRow;

/// 保留表头和合并单元格信息的语义表格。
///
/// 对应 OOXML `<w:tbl>` 元素，包含 `<w:tblGrid>` 列定义和 `<w:tr>` 行。
/// 无直接 Java 对应，是 easydoc-rust 自创的语义模型。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentTable {
    /// 表格行。
    pub rows: Vec<DocumentTableRow>,
}
