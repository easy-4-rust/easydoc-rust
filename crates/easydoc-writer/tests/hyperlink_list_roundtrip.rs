//! Integration tests verifying that hyperlinks and lists survive the
//! `DocumentContent -> render -> pack -> ZIP` pipeline.
//!
//! Each test constructs a `DocumentContent`, renders it to DOCX bytes via
//! `easydoc_writer::content_renderer::render_document_content`, then opens the
//! resulting ZIP archive and inspects the raw OOXML for the expected elements.

use std::io::{Cursor, Read as _};

use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentList, DocumentListItem, DocumentTextRun,
};
use easydoc_writer::content_renderer::render_document_content;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_run(text: &str) -> DocumentTextRun {
    DocumentTextRun {
        text: text.into(),
        ..DocumentTextRun::default()
    }
}

fn make_hyperlink_run(text: &str, url: &str) -> DocumentTextRun {
    DocumentTextRun {
        text: text.into(),
        hyperlink: Some(url.into()),
        ..DocumentTextRun::default()
    }
}

/// Renders a `DocumentContent` to DOCX bytes.
fn render_to_bytes(content: &DocumentContent) -> Vec<u8> {
    let docx = render_document_content(content).expect("render failed");
    let mut buf = Vec::new();
    let cursor = Cursor::new(&mut buf);
    docx.build().pack(cursor).expect("pack failed");
    buf
}

/// Extracts `word/document.xml` from a DOCX byte buffer.
fn extract_document_xml(bytes: &[u8]) -> String {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("open zip failed");
    let mut file = archive
        .by_name("word/document.xml")
        .expect("word/document.xml not found");
    let mut xml = String::new();
    file.read_to_string(&mut xml).expect("read failed");
    xml
}

/// Extracts `word/_rels/document.xml.rels` from a DOCX byte buffer.
fn extract_document_rels(bytes: &[u8]) -> String {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("open zip failed");
    let mut file = archive
        .by_name("word/_rels/document.xml.rels")
        .expect("word/_rels/document.xml.rels not found");
    let mut xml = String::new();
    file.read_to_string(&mut xml).expect("read failed");
    xml
}

/// Extracts `word/numbering.xml` from a DOCX byte buffer.
fn extract_numbering_xml(bytes: &[u8]) -> String {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("open zip failed");
    let mut file = archive
        .by_name("word/numbering.xml")
        .expect("word/numbering.xml not found");
    let mut xml = String::new();
    file.read_to_string(&mut xml).expect("read failed");
    xml
}

// ---------------------------------------------------------------------------
// Hyperlink tests
// ---------------------------------------------------------------------------

#[test]
fn hyperlink_generates_hyperlink_element_in_document_xml() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![make_hyperlink_run(
            "Click here",
            "https://example.com",
        )])],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let xml = extract_document_xml(&bytes);

    // The document XML must contain a <w:hyperlink element.
    assert!(
        xml.contains("<w:hyperlink"),
        "expected <w:hyperlink in document.xml, got: {}",
        &xml[..xml.len().min(500)]
    );
    // The hyperlink must wrap the run text.
    assert!(
        xml.contains("Click here"),
        "hyperlink text 'Click here' missing from document.xml"
    );
}

#[test]
fn hyperlink_generates_relationship_in_rels() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![make_hyperlink_run(
            "Link",
            "https://example.com",
        )])],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let rels = extract_document_rels(&bytes);

    // The rels file must contain a hyperlink relationship pointing to the URL.
    assert!(
        rels.contains("hyperlink"),
        "expected 'hyperlink' relationship type in rels, got: {rels}"
    );
    assert!(
        rels.contains("https://example.com"),
        "expected target URL in rels, got: {rels}"
    );
    assert!(
        rels.contains("TargetMode=\"External\""),
        "expected TargetMode=External in rels, got: {rels}"
    );
}

#[test]
fn hyperlink_rid_matches_between_document_and_rels() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![make_hyperlink_run(
            "Test",
            "https://example.org",
        )])],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let xml = extract_document_xml(&bytes);
    let rels = extract_document_rels(&bytes);

    // Extract the r:id from the <w:hyperlink r:id="rIdHyperlinkXXX"> element.
    let rid_start = xml
        .find(r#"w:hyperlink r:id=""#)
        .expect("no r:id in hyperlink");
    let rid_val_start = rid_start + r#"w:hyperlink r:id=""#.len();
    let rid_val_end = xml[rid_val_start..].find('"').unwrap() + rid_val_start;
    let rid = &xml[rid_val_start..rid_val_end];

    // The same rId must appear in the rels file.
    assert!(
        rels.contains(rid),
        "rId '{rid}' from document.xml not found in rels: {rels}"
    );
}

