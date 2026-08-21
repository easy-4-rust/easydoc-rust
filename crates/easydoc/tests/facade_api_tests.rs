//! facade（EasyDoc）公开 API 的深度测试：错误路径与组合场景。
#![allow(clippy::items_after_statements)]

use easydoc::EasyDoc;
use easydoc_core::{DocumentBlock, DocumentContent, DocumentMeta, DocumentTextRun, HeadingLevel};

fn tr(text: &str) -> DocumentTextRun {
    DocumentTextRun {
        text: text.to_owned(),
        ..DocumentTextRun::default()
    }
}

#[test]
fn read_text_missing_file_errors() {
    let result = EasyDoc::read_text("/nonexistent/file.docx");
    assert!(result.is_err(), "expected error for missing file");
}

#[test]
fn load_missing_file_errors() {
    let result = EasyDoc::load("/nonexistent/file.docx");
    assert!(result.is_err());
}

#[test]
fn write_content_to_bytes_roundtrip() {
    let content = DocumentContent {
        metadata: DocumentMeta::new().title("Bytes"),
        blocks: vec![DocumentBlock::Paragraph(vec![tr("hello bytes")])],
    };
    let bytes = EasyDoc::write_content_to_bytes(&content).expect("write bytes");
    assert!(!bytes.is_empty());
    // 是合法 ZIP（PK 魔数）
    assert_eq!(&bytes[0..2], b"PK");
}

#[test]
fn document_to_bytes_roundtrip() {
    let bytes = EasyDoc::document_to_bytes(|b| {
        b.add_heading("Title", HeadingLevel::H1)
            .add_paragraph(easydoc::Paragraph::new().add_text("Body"))
    })
    .expect("document to bytes");
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..2], b"PK");
}

#[test]
fn view_as_plain_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v.docx");
    EasyDoc::document(&path)
        .add_heading("Hello", HeadingLevel::H1)
        .save()
        .expect("save");
    let text = EasyDoc::view_as(&path, &easydoc::ViewMode::Plain).expect("view");
    assert!(text.contains("Hello"), "text: {text}");
}

#[test]
fn view_as_stats_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v.docx");
    EasyDoc::document(&path)
        .add_paragraph(easydoc::Paragraph::new().add_text("stats test"))
        .save()
        .expect("save");
    let stats = EasyDoc::view_as(&path, &easydoc::ViewMode::Stats).expect("stats");
    assert!(!stats.is_empty());
}

#[test]
fn view_as_outline_filters_paragraphs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v.docx");
    EasyDoc::document(&path)
        .add_heading("H", HeadingLevel::H1)
        .add_paragraph(easydoc::Paragraph::new().add_text("hidden body"))
        .save()
        .expect("save");
    let outline =
        EasyDoc::view_as(&path, &easydoc::ViewMode::Outline { max_level: 6 }).expect("outline");
    assert!(outline.contains('H'));
    assert!(!outline.contains("hidden body"), "outline: {outline}");
}

#[test]
fn to_markdown_missing_file_errors() {
    assert!(EasyDoc::to_markdown("/nonexistent/x.docx").is_err());
}

#[test]
fn write_markdown_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.docx");
    let out = dir.path().join("out.md");
    EasyDoc::document(&src)
        .add_paragraph(easydoc::Paragraph::new().add_text("markdown me"))
        .save()
        .expect("save");
    let result = EasyDoc::write_markdown(&src, &out).expect("write md");
    assert!(result.markdown.contains("markdown me"));
    assert!(out.exists(), "markdown file should exist");
}

#[test]
fn read_events_counts_document_events() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ev.docx");
    EasyDoc::document(&path)
        .add_heading("E", HeadingLevel::H1)
        .add_paragraph(easydoc::Paragraph::new().add_text("p"))
        .save()
        .expect("save");

    struct CountSink {
        events: usize,
    }
    impl easydoc::EventSink for CountSink {
        fn on_event(&mut self, _: &easydoc_core::DocumentEvent) -> easydoc_core::Result<()> {
            self.events += 1;
            Ok(())
        }
        fn on_complete(&mut self) {}
    }
    let mut sink = CountSink { events: 0 };
    EasyDoc::read_events(&path, &mut sink).expect("read events");
    assert!(
        sink.events >= 3,
        "expected DocumentStart+blocks+End, got {}",
        sink.events
    );
}

#[test]
fn fill_template_scalar_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = dir.path().join("tpl.docx");
    let out = dir.path().join("filled.docx");
    // 构造含 {name} 占位符的 docx
    EasyDoc::document(&tpl)
        .add_paragraph(easydoc::Paragraph::new().add_text("Hello {name}"))
        .save()
        .expect("save tpl");

    let mut data = std::collections::HashMap::new();
    data.insert("name".to_owned(), "World".to_owned());
    EasyDoc::fill_template(&tpl, &out, &data).expect("fill");
    assert!(out.exists());

    // 读回验证替换
    let text = EasyDoc::read_text(&out).expect("read");
    assert!(text.contains("World"), "filled text: {text}");
}

