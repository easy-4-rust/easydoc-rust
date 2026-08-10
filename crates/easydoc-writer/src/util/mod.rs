//! Shared utilities for OOXML generation.
//!
//! Contains unit conversion (CSS-like widths to OOXML twips) and
//! XML post-processing helpers for attributes not natively supported
//! by `docx-rs` (e.g. `noWrap`, `numFmt`).

mod parse_width;
mod parsed_width;
mod xml_insert;

pub use parse_width::*;
pub use parsed_width::*;
pub use xml_insert::*;
