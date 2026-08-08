//! 模板填充对二进制资源与 XML 动态值的回归测试。

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};

#[test]
fn template_fill_preserves_images_and_escapes_xml_values() {
    let directory = tempfile::tempdir().expect("tempdir");
    let template = directory.path().join("template.docx");
    let output = directory.path().join("output.docx");
    let binary = [0, 255, 128, 64, 13, 10, 0, 222];

    let file = fs::File::create(&template).expect("create template");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer
        .start_file("word/document.xml", options)
        .expect("start XML");
    writer
        .write_all(
            b"<w:document><w:p><w:r><w:t>{na</w:t></w:r><w:r><w:t>me}</w:t></w:r></w:p></w:document>",
        )
        .expect("write XML");
    writer
        .start_file("word/media/image1.png", options)
        .expect("start image");
    writer.write_all(&binary).expect("write image");
    writer.finish().expect("finish template");

    easydoc_template::fill_template(
        &template,
        &output,
        &HashMap::from([("name".to_owned(), "A&B <team>".to_owned())]),
    )
    .expect("fill template");

    let mut archive =
        zip::ZipArchive::new(fs::File::open(output).expect("open output")).expect("open ZIP");
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("XML")
        .read_to_string(&mut xml)
        .expect("read XML");
    assert!(xml.contains("A&amp;B &lt;team&gt;"));
    assert!(!xml.contains("{na"));
    assert!(!xml.contains("me}"));
    let mut actual = Vec::new();
    archive
        .by_name("word/media/image1.png")
        .expect("image")
        .read_to_end(&mut actual)
        .expect("read image");
    assert_eq!(actual, binary);
}
