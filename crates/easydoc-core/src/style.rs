//! 文档样式类型 -- 字体、段落、表格和颜色配置。
//!
//! 这些类型提供与后端无关的样式模型，可映射到 `docx-rs`（写入）和
//! `office_oxide`（读取）的表示。
//!
//! 对应 Java: `com.alibaba.excel.write.metadata.style` (`WriteCellStyle` / `WriteFont`)

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
