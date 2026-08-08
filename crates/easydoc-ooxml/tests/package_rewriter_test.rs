//! OOXML 包重写器的二进制保真、失败安全和资源限制测试。

use std::fs;
use std::io::{Read, Write};

use easydoc_core::DocError;
use easydoc_ooxml::{PackageLimits, PackageRewriter};

fn create_package(path: &std::path::Path, binary: &[u8]) {
    let file = fs::File::create(path).expect("create package");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer
        .start_file("word/document.xml", options)
        .expect("start XML");
    writer.write_all(b"<doc>{name}</doc>").expect("write XML");
    writer
        .start_file("word/media/image1.bin", options)
        .expect("start binary");
    writer.write_all(binary).expect("write binary");
    writer.finish().expect("finish package");
}

#[test]
fn preserves_binary_entries_byte_for_byte() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("input.docx");
    let output = directory.path().join("output.docx");
    let binary = [0, 159, 146, 150, 255, 13, 10, 0, 128];
    create_package(&input, &binary);

    PackageRewriter::default()
        .rewrite(&input, &output, |name, content| {
            Ok((name == "word/document.xml").then(|| {
                String::from_utf8_lossy(content)
                    .replace("{name}", "Alice")
                    .into_bytes()
            }))
        })
        .expect("rewrite package");

    let mut archive =
        zip::ZipArchive::new(fs::File::open(output).expect("open output")).expect("open ZIP");
    let mut actual = Vec::new();
    archive
        .by_name("word/media/image1.bin")
        .expect("binary entry")
        .read_to_end(&mut actual)
        .expect("read binary");
    assert_eq!(actual, binary);
}

#[test]
fn keeps_existing_target_when_transform_fails() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("input.docx");
    let output = directory.path().join("output.docx");
    create_package(&input, b"binary");
    fs::write(&output, b"original").expect("seed target");

    let result = PackageRewriter::default().rewrite(&input, &output, |_name, _content| {
        Err(DocError::Document("expected failure".to_owned()))
    });

    assert!(result.is_err());
    assert_eq!(fs::read(output).expect("read target"), b"original");
}

#[test]
fn rejects_packages_over_entry_limit() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("input.docx");
    create_package(&input, b"binary");
    let limits = PackageLimits {
        max_entries: 1,
        ..PackageLimits::default()
    };

    let result = PackageRewriter::new(limits).rewrite(
        &input,
        directory.path().join("output.docx"),
        |_name, _content| Ok(None),
    );

    assert!(matches!(result, Err(DocError::Format(_))));
}
