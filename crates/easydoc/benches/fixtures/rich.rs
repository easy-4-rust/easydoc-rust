//! Fixture 4：富文本（粗体、斜体、下划线、颜色、字号）。

use easydoc::EasyDoc;
use easydoc::prelude::{Paragraph, Run};

use super::types::FidelityFixture;

/// 构建富文本 fixture。
pub(super) fn build() -> FidelityFixture {
    let bytes = EasyDoc::document_to_bytes(|doc| {
        doc.title("Rich Text Fixture")
            .add_paragraph(
                Paragraph::new()
                    .add_run(Run::new("Bold text").bold())
                    .add_run(Run::new(" and "))
                    .add_run(Run::new("italic text").italic())
                    .add_run(Run::new(" and "))
                    .add_run(Run::new("underlined").underline())
                    .add_run(Run::new(" and "))
                    .add_run(Run::new("colored red").color(0xFF_0000))
                    .add_run(Run::new(" and "))
                    .add_run(Run::new("large size").size(36)),
            )
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new("All styles combined")
                        .bold()
                        .italic()
                        .underline()
                        .color(0x00_00FF)
                        .size(28)
                        .font("Arial"),
                ),
            )
    })
    .expect("build rich fixture");

    let expected = super::types::Fixtures::roundtrip_text(&bytes);

    FidelityFixture {
        name: "rich",
        original_size: bytes.len() as u64,
        expected_text: expected,
        docx_bytes: bytes,
    }
}
