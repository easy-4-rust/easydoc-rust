//! Math conversion re-exports.
//!
//! 兼容层：历史路径 `easydoc_markdown::math::*` 由独立的 `easydoc-math` crate
//! 提供，此处仅做 re-export，避免破坏既有调用方。

pub use easydoc_math::{latex_dict, omml_to_latex};
