//! 流式文档读取器构建器。

use std::path::PathBuf;

use crate::extractor;
use easydoc_core::{DocxRow, Result};

/// 流式文档读取的 Fluent 构建器。
///
/// 通过门面 `EasyDoc::read()` 方法创建。
///
/// 对应 Java: `EasyExcel.read(path).head(RowClass.class)`
pub struct DocReadBuilder {
    path: PathBuf,
}

impl DocReadBuilder {
    /// 创建新的读取器构建器。
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 执行同步读取，返回所有表格展平后的 `Vec<T>`。
    ///
    /// 使用 `office_oxide` 进行后端解析。
    ///
    /// 对应 Java: `EasyExcel.read(path).head(RowClass.class).sheet().doReadSync()`
    ///
    /// # Errors
    ///
    /// 返回 I/O、格式或转换错误。
    pub fn do_read<T: DocxRow>(self) -> Result<Vec<T>> {
        let tables: Vec<Vec<T>> = extractor::table::extract_tables::<T>(&self.path)?;
        Ok(tables.into_iter().flatten().collect())
    }
}
