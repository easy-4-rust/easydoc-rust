//! Fixture 5：嵌入 1x1 红色 PNG 图片。

use easydoc::EasyDoc;
use easydoc::prelude::{HeadingLevel, Paragraph};

use super::png::create_red_png;
use super::types::FidelityFixture;

/// 构建图片 fixture。
pub(super) fn build() -> FidelityFixture {
    let png_bytes = create_red_png();

    // 将 PNG 写入临时文件以便 DocImage 读取。
    let tmp_dir = tempfile::tempdir().expect("temp dir for image fixture");
    let png_path = tmp_dir.path().join("red.png");
    std::fs::write(&png_path, &png_bytes).expect("write temp png");

    let bytes = EasyDoc::document_to_bytes(|doc| {
        doc.title("Image Fixture")
            .add_heading("Embedded Image", HeadingLevel::H1)
            .add_paragraph(Paragraph::new().add_text("Below is a tiny red pixel PNG image."))
            .add_image(easydoc::DocImage::new(&png_path).alt_text("Red pixel"))
    })
    .expect("build image fixture");

    let expected = super::types::Fixtures::roundtrip_text(&bytes);

    // tmp_dir 在此丢弃 -- PNG 文件被删除，但 DOCX 字节已包含嵌入副本。
    drop(tmp_dir);

    FidelityFixture {
        name: "image",
        original_size: bytes.len() as u64,
        expected_text: expected,
        docx_bytes: bytes,
    }
}
