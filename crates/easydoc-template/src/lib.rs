//! DOCX 模板填充引擎。
//!
//! 检测 DOCX 文档中的 `{key}` 和 `{.field}` 占位符，并用提供的数据替换。
//!
//! 对应 Java: com.alibaba.excel (easyexcel-template) 的模板填充功能

#![deny(unsafe_code)]

mod fill_config;
mod fill_executor;
mod fill_template;
mod fill_template_list;
mod placeholder;

pub use fill_config::FillConfig;
pub use fill_config::FillDirection;
pub use fill_executor::TemplateFillBuilder;
pub use fill_template::fill_template;
pub use fill_template_list::fill_template_list;
pub use placeholder::Placeholder;
