//! Document style types — font, paragraph, table, and color configuration.
//!
//! These types provide a backend-agnostic style model that maps to
//! both `docx-rs` (write) and `office_oxide` (read) representations.

pub mod color;
pub mod font;
pub mod paragraph;
pub mod table;

pub use color::Color;
pub use font::FontConfig;
pub use paragraph::ParagraphStyle;
pub use table::TableStyle;
