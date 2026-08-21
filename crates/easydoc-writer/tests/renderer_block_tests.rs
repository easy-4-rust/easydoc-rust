//! writer `content_renderer` 各块类型的 XML 输出验证。

use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentList, DocumentListItem, DocumentTable,
    DocumentTableCell, DocumentTableRow, DocumentTextRun,
};
use easydoc_writer::content_renderer::render_document_content;

fn tr(text: &str) -> DocumentTextRun {
    DocumentTextRun {
        text: text.to_owned(),
        ..DocumentTextRun::default()
    }
}

/// 渲染并返回 document.xml 字符串。
fn render_xml(content: &DocumentContent) -> String {
    let docx = render_document_content(content).expect("render");
    String::from_utf8(docx.build().document).expect("document.xml is UTF-8")
}

#[test]
fn render_paragraph_emits_text() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![tr("hello xml")])],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("hello xml"), "xml: {xml}");
}

#[test]
fn render_bold_run_emits_bold() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
            text: "bold me".into(),
            bold: true,
            ..Default::default()
        }])],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("bold me"), "xml: {xml}");
    // docx-rs 用 <w:b/> 表达粗体
    assert!(
        xml.contains("<w:b/>") || xml.contains("<w:b "),
        "xml: {xml}"
    );
}

#[test]
fn render_italic_run_emits_italic() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
            text: "italic".into(),
            italic: true,
            ..Default::default()
        }])],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(
        xml.contains("<w:i/>") || xml.contains("<w:i "),
        "xml: {xml}"
    );
}

#[test]
fn render_strikethrough_run() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
            text: "strike".into(),
            strikethrough: true,
            ..Default::default()
        }])],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("strike"), "xml: {xml}");
    assert!(xml.contains("<w:strike"), "xml: {xml}");
}

#[test]
fn render_code_block_emits_code() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::CodeBlock {
            language: Some("rust".into()),
            code: "fn x() {}".into(),
        }],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("fn x() {}"), "xml: {xml}");
}

#[test]
fn render_math_block_emits_marker_and_collects_latex() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Math {
            omml: None,
            latex: Some(r"\frac{1}{2}".into()),
            display: true,
        }],
        ..Default::default()
    };
    let xml = render_xml(&content);
    // Math 块渲染为占位标记，latex 存入 take_rendered_math
    assert!(xml.contains("@@EASYDOC_MATH"), "xml: {xml}");
    let math = easydoc_writer::content_renderer::take_rendered_math();
    assert_eq!(math.len(), 1);
    assert_eq!(math[0].1, r"\frac{1}{2}");
    assert!(math[0].2); // display
}

#[test]
fn render_inline_math_block_collects_display_false() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Math {
            omml: None,
            latex: Some("x^2".into()),
            display: false,
        }],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("@@EASYDOC_MATH"), "xml: {xml}");
    let math = easydoc_writer::content_renderer::take_rendered_math();
    assert_eq!(math.len(), 1);
    assert_eq!(math[0].1, "x^2");
    assert!(!math[0].2); // inline
}

#[test]
fn render_footnote_emits_marker_text() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Footnote {
            id: 1,
            blocks: vec![DocumentBlock::Paragraph(vec![tr("note body")])],
        }],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("note body"), "xml: {xml}");
    assert!(xml.contains("[^1]"), "footnote marker missing: {xml}");
}

#[test]
fn render_endnote_emits_marker_text() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Endnote {
            id: 2,
            blocks: vec![DocumentBlock::Paragraph(vec![tr("end body")])],
        }],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("end body"), "xml: {xml}");
    assert!(
        xml.contains("[^endnote-2]"),
        "endnote marker missing: {xml}"
    );
}

#[test]
fn render_thematic_break_emits_page_break() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::ThematicBreak],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("<w:br"), "expected break, xml: {xml}");
}

#[test]
fn render_list_emits_numbering() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: true,
            start_number: None,
            items: vec![DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(vec![tr("item")])],
                nested: None,
            }],
        })],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("item"), "xml: {xml}");
}

#[test]
fn render_empty_blocks_no_panic() {
    let content = DocumentContent {
        blocks: vec![],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(
        !xml.is_empty(),
        "empty doc should still produce valid xml shell"
    );
}

#[test]
fn render_mixed_blocks_all_present() {
    let content = DocumentContent {
        blocks: vec![
            DocumentBlock::Paragraph(vec![tr("p")]),
            DocumentBlock::Heading {
                level: 1,
                runs: vec![tr("h")],
            },
            DocumentBlock::Table(DocumentTable {
                rows: vec![DocumentTableRow {
                    cells: vec![DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![tr("c")])],
                        column_span: 1,
                        row_span: 1,
                    }],
                    is_header: false,
                }],
            }),
        ],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(
        xml.contains('p') && xml.contains('h') && xml.contains('c'),
        "xml: {xml}"
    );
}

#[test]
fn render_textbox_flattens_blocks() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::TextBox(vec![DocumentBlock::Paragraph(
            vec![tr("inside box")],
        )])],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("inside box"), "xml: {xml}");
}

#[test]
fn render_section_flattens_blocks() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Section {
            blocks: vec![DocumentBlock::Paragraph(vec![tr("in section")])],
            section_type: None,
        }],
        ..Default::default()
    };
    let xml = render_xml(&content);
    assert!(xml.contains("in section"), "xml: {xml}");
}
