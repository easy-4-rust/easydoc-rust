//! DOCX template fill engine for `easydoc-rs`.
//!
//! Detects `{key}` and `{.field}` placeholders in DOCX documents and
//! replaces them with provided data — analogous to `easyexcel-template`.

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::path::Path;

use easydoc_core::Result;

mod fill_config;
mod fill_executor;
mod placeholder;

pub use fill_config::FillConfig;
pub use fill_config::FillDirection;
pub use fill_executor::TemplateFillBuilder;
pub use placeholder::Placeholder;

/// Fills scalar `{key}` placeholders in a DOCX template.
///
/// # Errors
///
/// Returns I/O or template-processing errors.
#[allow(clippy::implicit_hasher)]
pub fn fill_template(template: &Path, output: &Path, data: &HashMap<String, String>) -> Result<()> {
    fill_executor::fill_scalar(template, output, data)
}

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
    fill_executor::fill_list(template, output, data, list_field)
}
