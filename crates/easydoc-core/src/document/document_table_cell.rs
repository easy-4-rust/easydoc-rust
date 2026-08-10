use crate::DocumentBlock;

/// 支持嵌套块和跨行跨列的表格单元格。
///
/// 对应 OOXML `<w:tc>` 元素，包含 `<w:tcPr>` 属性（gridSpan / vMerge）。
/// 无直接 Java 对应，是 easydoc-rust 自创的语义模型。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentTableCell {
    /// 单元格中的块级内容。
    pub blocks: Vec<DocumentBlock>,
    /// 跨列数，最小为一。
    pub column_span: u32,
    /// 跨行数，最小为一。
    pub row_span: u32,
}
