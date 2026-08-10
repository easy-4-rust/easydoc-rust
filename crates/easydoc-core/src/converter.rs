//! 内置类型转换器与转换器注册表。
//!
//! 对标 easyexcel-core 的 `converter/` 模块。
//!
//! 对应 Java: com.alibaba.excel.converters

pub mod mod_file;
mod registry;

pub use registry::{ConverterRegistry, ErasedConverter};

// Re-export the converter trait so users see it from here.
pub use crate::traits::DocConverter as Converter;
