//! Document style types — font, paragraph, table, and color configuration.
//!
//! These types provide a backend-agnostic style model that maps to
//! both `docx-rs` (write) and `office_oxide` (read) representations.

/// 24-bit RGB color.
pub mod color;
/// Font configuration for text runs.
pub mod font;
/// Paragraph-level formatting style.
pub mod paragraph;
/// Table-level formatting style.
pub mod table;

pub use color::Color;
pub use font::FontConfig;
pub use paragraph::ParagraphStyle;
pub use table::TableStyle;