#[test]
fn markdown_builder_image_directory_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("m.docx");
    EasyDoc::document(&src)
        .add_heading("MD", HeadingLevel::H1)
        .save()
        .expect("save");
    let result = EasyDoc::markdown(&src).do_convert().expect("convert");
    assert!(result.markdown.contains("MD"));
}

#[test]
fn read_tables_empty_returns_empty_vec() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.docx");
    EasyDoc::document(&path)
        .add_paragraph(easydoc::Paragraph::new().add_text("no tables"))
        .save()
        .expect("save");
    // 无表格时返回空嵌套 Vec
    let tables: Vec<Vec<SimpleRow>> = EasyDoc::read_tables(&path).expect("read tables");
    assert!(tables.is_empty());
}

#[derive(easydoc::DocxRowDerive)]
struct SimpleRow {
    #[docx(name = "A", order = 0)]
    a: String,
}

#[test]
fn write_table_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("table.docx");
    let rows = vec![SimpleRow { a: "x".into() }];
    EasyDoc::write_table(&path, &rows)
        .do_write()
        .expect("write table");
    assert!(path.exists());
    let text = EasyDoc::read_text(&path).expect("read");
    assert!(text.contains('x'), "table text: {text}");
}

#[test]
fn edit_opens_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("e.docx");
    EasyDoc::document(&path)
        .add_paragraph(easydoc::Paragraph::new().add_text("editable"))
        .save()
        .expect("save");
    let editor = EasyDoc::edit(&path).expect("open for edit");
    let _ = editor;
}

#[test]
fn edit_missing_file_errors() {
    assert!(EasyDoc::edit("/nonexistent/e.docx").is_err());
}

#[test]
fn write_content_missing_dir_errors() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![tr("x")])],
        ..Default::default()
    };
    let result = EasyDoc::write_content(&content, "/nonexistent_dir/doc.docx");
    assert!(result.is_err(), "expected error writing to missing dir");
}

// ===========================================================================
// Paragraph / Run builder 渲染验证（经 facade 全链路）
// ===========================================================================

#[test]
fn paragraph_builder_renders_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.docx");
    EasyDoc::document(&path)
        .add_paragraph(
            easydoc::Paragraph::new()
                .add_text("hello ")
                .add_run(easydoc::Run::new("world")),
        )
        .save()
        .expect("save");
    let text = EasyDoc::read_text(&path).expect("read");
    assert!(
        text.contains("hello") && text.contains("world"),
        "text: {text}"
    );
}

#[test]
fn run_bold_renders_bold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("b.docx");
    EasyDoc::document(&path)
        .add_paragraph(easydoc::Paragraph::new().add_run(easydoc::Run::new("bold!").bold()))
        .save()
        .expect("save");
    let text = EasyDoc::read_text(&path).expect("read");
    assert!(text.contains("bold!"), "text: {text}");
}

#[test]
fn run_italic_renders() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("i.docx");
    EasyDoc::document(&path)
        .add_paragraph(easydoc::Paragraph::new().add_run(easydoc::Run::new("ital").italic()))
        .save()
        .expect("save");
    let text = EasyDoc::read_text(&path).expect("read");
    assert!(text.contains("ital"), "text: {text}");
}

#[test]
fn paragraph_alignment_renders() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.docx");
    EasyDoc::document(&path)
        .add_paragraph(
            easydoc::Paragraph::new()
                .add_text("center me")
                .alignment(easydoc_core::HorizontalAlignment::Center),
        )
        .save()
        .expect("save");
    let text = EasyDoc::read_text(&path).expect("read");
    assert!(text.contains("center me"), "text: {text}");
}

#[test]
fn run_unicode_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("u.docx");
    EasyDoc::document(&path)
        .add_paragraph(easydoc::Paragraph::new().add_run(easydoc::Run::new("中文内容")))
        .save()
        .expect("save");
    let text = EasyDoc::read_text(&path).expect("read");
    assert!(text.contains("中文内容"), "text: {text}");
}

#[test]
fn paragraph_multiple_runs_ordered() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.docx");
    EasyDoc::document(&path)
        .add_paragraph(
            easydoc::Paragraph::new()
                .add_text("one ")
                .add_run(easydoc::Run::new("two ").bold())
                .add_text("three"),
        )
        .save()
        .expect("save");
    let text = EasyDoc::read_text(&path).expect("read");
    let t1 = text.find("one").unwrap();
    let t2 = text.find("two").unwrap();
    let t3 = text.find("three").unwrap();
    assert!(t1 < t2 && t2 < t3, "runs should preserve order: {text}");
}

#[test]
fn empty_paragraph_renders() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("e.docx");
    EasyDoc::document(&path)
        .add_paragraph(easydoc::Paragraph::new())
        .save()
        .expect("save");
    assert!(path.exists());
}
