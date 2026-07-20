//! Built-in type converters and the converter registry.
//!
//! Analogous to `converter/` in `easyexcel-core`.

pub mod mod_file;
mod registry;

pub use registry::ConverterRegistry;

// Re-export the converter trait so users see it from here.
pub use crate::traits::DocConverter as Converter;
