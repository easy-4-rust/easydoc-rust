//! 集合占位符模板填充。

use std::path::Path;

use easydoc_core::Result;

/// Fills a DOCX template with collection data (`{.field}` placeholders).
///
/// # Errors
///
/// Returns I/O or template-processing errors.
pub fn fill_template_list<T: serde::Serialize + std::fmt::Debug>(
    template: &Path,
    output: &Path,
    data: &[T],
    list_field: &str,
) -> Result<()> {
    crate::fill_executor::fill_list(template, output, data, list_field)
}
