//! 同步读取文档纯文本。

use std::path::Path;

use easydoc_core::Result;

/// 同步读取文档中的所有纯文本。
///
/// 通过 `office_oxide` 自动检测 DOCX/DOC 格式。
///
/// 对应 Java: `EasyExcel.read(path).sheet().doReadSync()` 的文本提取简化版
///
/// # Errors
///
/// 文件无法读取时返回 I/O 或格式错误。
pub fn read_text(path: &Path) -> Result<String> {
    crate::extractor::text::extract_text(path)
}

/// 从内存字节同步读取文档纯文本（不触碰文件系统）。
///
/// 通过 magic bytes 自动检测 DOCX/DOC 格式。适用于 fuzz 目标、
/// 网络/流式数据源等无文件路径的场景。
///
/// # Errors
///
/// 字节不是受支持的文档格式或解析失败时返回格式错误。
pub fn read_text_from_bytes(bytes: &[u8]) -> Result<String> {
    crate::extractor::text::extract_text_from_bytes(bytes)
}
