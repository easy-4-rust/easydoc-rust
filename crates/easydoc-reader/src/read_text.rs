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
