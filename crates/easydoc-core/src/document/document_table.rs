use crate::DocumentTableRow;

/// 保留表头和合并单元格信息的语义表格。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocumentTable {
    /// 表格行。
    pub rows: Vec<DocumentTableRow>,
}
