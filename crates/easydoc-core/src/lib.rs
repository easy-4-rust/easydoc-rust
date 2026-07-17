//! Backend-neutral document model for easydoc-rs.

mod error;
mod model;
mod style;
mod units;

pub use error::{Error, Result};
pub use model::{Block, Cell, Document, Image, Inline, Paragraph, Row, Table, TextRun};
pub use style::{
    Alignment, DocumentConfig, FontFamily, Margins, PageSize, ParagraphStyle, Style, TextStyle,
};
pub use units::{Length, Pt, Px};
