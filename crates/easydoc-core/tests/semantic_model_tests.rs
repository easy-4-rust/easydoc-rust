//! 语义模型类型（DocumentTextRun / `DocumentList` / `DocumentTable` 等）深度测试。

use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentImage, DocumentList, DocumentListItem, DocumentMeta,
    DocumentTable, DocumentTableCell, DocumentTableRow, DocumentTextRun,
};

fn run(text: &str) -> DocumentTextRun {
    DocumentTextRun {
        text: text.to_owned(),
        ..DocumentTextRun::default()
    }
}

// ===========================================================================
// DocumentTextRun
// ===========================================================================

#[test]
fn run_default_is_plain() {
    let r = DocumentTextRun::default();
    assert_eq!(r.text, "");
    assert!(!r.bold);
    assert!(!r.italic);
    assert!(!r.strikethrough);
    assert_eq!(r.hyperlink, None);
}

#[test]
fn run_builder_fields() {
    let r = DocumentTextRun {
        text: "x".into(),
        bold: true,
        italic: false,
        strikethrough: true,
        hyperlink: Some("https://e.com".into()),
    };
    assert!(r.bold && r.strikethrough);
    assert!(!r.italic);
    assert_eq!(r.hyperlink.as_deref(), Some("https://e.com"));
}

#[test]
fn run_clone_eq() {
    let a = run("hello");
    let b = a.clone();
    assert_eq!(a, b);
    assert_eq!(a.text, b.text);
}

#[test]
fn run_partial_eq_distinguishes_styles() {
    let plain = run("x");
    let bold = DocumentTextRun {
        bold: true,
        ..run("x")
    };
    assert_ne!(plain, bold);
    let italic = DocumentTextRun {
        italic: true,
        ..run("x")
    };
    assert_ne!(plain, italic);
    assert_ne!(bold, italic);
}

#[test]
fn run_hyperlink_variants() {
    let with_link = DocumentTextRun {
        hyperlink: Some("u".into()),
        ..run("t")
    };
    let without = run("t");
    assert_ne!(with_link, without);
}

// ===========================================================================
// DocumentList
// ===========================================================================

#[test]
fn list_default_unordered() {
    let l = DocumentList::default();
    assert!(!l.ordered);
    assert_eq!(l.start_number, None);
    assert!(l.items.is_empty());
}

#[test]
fn list_basic_ordered() {
    let l = DocumentList {
        ordered: true,
        start_number: Some(5),
        items: vec![DocumentListItem {
            blocks: vec![DocumentBlock::Paragraph(vec![run("a")])],
            nested: None,
        }],
    };
    assert!(l.ordered);
    assert_eq!(l.start_number, Some(5));
    assert_eq!(l.items.len(), 1);
}

#[test]
fn list_nested_structure() {
    let nested = DocumentList {
        ordered: false,
        start_number: None,
        items: vec![DocumentListItem {
            blocks: vec![DocumentBlock::Paragraph(vec![run("child")])],
            nested: None,
        }],
    };
    let parent = DocumentList {
        ordered: false,
        start_number: None,
        items: vec![DocumentListItem {
            blocks: vec![DocumentBlock::Paragraph(vec![run("parent")])],
            nested: Some(Box::new(nested)),
        }],
    };
    assert!(parent.items[0].nested.is_some());
}

#[test]
fn list_clone() {
    let l = DocumentList {
        ordered: true,
        start_number: None,
        items: vec![DocumentListItem {
            blocks: vec![],
            nested: None,
        }],
    };
    let c = l.clone();
    assert_eq!(c.ordered, l.ordered);
    assert_eq!(c.items.len(), l.items.len());
}

// ===========================================================================
// DocumentTable
// ===========================================================================

#[test]
fn table_empty() {
    let t = DocumentTable { rows: Vec::new() };
    assert!(t.rows.is_empty());
}

#[test]
fn table_with_rows() {
    let t = DocumentTable {
        rows: vec![DocumentTableRow {
            cells: vec![DocumentTableCell {
                blocks: vec![DocumentBlock::Paragraph(vec![run("c")])],
                column_span: 1,
                row_span: 1,
            }],
            is_header: true,
        }],
    };
    assert_eq!(t.rows.len(), 1);
    assert!(t.rows[0].is_header);
    assert_eq!(t.rows[0].cells.len(), 1);
}

#[test]
fn table_cell_span_fields() {
    let cell = DocumentTableCell {
        blocks: vec![],
        column_span: 3,
        row_span: 2,
    };
    assert_eq!(cell.column_span, 3);
    assert_eq!(cell.row_span, 2);
}

#[test]
fn table_cell_default() {
    let cell = DocumentTableCell::default();
    // derive(Default) 生成 0；读取端通过 .max(1) 归一化为 1
    assert_eq!(cell.column_span, 0);
    assert_eq!(cell.row_span, 0);
    assert!(cell.blocks.is_empty());
}

// ===========================================================================
// DocumentImage
// ===========================================================================

#[test]
fn image_fields() {
    let img = DocumentImage {
        alt_text: Some("logo".into()),
        data: Some(vec![1, 2, 3]),
        extension: Some("png".into()),
    };
    assert_eq!(img.alt_text.as_deref(), Some("logo"));
    assert_eq!(img.data.as_deref(), Some(&[1, 2, 3][..]));
    assert_eq!(img.extension.as_deref(), Some("png"));
}

#[test]
fn image_default() {
    let img = DocumentImage::default();
    assert_eq!(img.alt_text, None);
    assert_eq!(img.data, None);
    assert_eq!(img.extension, None);
}

#[test]
fn image_clone() {
    let img = DocumentImage {
        alt_text: Some("a".into()),
        data: Some(vec![9]),
        extension: None,
    };
    let c = img.clone();
    assert_eq!(img, c);
}

// ===========================================================================
// DocumentContent
// ===========================================================================

#[test]
fn content_default_empty() {
    let c = DocumentContent::default();
    assert!(c.blocks.is_empty());
    assert_eq!(c.metadata, DocumentMeta::default());
}

#[test]
fn content_with_blocks() {
    let c = DocumentContent {
        metadata: DocumentMeta::new().title("Doc"),
        blocks: vec![
            DocumentBlock::Paragraph(vec![run("p1")]),
            DocumentBlock::Heading {
                level: 1,
                runs: vec![run("h")],
            },
        ],
    };
    assert_eq!(c.blocks.len(), 2);
    assert_eq!(c.metadata.title.as_deref(), Some("Doc"));
}

#[test]
fn content_block_variants_constructible() {
    let blocks = vec![
        DocumentBlock::Paragraph(vec![]),
        DocumentBlock::Heading {
            level: 2,
            runs: vec![],
        },
        DocumentBlock::Table(DocumentTable { rows: vec![] }),
        DocumentBlock::List(DocumentList::default()),
        DocumentBlock::Image(DocumentImage::default()),
        DocumentBlock::ThematicBreak,
        DocumentBlock::PageBreak,
        DocumentBlock::ColumnBreak,
        DocumentBlock::CodeBlock {
            language: None,
            code: String::new(),
        },
    ];
    assert_eq!(blocks.len(), 9);
}

#[test]
fn content_clone() {
    let c = DocumentContent {
        metadata: DocumentMeta::new().title("T"),
        blocks: vec![DocumentBlock::Paragraph(vec![run("x")])],
    };
    let cloned = c.clone();
    assert_eq!(c, cloned);
}
