//! 同步读取文档表格。

use std::path::Path;

use easydoc_core::{DocxRow, Result};

/// 同步读取文档中的所有表格，将每行反序列化为 `Vec<T>`。
///
/// 对应 Java: `EasyExcel.read(path).head(RowClass.class).sheet().doReadSync()`
///
/// # Errors
///
/// 返回 I/O、格式或转换错误。
pub fn read_tables<T: DocxRow>(path: &Path) -> Result<Vec<Vec<T>>> {
    crate::extractor::table::extract_tables::<T>(path)
}
