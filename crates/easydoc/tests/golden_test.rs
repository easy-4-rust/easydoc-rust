//! Golden test suite：代表性文档结构 → 确定性输出快照比对。
//!
//! 每个用例构造固定的 `DocumentContent`，写出 DOCX 后读回 Markdown，
//! 与 `tests/golden/*.md` 快照比对，防止解析/渲染回归。
//!
//! 更新快照：`UPDATE_GOLDEN=1 cargo test -p easydoc --test golden_test`
//! （快照仅在输出稳定后更新，PR 中应审查快照 diff）。

use std::path::PathBuf;

use easydoc::EasyDoc;
use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentList, DocumentListItem, DocumentMeta, DocumentTable,
    DocumentTableCell, DocumentTableRow, DocumentTextRun,
};

/// golden 快照目录。
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// 执行单个 golden 用例：渲染 → 读回 → 与快照比对。
fn check_golden(name: &str, content: &DocumentContent) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("{name}.docx"));
    EasyDoc::write_content(content, &path).unwrap();
    let markdown = EasyDoc::to_markdown(&path).unwrap();

    let snapshot_path = golden_dir().join(format!("{name}.md"));
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(golden_dir()).unwrap();
        // 快照统一用 \n 换行（Windows 上 docx-rs/ZIP 文本带 \r\n）
        std::fs::write(&snapshot_path, normalize_newlines(&markdown)).unwrap();
        return;
    }

    let expected = std::fs::read_to_string(&snapshot_path).unwrap_or_else(|_| {
        panic!(
            "missing golden snapshot {}; run UPDATE_GOLDEN=1 to create",
            snapshot_path.display()
        )
    });
    assert_eq!(
        normalize_newlines(&expected),
        normalize_newlines(&markdown),
        "golden mismatch for {name} (snapshot: {})",
        snapshot_path.display()
    );
}

/// 归一化换行：`\r\n` → `\n`（Windows 上 ZIP 文本读取带 CRLF）。
fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/// 文本 run 辅助。
fn tr(text: &str) -> DocumentTextRun {
    DocumentTextRun {
        text: text.to_owned(),
        ..DocumentTextRun::default()
    }
}

/// 带格式 run 辅助。
fn fmt_run(text: &str, bold: bool, italic: bool) -> DocumentTextRun {
    DocumentTextRun {
        text: text.to_owned(),
        bold,
        italic,
        ..DocumentTextRun::default()
    }
}

#[test]
fn golden_heading_paragraph() {
    let content = DocumentContent {
        metadata: DocumentMeta::new().title("Golden Doc").author("Tester"),
        blocks: vec![
            DocumentBlock::Heading {
                level: 1,
                runs: vec![tr("Main Title")],
            },
            DocumentBlock::Paragraph(vec![
                tr("Plain text. "),
                fmt_run("bold", true, false),
                tr(" and "),
                fmt_run("italic", false, true),
                tr("."),
            ]),
        ],
    };
    check_golden("heading_paragraph", &content);
}

#[test]
fn golden_table_basic() {
    let content = DocumentContent {
        metadata: DocumentMeta::default(),
        blocks: vec![DocumentBlock::Table(DocumentTable {
            rows: vec![
                DocumentTableRow {
                    cells: vec![cell("Name", 1), cell("Value", 1)],
                    is_header: true,
                },
                DocumentTableRow {
                    cells: vec![cell("A", 1), cell("1", 1)],
                    is_header: false,
                },
                DocumentTableRow {
                    cells: vec![cell("B", 1), cell("2", 1)],
                    is_header: false,
                },
            ],
        })],
    };
    check_golden("table_basic", &content);
}

#[test]
fn golden_table_merged_cells() {
    let content = DocumentContent {
        metadata: DocumentMeta::default(),
        blocks: vec![DocumentBlock::Table(DocumentTable {
            rows: vec![
                DocumentTableRow {
                    cells: vec![cell("Span 2", 2), cell("C", 1)],
                    is_header: false,
                },
                DocumentTableRow {
                    cells: vec![cell("D", 1), cell("E", 1)],
                    is_header: false,
                },
            ],
        })],
    };
    check_golden("table_merged", &content);
}

#[test]
fn golden_lists() {
    let content = DocumentContent {
        metadata: DocumentMeta::default(),
        blocks: vec![
            DocumentBlock::List(DocumentList {
                ordered: false,
                start_number: None,
                items: vec![
                    DocumentListItem {
                        blocks: vec![DocumentBlock::Paragraph(vec![tr("Unordered 1")])],
                        nested: None,
                    },
                    DocumentListItem {
                        blocks: vec![DocumentBlock::Paragraph(vec![tr("Unordered 2")])],
                        nested: Some(Box::new(DocumentList {
                            ordered: false,
                            start_number: None,
                            items: vec![DocumentListItem {
                                blocks: vec![DocumentBlock::Paragraph(vec![tr("Nested")])],
                                nested: None,
                            }],
                        })),
                    },
                ],
            }),
            DocumentBlock::List(DocumentList {
                ordered: true,
                start_number: Some(3),
                items: vec![DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![tr("Starts at 3")])],
                    nested: None,
                }],
            }),
        ],
    };
    check_golden("lists", &content);
}

#[test]
fn golden_code_and_break() {
    let content = DocumentContent {
        metadata: DocumentMeta::default(),
        blocks: vec![
            DocumentBlock::CodeBlock {
                language: Some("rust".into()),
                code: "fn main() {}\n".into(),
            },
            DocumentBlock::ThematicBreak,
            DocumentBlock::Paragraph(vec![tr("After break")]),
        ],
    };
    check_golden("code_and_break", &content);
}

/// 单元格辅助。
fn cell(text: &str, column_span: u32) -> DocumentTableCell {
    DocumentTableCell {
        blocks: vec![DocumentBlock::Paragraph(vec![tr(text)])],
        column_span,
        row_span: 1,
    }
}
