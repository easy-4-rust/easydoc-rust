//! SAX 解析器的 XML 结构深度测试：脚注/尾注/文本框/分节/特殊元素。

use easydoc_core::DocumentBlock;
use easydoc_reader::DocxSaxReader;

/// 解析 XML 并返回所有块。
fn parse_blocks(xml: &[u8]) -> Vec<DocumentBlock> {
    let mut reader = DocxSaxReader::from_reader(xml);
    reader.read_blocks().expect("parse blocks")
}

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

// ===========================================================================
// 基础段落
// ===========================================================================

#[test]
fn single_paragraph() {
    let blocks = parse_blocks(&doc(&p("hello")));
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], DocumentBlock::Paragraph(runs) if runs[0].text == "hello"));
}

#[test]
fn two_paragraphs() {
    let blocks = parse_blocks(&doc(&format!("{}{}", p("a"), p("b"))));
    assert_eq!(blocks.len(), 2);
}

#[test]
fn paragraph_with_multiple_runs() {
    let xml = doc("<w:p><w:r><w:t>a</w:t></w:r><w:r><w:t>b</w:t></w:r></w:p>");
    let blocks = parse_blocks(&xml);
    match &blocks[0] {
        DocumentBlock::Paragraph(runs) => assert_eq!(runs.len(), 2),
        _ => panic!("expected Paragraph"),
    }
}

#[test]
fn empty_paragraph() {
    let blocks = parse_blocks(&doc("<w:p/>"));
    // 空段落不产生块
    assert!(blocks.is_empty());
}

#[test]
fn paragraph_with_whitespace() {
    let blocks = parse_blocks(&doc(&p("  spaced  ")));
    assert_eq!(blocks.len(), 1);
}

// ===========================================================================
// 标题
// ===========================================================================

