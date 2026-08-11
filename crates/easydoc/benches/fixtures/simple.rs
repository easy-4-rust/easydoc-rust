//! Fixture 1：简单文本（1 个标题 + 3 个段落）。

use easydoc::EasyDoc;
use easydoc::prelude::{HeadingLevel, Paragraph};

use super::types::FidelityFixture;

/// 构建简单文本 fixture。
pub(super) fn build() -> FidelityFixture {
    let bytes = EasyDoc::document_to_bytes(|doc| {
        doc.title("Simple Fixture")
            .add_heading("Introduction", HeadingLevel::H1)
            .add_paragraph(Paragraph::new().add_text("First paragraph of the simple document."))
            .add_paragraph(Paragraph::new().add_text("Second paragraph with additional content."))
            .add_paragraph(Paragraph::new().add_text("Third and final paragraph."))
    })
    .expect("build simple fixture");

    let expected = super::types::Fixtures::roundtrip_text(&bytes);

    FidelityFixture {
        name: "simple",
        original_size: bytes.len() as u64,
        expected_text: expected,
        docx_bytes: bytes,
    }
}
