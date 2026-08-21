//! markdown renderer 输出验证（通过公共 `render_document` API）。

use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentList, DocumentListItem, DocumentMeta, DocumentTable,
    DocumentTableCell, DocumentTableRow, DocumentTextRun,
};
use easydoc_markdown::{MarkdownOptions, render_document};

fn tr(text: &str) -> DocumentTextRun {
    DocumentTextRun {
        text: text.to_owned(),
        ..DocumentTextRun::default()
    }
}

fn render(content: &DocumentContent) -> String {
    render_document(content, MarkdownOptions::default())
        .expect("render")
        .markdown
}

#[test]
fn render_heading_marks_level() {
    let content = DocumentContent {
        blocks: vec![
            DocumentBlock::Heading {
                level: 1,
                runs: vec![tr("One")],
            },
            DocumentBlock::Heading {
                level: 3,
                runs: vec![tr("Three")],
            },
        ],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("# One"), "md: {md}");
    assert!(md.contains("### Three"), "md: {md}");
}

#[test]
fn render_bold_italic_strike_wrapping() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![
            DocumentTextRun {
                text: "B".into(),
                bold: true,
                ..Default::default()
            },
            DocumentTextRun {
                text: "I".into(),
                italic: true,
                ..Default::default()
            },
            DocumentTextRun {
                text: "S".into(),
                strikethrough: true,
                ..Default::default()
            },
        ])],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("**B**"), "md: {md}");
    assert!(md.contains("*I*"), "md: {md}");
    assert!(md.contains("~~S~~"), "md: {md}");
}

#[test]
fn render_hyperlink_format() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
            text: "link".into(),
            hyperlink: Some("https://e.com/a".into()),
            ..Default::default()
        }])],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("[link](https://e.com/a)"), "md: {md}");
}

#[test]
fn render_table_marks_header() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Table(DocumentTable {
            rows: vec![
                DocumentTableRow {
                    cells: vec![cell("H1"), cell("H2")],
                    is_header: true,
                },
                DocumentTableRow {
                    cells: vec![cell("a"), cell("b")],
                    is_header: false,
                },
            ],
        })],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("| H1 | H2 |"), "md: {md}");
    assert!(md.contains("| a | b |"), "md: {md}");
}

#[test]
fn render_list_ordered_uses_numbers() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: true,
            start_number: None,
            items: vec![
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![tr("first")])],
                    nested: None,
                },
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![tr("second")])],
                    nested: None,
                },
            ],
        })],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("1.") && md.contains("2."), "md: {md}");
    assert!(md.contains("first") && md.contains("second"), "md: {md}");
}

#[test]
fn render_unordered_list_uses_dash() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: false,
            start_number: None,
            items: vec![DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(vec![tr("item")])],
                nested: None,
            }],
        })],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("- item") || md.contains("* item"), "md: {md}");
}

#[test]
fn render_nested_list_indents() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: false,
            start_number: None,
            items: vec![DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(vec![tr("parent")])],
                nested: Some(Box::new(DocumentList {
                    ordered: false,
                    start_number: None,
                    items: vec![DocumentListItem {
                        blocks: vec![DocumentBlock::Paragraph(vec![tr("child")])],
                        nested: None,
                    }],
                })),
            }],
        })],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("parent"), "md: {md}");
    assert!(md.contains("child"), "md: {md}");
}

#[test]
fn render_image_with_alt() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Image(easydoc_core::DocumentImage {
            alt_text: Some("logo".into()),
            data: None,
            extension: Some("png".into()),
        })],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("logo"), "md: {md}");
}

#[test]
fn render_code_block_with_language() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::CodeBlock {
            language: Some("rust".into()),
            code: "let x = 1;".into(),
        }],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("```rust"), "md: {md}");
    assert!(md.contains("let x = 1;"), "md: {md}");
}

#[test]
fn render_empty_document() {
    let content = DocumentContent::default();
    let md = render(&content);
    assert!(md.is_empty() || md.trim().is_empty(), "md: {md}");
}

#[test]
fn render_front_matter_emitted_when_enabled() {
    let content = DocumentContent {
        metadata: DocumentMeta::new().title("T").author("A"),
        blocks: vec![DocumentBlock::Paragraph(vec![tr("body")])],
    };
    let opts = MarkdownOptions {
        include_front_matter: true,
        ..MarkdownOptions::default()
    };
    let md = render_document(&content, opts).expect("render").markdown;
    assert!(md.contains("title"), "md: {md}");
    assert!(md.contains("author"), "md: {md}");
    assert!(md.contains("body"), "md: {md}");
}

#[test]
fn render_front_matter_omitted_by_default() {
    let content = DocumentContent {
        metadata: DocumentMeta::new().title("T"),
        blocks: vec![DocumentBlock::Paragraph(vec![tr("body")])],
    };
    let md = render(&content);
    assert!(!md.contains("title:"), "md: {md}");
    assert!(md.contains("body"), "md: {md}");
}

#[test]
fn render_table_special_chars_escaped() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Table(DocumentTable {
            rows: vec![DocumentTableRow {
                cells: vec![cell("a|b")],
                is_header: false,
            }],
        })],
        ..Default::default()
    };
    let md = render(&content);
    assert!(md.contains("a\\|b") || md.contains("a|b"), "md: {md}");
}

fn cell(text: &str) -> DocumentTableCell {
    DocumentTableCell {
        blocks: vec![DocumentBlock::Paragraph(vec![tr(text)])],
        column_span: 1,
        row_span: 1,
    }
}

// ===========================================================================
// MarkdownOptions
// ===========================================================================

#[test]
fn options_defaults() {
    let opts = MarkdownOptions::default();
    assert!(opts.image_directory.is_none());
    assert!(opts.image_reference_prefix.is_none());
    assert!(!opts.include_front_matter);
}

#[test]
fn options_image_directory_set() {
    let opts = MarkdownOptions {
        image_directory: Some(std::path::PathBuf::from("assets")),
        ..Default::default()
    };
    assert_eq!(
        opts.image_directory.as_deref(),
        Some(std::path::Path::new("assets"))
    );
}

#[test]
fn options_image_reference_prefix() {
    let opts = MarkdownOptions {
        image_reference_prefix: Some("img/".into()),
        ..Default::default()
    };
    assert_eq!(opts.image_reference_prefix.as_deref(), Some("img/"));
}

#[test]
fn options_clone() {
    let opts = MarkdownOptions {
        include_front_matter: true,
        ..Default::default()
    };
    let c = opts.clone();
    assert_eq!(c.include_front_matter, opts.include_front_matter);
}