#[test]
fn hyperlink_in_heading_preserved() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Heading {
            level: 1,
            runs: vec![
                make_run("See "),
                make_hyperlink_run("documentation", "https://docs.example.com"),
            ],
        }],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let xml = extract_document_xml(&bytes);

    assert!(
        xml.contains("<w:hyperlink"),
        "hyperlink in heading should produce <w:hyperlink> element"
    );
    assert!(xml.contains("documentation"));
}

// ---------------------------------------------------------------------------
// List tests
// ---------------------------------------------------------------------------

#[test]
fn bullet_list_has_numbering_property() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: false,
            start_number: None,
            items: vec![
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![make_run("Item 1")])],
                    nested: None,
                },
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![make_run("Item 2")])],
                    nested: None,
                },
            ],
        })],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let xml = extract_document_xml(&bytes);

    // List paragraphs must have <w:numPr> with a numId.
    assert!(
        xml.contains("<w:numPr>"),
        "bullet list paragraph must have <w:numPr>, got: {}",
        &xml[..xml.len().min(1000)]
    );
    assert!(
        xml.contains("<w:numId w:val=\"10\""),
        "bullet list must use numId=10, got: {}",
        &xml[..xml.len().min(1000)]
    );
}

#[test]
fn ordered_list_has_numbering_property() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: true,
            start_number: Some(1),
            items: vec![
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![make_run("First")])],
                    nested: None,
                },
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![make_run("Second")])],
                    nested: None,
                },
            ],
        })],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let xml = extract_document_xml(&bytes);

    assert!(
        xml.contains("<w:numPr>"),
        "ordered list paragraph must have <w:numPr>"
    );
    assert!(
        xml.contains("<w:numId w:val=\"11\""),
        "ordered list must use numId=11, got: {}",
        &xml[..xml.len().min(1000)]
    );
}

#[test]
fn nested_list_uses_different_indent_levels() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: false,
            start_number: None,
            items: vec![DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(vec![make_run("Top")])],
                nested: Some(Box::new(DocumentList {
                    ordered: false,
                    start_number: None,
                    items: vec![DocumentListItem {
                        blocks: vec![DocumentBlock::Paragraph(vec![make_run("Nested")])],
                        nested: None,
                    }],
                })),
            }],
        })],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let xml = extract_document_xml(&bytes);

    // Top-level item: ilvl=0.
    assert!(
        xml.contains("<w:ilvl w:val=\"0\""),
        "top-level list item must have ilvl=0"
    );
    // Nested item: ilvl=1.
    assert!(
        xml.contains("<w:ilvl w:val=\"1\""),
        "nested list item must have ilvl=1"
    );
}

#[test]
fn numbering_xml_contains_bullet_and_decimal_definitions() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: false,
            start_number: None,
            items: vec![DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(vec![make_run("Item")])],
                nested: None,
            }],
        })],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let num_xml = extract_numbering_xml(&bytes);

    // numbering.xml must exist and contain bullet format.
    assert!(
        num_xml.contains("bullet"),
        "numbering.xml must define bullet format, got: {}",
        &num_xml[..num_xml.len().min(500)]
    );
    // And must contain decimal format.
    assert!(
        num_xml.contains("decimal"),
        "numbering.xml must define decimal format, got: {}",
        &num_xml[..num_xml.len().min(500)]
    );
    // And must reference our abstractNum IDs.
    assert!(
        num_xml.contains("abstractNumId=\"0\""),
        "numbering.xml must define abstractNum 0 (bullet)"
    );
    assert!(
        num_xml.contains("abstractNumId=\"1\""),
        "numbering.xml must define abstractNum 1 (decimal)"
    );
}

#[test]
fn list_with_multiple_items_produces_multiple_paragraphs() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: false,
            start_number: None,
            items: vec![
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![make_run("A")])],
                    nested: None,
                },
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![make_run("B")])],
                    nested: None,
                },
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![make_run("C")])],
                    nested: None,
                },
            ],
        })],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let xml = extract_document_xml(&bytes);

    // Each list item should produce a paragraph with numPr.
    let numpr_count = xml.matches("<w:numPr>").count();
    assert!(
        numpr_count >= 3,
        "expected at least 3 <w:numPr> elements for 3 list items, found {numpr_count}"
    );
}

#[test]
fn hyperlink_in_list_item() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: false,
            start_number: None,
            items: vec![DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(vec![
                    make_run("Visit "),
                    make_hyperlink_run("our site", "https://example.com"),
                ])],
                nested: None,
            }],
        })],
        ..Default::default()
    };
    let bytes = render_to_bytes(&content);
    let xml = extract_document_xml(&bytes);

    // The list paragraph must have both numPr and hyperlink.
    assert!(
        xml.contains("<w:numPr>"),
        "list item with hyperlink must have <w:numPr>"
    );
    assert!(
        xml.contains("<w:hyperlink"),
        "list item with hyperlink must have <w:hyperlink>"
    );
    assert!(xml.contains("our site"));
}
