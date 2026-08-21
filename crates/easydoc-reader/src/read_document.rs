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

/// 从内存字节读取 DOC/DOCX 语义文档模型（不触碰文件系统）。
///
/// 通过 magic bytes 自动检测格式。适用于 fuzz 目标、网络/流式数据源等
/// 无文件路径的场景。
///
/// # Errors
///
/// 字节不是受支持的文档格式或解析失败时返回错误。
pub fn read_document_from_bytes(bytes: &[u8]) -> Result<DocumentContent> {
    crate::extractor::semantic::extract_document_from_bytes(bytes)
}
