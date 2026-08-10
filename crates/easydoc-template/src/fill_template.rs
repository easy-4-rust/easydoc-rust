//! 标量占位符模板填充。

use std::collections::HashMap;
use std::path::Path;

use easydoc_core::Result;

/// Fills scalar `{key}` placeholders in a DOCX template.
///
/// # Errors
///
/// Returns I/O or template-processing errors.
#[allow(clippy::implicit_hasher)]
pub fn fill_template(template: &Path, output: &Path, data: &HashMap<String, String>) -> Result<()> {
    crate::fill_executor::fill_scalar(template, output, data)
}
