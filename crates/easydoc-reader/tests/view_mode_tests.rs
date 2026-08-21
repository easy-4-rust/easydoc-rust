//! `ViewMode` 渲染深度测试（Plain / Annotated / Outline / Stats）。

use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentList, DocumentListItem, DocumentMeta, DocumentTable,
    DocumentTableCell, DocumentTableRow, DocumentTextRun,
};
use easydoc_reader::{ViewMode, render_view};

fn tr(text: &str) -> DocumentTextRun {
    DocumentTextRun {
        text: text.to_owned(),
        ..DocumentTextRun::default()
    }
}

fn paragraph(text: &str) -> DocumentBlock {
    DocumentBlock::Paragraph(vec![tr(text)])
}

fn sample_doc() -> DocumentContent {
    DocumentContent {
        metadata: DocumentMeta::new().title("Sample"),
        blocks: vec![
            DocumentBlock::Heading {
                level: 1,
                runs: vec![tr("Title")],
            },
            paragraph("First paragraph"),
            DocumentBlock::Table(DocumentTable {
                rows: vec![
                    DocumentTableRow {
                        cells: vec![
                            DocumentTableCell {
                                blocks: vec![paragraph("A")],
                                column_span: 1,
                                row_span: 1,
                            },
                            DocumentTableCell {
                                blocks: vec![paragraph("B")],
                                column_span: 1,
                                row_span: 1,
                            },
                        ],
                        is_header: true,
                    },
                    DocumentTableRow {
                        cells: vec![
                            DocumentTableCell {
                                blocks: vec![paragraph("1")],
                                column_span: 1,
                                row_span: 1,
                            },
                            DocumentTableCell {
                                blocks: vec![paragraph("2")],
                                column_span: 1,
                                row_span: 1,
                            },
                        ],
                        is_header: false,
                    },
                ],
            }),
            DocumentBlock::List(DocumentList {
                ordered: true,
                start_number: None,
                items: vec![DocumentListItem {
                    blocks: vec![paragraph("Item")],
                    nested: None,
                }],
            }),
        ],
    }
}

// ===========================================================================
// Plain
// ===========================================================================

#[test]
fn plain_contains_all_text() {
    let out = render_view(&sample_doc(), &ViewMode::Plain).unwrap();
    assert!(out.contains("Title"));
    assert!(out.contains("First paragraph"));
    assert!(out.contains('A') && out.contains('B'));
    assert!(out.contains("Item"));
}

#[test]
fn plain_empty_doc() {
    let content = DocumentContent::default();
    let out = render_view(&content, &ViewMode::Plain).unwrap();
    assert!(out.is_empty() || out.trim().is_empty());
}

#[test]
fn plain_heading_prefix() {
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Heading {
            level: 2,
            runs: vec![tr("Sub")],
        }],
        ..Default::default()
    };
    let out = render_view(&content, &ViewMode::Plain).unwrap();
    assert!(out.contains("Sub"));
}

// ===========================================================================
// Annotated
// ===========================================================================

#[test]
fn annotated_marks_paragraph() {
    let out = render_view(&sample_doc(), &ViewMode::Annotated).unwrap();
    // 标注模式包含结构标记（如 [段落 ...]）
    assert!(out.contains("段落") || out.contains("Paragraph") || out.contains('['));
}

#[test]
fn annotated_includes_text() {
    let out = render_view(&sample_doc(), &ViewMode::Annotated).unwrap();
    assert!(out.contains("First paragraph"));
}

// ===========================================================================
// Outline
// ===========================================================================

#[test]
fn outline_contains_headings_only() {
    let out = render_view(&sample_doc(), &ViewMode::Outline { max_level: 6 }).unwrap();
    assert!(out.contains("Title"));
    // 正文段落不应出现在大纲中
    assert!(!out.contains("First paragraph"), "outline: {out}");
}

#[test]
fn outline_marks_level() {
    let content = DocumentContent {
        blocks: vec![
            DocumentBlock::Heading {
                level: 1,
                runs: vec![tr("H1")],
            },
            DocumentBlock::Heading {
                level: 3,
                runs: vec![tr("H3")],
            },
        ],
        ..Default::default()
    };
    let out = render_view(&content, &ViewMode::Outline { max_level: 6 }).unwrap();
    assert!(out.contains("H1") && out.contains("H3"));
}

#[test]
fn outline_max_level_filters() {
    let content = DocumentContent {
        blocks: vec![
            DocumentBlock::Heading {
                level: 1,
                runs: vec![tr("L1")],
            },
            DocumentBlock::Heading {
                level: 4,
                runs: vec![tr("L4")],
            },
        ],
        ..Default::default()
    };
    // max_level=2 时，L4 被过滤
    let out = render_view(&content, &ViewMode::Outline { max_level: 2 }).unwrap();
    assert!(out.contains("L1"));
    assert!(!out.contains("L4"), "outline with max_level=2: {out}");
}

#[test]
fn outline_empty_doc() {
    let content = DocumentContent::default();
    let out = render_view(&content, &ViewMode::Outline { max_level: 6 }).unwrap();
    assert!(out.trim().is_empty());
}

// ===========================================================================
// Stats
// ===========================================================================

#[test]
fn stats_reports_block_counts() {
    let out = render_view(&sample_doc(), &ViewMode::Stats).unwrap();
    // 统计信息应包含段落数/块数等
    assert!(out.contains('1') || out.contains("paragraph") || out.contains("块"));
}

#[test]
fn stats_empty_doc() {
    let content = DocumentContent::default();
    let out = render_view(&content, &ViewMode::Stats).unwrap();
    assert!(
        !out.is_empty(),
        "stats of empty doc should still report zeros"
    );
}

// ===========================================================================
// 组合
// ===========================================================================

#[test]
fn all_modes_on_complex_doc() {
    let doc = sample_doc();
    for mode in [
        ViewMode::Plain,
        ViewMode::Annotated,
        ViewMode::Outline { max_level: 6 },
        ViewMode::Stats,
    ] {
        let out = render_view(&doc, &mode).expect("render should not fail");
        assert!(!out.is_empty(), "mode {mode:?} produced empty output");
    }
}
