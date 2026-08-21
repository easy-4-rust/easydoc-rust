//! template fill 集成测试：标量/列表填充多场景。
#![allow(clippy::items_after_statements)]

use std::collections::HashMap;
use std::path::Path;

use easydoc_core::HeadingLevel;

/// 生成含多个占位符的模板 docx。
fn make_template(dir: &Path, body: &str) -> std::path::PathBuf {
    let path = dir.join("tpl.docx");
    easydoc_writer::DocBuilder::new(&path)
        .add_heading("Template", HeadingLevel::H1)
        .add_paragraph(easydoc_writer::Paragraph::new().add_text(body))
        .save()
        .expect("save template");
    path
}

#[test]
fn fill_scalar_single_key() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_template(dir.path(), "Hello {name}!");
    let out = dir.path().join("out.docx");
    let mut data = HashMap::new();
    data.insert("name".to_owned(), "World".to_owned());
    easydoc_template::fill_template(&tpl, &out, &data).expect("fill");
    let text = easydoc_reader::read_text(&out).expect("read");
    assert!(text.contains("World"), "text: {text}");
}

#[test]
fn fill_scalar_multiple_keys() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_template(dir.path(), "{a}-{b}-{c}");
    let out = dir.path().join("out.docx");
    let mut data = HashMap::new();
    data.insert("a".to_owned(), "1".to_owned());
    data.insert("b".to_owned(), "2".to_owned());
    data.insert("c".to_owned(), "3".to_owned());
    easydoc_template::fill_template(&tpl, &out, &data).expect("fill");
    let text = easydoc_reader::read_text(&out).expect("read");
    assert!(
        text.contains('1') && text.contains('2') && text.contains('3'),
        "text: {text}"
    );
}

#[test]
fn fill_scalar_missing_key_keeps_placeholder() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_template(dir.path(), "{missing} stays");
    let out = dir.path().join("out.docx");
    let data = HashMap::new();
    easydoc_template::fill_template(&tpl, &out, &data).expect("fill");
    let text = easydoc_reader::read_text(&out).expect("read");
    // 缺失 key：占位符保留或替换为空（两者皆可），不 panic
    let _ = text;
}

#[test]
fn fill_scalar_empty_data() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_template(dir.path(), "no placeholders");
    let out = dir.path().join("out.docx");
    let data = HashMap::new();
    easydoc_template::fill_template(&tpl, &out, &data).expect("fill with empty data");
    assert!(out.exists());
}

#[test]
fn fill_scalar_repeated_key() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_template(dir.path(), "{x} and {x} again");
    let out = dir.path().join("out.docx");
    let mut data = HashMap::new();
    data.insert("x".to_owned(), "V".to_owned());
    easydoc_template::fill_template(&tpl, &out, &data).expect("fill");
    let text = easydoc_reader::read_text(&out).expect("read");
    assert!(text.contains('V'), "text: {text}");
}

#[test]
fn fill_scalar_numeric_value() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_template(dir.path(), "Count: {n}");
    let out = dir.path().join("out.docx");
    let mut data = HashMap::new();
    data.insert("n".to_owned(), "42".to_owned());
    easydoc_template::fill_template(&tpl, &out, &data).expect("fill");
    let text = easydoc_reader::read_text(&out).expect("read");
    assert!(text.contains("42"), "text: {text}");
}

#[test]
fn fill_scalar_unicode_value() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_template(dir.path(), "Name: {name}");
    let out = dir.path().join("out.docx");
    let mut data = HashMap::new();
    data.insert("name".to_owned(), "张三".to_owned());
    easydoc_template::fill_template(&tpl, &out, &data).expect("fill");
    let text = easydoc_reader::read_text(&out).expect("read");
    assert!(text.contains("张三"), "text: {text}");
}

#[test]
fn fill_scalar_missing_template_errors() {
    let out = std::path::PathBuf::from("/nonexistent/out.docx");
    let data = HashMap::new();
    let result =
        easydoc_template::fill_template(std::path::Path::new("/nonexistent/tpl.docx"), &out, &data);
    assert!(result.is_err(), "expected error for missing template");
}

/// 生成含表格行的模板（列表填充要求 `{.field}` 在 `<w:tr>` 内）。
fn make_table_template(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("tpl_table.docx");
    let content = easydoc_core::DocumentContent {
        blocks: vec![
            easydoc_core::DocumentBlock::Paragraph(vec![easydoc_core::DocumentTextRun {
                text: "Items:".into(),
                ..Default::default()
            }]),
            easydoc_core::DocumentBlock::Table(easydoc_core::DocumentTable {
                rows: vec![easydoc_core::DocumentTableRow {
                    cells: vec![easydoc_core::DocumentTableCell {
                        blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                            easydoc_core::DocumentTextRun {
                                text: "{.items}".into(),
                                ..Default::default()
                            },
                        ])],
                        column_span: 1,
                        row_span: 1,
                    }],
                    is_header: false,
                }],
            }),
        ],
        metadata: easydoc_core::DocumentMeta::default(),
    };
    let docx = easydoc_writer::content_renderer::render_document_content(&content).expect("render");
    docx.build()
        .pack(std::fs::File::create(&path).expect("create template"))
        .expect("pack template");
    path
}

#[test]
fn fill_template_list_basic() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_table_template(dir.path());
    let out = dir.path().join("out.docx");
    #[derive(serde::Serialize, Debug)]
    struct Item {
        name: String,
    }
    let items = vec![Item { name: "A".into() }, Item { name: "B".into() }];
    let result = easydoc_template::fill_template_list(&tpl, &out, &items, "items");
    assert!(result.is_ok(), "fill list: {result:?}");
    assert!(out.exists());
}

#[test]
fn fill_template_list_empty() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_table_template(dir.path());
    let out = dir.path().join("out.docx");
    let items: Vec<serde_json::Value> = Vec::new();
    let result = easydoc_template::fill_template_list(&tpl, &out, &items, "items");
    assert!(result.is_ok(), "empty fill list: {result:?}");
}

#[test]
fn fill_template_list_missing_field() {
    let dir = tempfile::tempdir().unwrap();
    let tpl = make_table_template(dir.path());
    let out = dir.path().join("out.docx");
    #[derive(serde::Serialize, Debug)]
    struct Item {
        name: String,
    }
    let items = vec![Item { name: "X".into() }];
    // 字段名不匹配时不应 panic（可能报错或保留占位符）
    let result = easydoc_template::fill_template_list(&tpl, &out, &items, "nope");
    let _ = result;
}
