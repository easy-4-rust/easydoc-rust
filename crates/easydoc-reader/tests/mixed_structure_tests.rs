//! reader 混合文档结构测试：多种块类型组合。

use easydoc_core::DocumentBlock;
use easydoc_reader::DocxSaxReader;

fn doc(body: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>{body}</w:body>
</w:document>"#
    )
    .into_bytes()
}

fn p(text: &str) -> String {
    format!("<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
}

fn heading(text: &str, level: u32) -> String {
    format!(
        r#"<w:p><w:pPr><w:pStyle w:val="Heading{level}"/></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
    )
}

fn table_2x2(a: &str, b: &str, c: &str, d: &str) -> String {
    format!(
        r"<w:tbl><w:tr><w:tc>{}</w:tc><w:tc>{}</w:tc></w:tr><w:tr><w:tc>{}</w:tc><w:tc>{}</w:tc></w:tr></w:tbl>",
        p(a),
        p(b),
        p(c),
        p(d)
    )
}

fn list_item(text: &str, ilvl: u32) -> String {
    format!(
        r#"<w:p><w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="{ilvl}"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
    )
}

fn blocks(xml: &[u8]) -> Vec<DocumentBlock> {
    let mut reader = DocxSaxReader::from_reader(xml);
    reader.read_blocks().expect("parse")
}

#[test]
fn mixed_heading_paragraph_table() {
    let xml = doc(&format!(
        "{}{}{}",
        heading("Title", 1),
        p("intro"),
        table_2x2("a", "b", "c", "d")
    ));
    let bs = blocks(&xml);
    assert!(bs.len() >= 3, "blocks: {bs:?}");
    assert!(matches!(bs[0], DocumentBlock::Heading { .. }));
    assert!(matches!(bs[1], DocumentBlock::Paragraph(_)));
    assert!(bs.iter().any(|b| matches!(b, DocumentBlock::Table(_))));
}

#[test]
fn mixed_list_table_list() {
    let xml = doc(&format!(
        "{}{}{}",
        list_item("one", 0),
        table_2x2("x", "y", "z", "w"),
        list_item("two", 0)
    ));
    let bs = blocks(&xml);
    // 列表-表格-列表：表格前后的列表各自独立
    let lists = bs
        .iter()
        .filter(|b| matches!(b, DocumentBlock::List(_)))
        .count();
    assert!(lists >= 1, "lists: {lists}");
    assert!(bs.iter().any(|b| matches!(b, DocumentBlock::Table(_))));
}

#[test]
fn paragraph_table_paragraph_preserves_order() {
    let xml = doc(&format!(
        "{} {} {}",
        p("first"),
        table_2x2("a", "b", "c", "d"),
        p("last")
    ));
    let bs = blocks(&xml);
    // 顺序：Paragraph, Table, Paragraph
    assert!(
        matches!(bs[0], DocumentBlock::Paragraph(_)),
        "bs[0]: {:?}",
        bs[0]
    );
    let table_pos = bs
        .iter()
        .position(|b| matches!(b, DocumentBlock::Table(_)))
        .unwrap();
    let last_para = bs
        .iter()
        .rposition(|b| matches!(b, DocumentBlock::Paragraph(_)))
        .unwrap();
    assert!(
        table_pos < last_para,
        "table should come before last paragraph"
    );
}

#[test]
fn heading_levels_all_preserved() {
    let xml = doc(&format!(
        "{}{}{}",
        heading("H1", 1),
        heading("H2", 2),
        heading("H3", 3)
    ));
    let bs = blocks(&xml);
    let levels: Vec<u8> = bs
        .iter()
        .filter_map(|b| match b {
            DocumentBlock::Heading { level, .. } => Some(*level),
            _ => None,
        })
        .collect();
    assert_eq!(levels, vec![1, 2, 3], "levels: {levels:?}");
}

#[test]
fn nested_list_two_levels() {
    let xml = doc(&format!(
        "{}{}",
        list_item("parent", 0),
        list_item("child", 1)
    ));
    let bs = blocks(&xml);
    match &bs[0] {
        DocumentBlock::List(l) => {
            assert_eq!(l.items.len(), 1);
            assert!(l.items[0].nested.is_some(), "should have nested child");
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn three_paragraphs_preserved() {
    let xml = doc(&format!("{}{}{}", p("a"), p("b"), p("c")));
    let bs = blocks(&xml);
    assert_eq!(bs.len(), 3);
    let texts: Vec<String> = bs
        .iter()
        .filter_map(|b| match b {
            DocumentBlock::Paragraph(runs) => {
                Some(runs.iter().map(|r| r.text.clone()).collect::<String>())
            }
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["a", "b", "c"]);
}

#[test]
fn table_followed_by_list() {
    let xml = doc(&format!(
        "{}{}",
        table_2x2("1", "2", "3", "4"),
        list_item("after", 0)
    ));
    let bs = blocks(&xml);
    assert!(bs.iter().any(|b| matches!(b, DocumentBlock::Table(_))));
    assert!(bs.iter().any(|b| matches!(b, DocumentBlock::List(_))));
}

#[test]
fn empty_body_no_blocks() {
    let bs = blocks(&doc(""));
    assert!(bs.is_empty());
}

#[test]
fn only_whitespace_body() {
    let bs = blocks(&doc("   "));
    assert!(bs.is_empty());
}

#[test]
fn table_with_single_row_single_cell() {
    let xml =
        doc(r"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>only</w:t></w:r></w:p></w:tc></w:tr></w:tbl>");
    let bs = blocks(&xml);
    match &bs[0] {
        DocumentBlock::Table(t) => {
            assert_eq!(t.rows.len(), 1);
            assert_eq!(t.rows[0].cells.len(), 1);
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn heading_then_list_then_heading() {
    let xml = doc(&format!(
        "{}{}{}",
        heading("Intro", 2),
        list_item("point", 0),
        heading("End", 2)
    ));
    let bs = blocks(&xml);
    let headings = bs
        .iter()
        .filter(|b| matches!(b, DocumentBlock::Heading { .. }))
        .count();
    assert_eq!(headings, 2);
    assert!(bs.iter().any(|b| matches!(b, DocumentBlock::List(_))));
}

#[test]
fn two_tables_separate() {
    let xml = doc(&format!(
        "{}{}",
        table_2x2("a", "b", "c", "d"),
        table_2x2("e", "f", "g", "h")
    ));
    let bs = blocks(&xml);
    let tables = bs
        .iter()
        .filter(|b| matches!(b, DocumentBlock::Table(_)))
        .count();
    assert_eq!(tables, 2, "two tables should be separate blocks");
}