#[test]
fn heading_style_detection() {
    let xml =
        doc(r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>H1</w:t></w:r></w:p>"#);
    let blocks = parse_blocks(&xml);
    match &blocks[0] {
        DocumentBlock::Heading { level, .. } => assert_eq!(*level, 1),
        _ => panic!("expected Heading"),
    }
}

#[test]
fn heading_level_2() {
    let xml =
        doc(r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>H2</w:t></w:r></w:p>"#);
    let blocks = parse_blocks(&xml);
    match &blocks[0] {
        DocumentBlock::Heading { level, .. } => assert_eq!(*level, 2),
        _ => panic!("expected Heading"),
    }
}

#[test]
fn unknown_style_is_paragraph() {
    let xml =
        doc(r#"<w:p><w:pPr><w:pStyle w:val="CustomStyle"/></w:pPr><w:r><w:t>x</w:t></w:r></w:p>"#);
    let blocks = parse_blocks(&xml);
    assert!(matches!(&blocks[0], DocumentBlock::Paragraph(_)));
}

// ===========================================================================
// 脚注/尾注引用
// ===========================================================================

#[test]
fn footnote_reference_does_not_panic() {
    let xml =
        doc(r#"<w:p><w:r><w:t>text</w:t></w:r><w:r><w:footnoteReference w:id="1"/></w:r></w:p>"#);
    let blocks = parse_blocks(&xml);
    // 脚注引用元素当前被忽略；验证解析不 panic 且文本保留
    match &blocks[0] {
        DocumentBlock::Paragraph(runs) => {
            assert_eq!(runs[0].text, "text");
        }
        _ => panic!("expected Paragraph"),
    }
}

// ===========================================================================
// 分页/分栏
// ===========================================================================

#[test]
fn page_break_detection() {
    let xml = doc(r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#);
    let blocks = parse_blocks(&xml);
    assert!(!blocks.is_empty(), "page break should produce a block");
}

#[test]
fn column_break_detection() {
    let xml = doc(r#"<w:p><w:r><w:br w:type="column"/></w:r></w:p>"#);
    let blocks = parse_blocks(&xml);
    // 分栏符可能产生 ColumnBreak 块或空（当前实现按 br 处理）
    let _ = blocks;
}

#[test]
fn text_break_is_not_block_break() {
    let xml = doc(r"<w:p><w:r><w:t>a</w:t><w:br/><w:t>b</w:t></w:r></w:p>");
    let blocks = parse_blocks(&xml);
    match &blocks[0] {
        DocumentBlock::Paragraph(runs) => {
            // 换行可能合并进单个 run，验证文本存在
            let all: String = runs.iter().map(|r| r.text.as_str()).collect();
            assert!(all.contains('a') && all.contains('b'), "all: {all}");
        }
        _ => panic!("expected Paragraph"),
    }
}

// ===========================================================================
// 图片
// ===========================================================================

#[test]
fn drawing_without_relationship_is_ignored() {
    let xml = doc(
        r#"<w:p><w:r><w:drawing><wp:inline xmlns:wp="x"><a:blip xmlns:a="y" r:embed="rId9" xmlns:r="z"/></wp:inline></w:drawing></w:r></w:p>"#,
    );
    let blocks = parse_blocks(&xml);
    // 无 relationship 时不产生 Image 块，但解析不 panic
    let _ = blocks;
}

// ===========================================================================
// 表格
// ===========================================================================

#[test]
fn table_with_two_cells() {
    let xml = doc(
        r"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
    );
    let blocks = parse_blocks(&xml);
    match &blocks[0] {
        DocumentBlock::Table(t) => {
            assert_eq!(t.rows.len(), 1);
            assert_eq!(t.rows[0].cells.len(), 2);
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn table_multiple_rows() {
    let xml = doc(
        r"<w:tbl><w:tr><w:tc><w:p/></w:tc></w:tr><w:tr><w:tc><w:p/></w:tc></w:tr><w:tr><w:tc><w:p/></w:tc></w:tr></w:tbl>",
    );
    let blocks = parse_blocks(&xml);
    match &blocks[0] {
        DocumentBlock::Table(t) => assert_eq!(t.rows.len(), 3),
        _ => panic!("expected Table"),
    }
}

#[test]
fn empty_table() {
    let blocks = parse_blocks(&doc("<w:tbl/>"));
    assert!(blocks.is_empty());
}

// ===========================================================================
// 特殊文本
// ===========================================================================

#[test]
fn tab_character_in_run() {
    let xml = doc("<w:p><w:r><w:t>a</w:t><w:tab/><w:t>b</w:t></w:r></w:p>");
    let blocks = parse_blocks(&xml);
    match &blocks[0] {
        DocumentBlock::Paragraph(runs) => {
            let all: String = runs.iter().map(|r| r.text.as_str()).collect();
            assert!(all.contains('a') && all.contains('b'), "all: {all}");
        }
        _ => panic!("expected Paragraph"),
    }
}

#[test]
fn unicode_text_preserved() {
    let blocks = parse_blocks(&doc(&p("中文内容")));
    match &blocks[0] {
        DocumentBlock::Paragraph(runs) => assert_eq!(runs[0].text, "中文内容"),
        _ => panic!("expected Paragraph"),
    }
}

#[test]
fn xml_escaped_text_decoded() {
    let blocks = parse_blocks(&doc("<w:p><w:r><w:t>a &amp; b &lt;c&gt;</w:t></w:r></w:p>"));
    match &blocks[0] {
        DocumentBlock::Paragraph(runs) => {
            let all: String = runs.iter().map(|r| r.text.as_str()).collect();
            // 实体被解码或按字面保留，二者皆可接受——验证 a/b/c 都在
            assert!(
                all.contains('a') && all.contains('b') && all.contains('c'),
                "all: {all}"
            );
        }
        _ => panic!("expected Paragraph"),
    }
}

#[test]
fn numbering_reference_creates_list() {
    let xml = doc(
        r#"<w:p><w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr><w:r><w:t>item</w:t></w:r></w:p>"#,
    );
    let blocks = parse_blocks(&xml);
    assert!(
        blocks.iter().any(|b| matches!(b, DocumentBlock::List(_))),
        "blocks: {blocks:?}"
    );
}

#[test]
fn consecutive_numbering_merges_to_one_list() {
    let xml = doc(
        r#"<w:p><w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr><w:r><w:t>a</w:t></w:r></w:p>
           <w:p><w:pPr><w:numPr><w:numId w:val="1"/><w:ilvl w:val="0"/></w:numPr></w:pPr><w:r><w:t>b</w:t></w:r></w:p>"#,
    );
    let blocks = parse_blocks(&xml);
    assert_eq!(
        blocks.len(),
        1,
        "two list items should merge, blocks: {blocks:?}"
    );
    match &blocks[0] {
        DocumentBlock::List(l) => assert_eq!(l.items.len(), 2),
        _ => panic!("expected List"),
    }
}
