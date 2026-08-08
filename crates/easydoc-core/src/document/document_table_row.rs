use crate::DocumentTableCell;

/// 语义表格中的一行。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentTableRow {
    /// 行内单元格。
    pub cells: Vec<DocumentTableCell>,
    /// 是否为表头行。
    pub is_header: bool,
}
