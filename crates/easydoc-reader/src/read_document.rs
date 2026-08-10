//! 读取语义文档模型。

use std::path::Path;

use easydoc_core::{DocumentContent, Result};

/// 读取 DOC 或 DOCX，并转换为不暴露底层解析器类型的语义文档。
///
/// # Errors
///
/// 文件无法打开或解析时返回错误。
pub fn read_document(path: &Path) -> Result<DocumentContent> {
    crate::extractor::semantic::extract_document(path)
}
