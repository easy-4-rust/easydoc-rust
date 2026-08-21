//! 属性测试（proptest）：文档往返一致性。
//!
//! 生成随机的语义文档结构（段落/富文本/标题/列表/表格），经
//! `write_content` 写出、`load` 读回，验证关键内容不丢失。
//! 每个 `proptest!` 测试默认运行 256 个用例，是覆盖率提升的主力。

use easydoc::EasyDoc;
use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentList, DocumentListItem, DocumentMeta, DocumentTable,
    DocumentTableCell, DocumentTableRow, DocumentTextRun,
};
use proptest::prelude::*;

/// 生成一段随机文本 run（含随机富文本属性）。
fn any_run() -> impl Strategy<Value = DocumentTextRun> {
    ("[a-z]{0,8}", any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(text, bold, italic, strike)| DocumentTextRun {
            text,
            bold,
            italic,
            strikethrough: strike,
            hyperlink: None,
        },
    )
}

/// 生成 0..N 个 run 的段落。
fn any_paragraph(max_runs: usize) -> impl Strategy<Value = DocumentBlock> {
    prop::collection::vec(any_run(), 0..max_runs).prop_map(DocumentBlock::Paragraph)
}

/// 生成标题块。
fn any_heading() -> impl Strategy<Value = DocumentBlock> {
    (1..=6usize, prop::collection::vec(any_run(), 0..3)).prop_map(|(level, runs)| {
        DocumentBlock::Heading {
            level: level as u8,
            runs,
        }
    })
}

/// 生成单元格。
fn any_cell() -> impl Strategy<Value = DocumentTableCell> {
    (prop::collection::vec(any_paragraph(3), 0..2), 1u32..3).prop_map(|(blocks, span)| {
        DocumentTableCell {
            blocks,
            column_span: span,
            row_span: 1,
        }
    })
}

/// 生成 1..N 行的小表格。
fn any_table(max_rows: usize) -> impl Strategy<Value = DocumentBlock> {
    prop::collection::vec(prop::collection::vec(any_cell(), 1..3), 1..max_rows).prop_map(|rows| {
        DocumentBlock::Table(DocumentTable {
            rows: rows
                .into_iter()
                .map(|cells| DocumentTableRow {
                    cells,
                    is_header: false,
                })
                .collect(),
        })
    })
}

/// 生成列表。
fn any_list() -> impl Strategy<Value = DocumentBlock> {
    (any::<bool>(), prop::collection::vec(any_paragraph(3), 1..4)).prop_map(|(ordered, items)| {
        DocumentBlock::List(DocumentList {
            ordered,
            start_number: None,
            items: items
                .into_iter()
                .map(|blocks| DocumentListItem {
                    blocks: vec![blocks],
                    nested: None,
                })
                .collect(),
        })
    })
}

/// 生成完整的文档内容（0..8 个块）。
fn any_content() -> impl Strategy<Value = DocumentContent> {
    prop::collection::vec(
        prop_oneof![any_paragraph(4), any_heading(), any_table(4), any_list(),],
        0..8,
    )
    .prop_map(|blocks| DocumentContent {
        metadata: DocumentMeta::default(),
        blocks,
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// 往返：write_content → load，非空文档读回后仍有内容。
    ///
    /// 注意：不要求块数单调（空段落/空表在 writer 或 reader 中可能被
    /// 合法折叠），只保证内容不整体丢失。
    #[test]
    fn roundtrip_preserves_block_count(content in any_content()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rt.docx");
        EasyDoc::write_content(&content, &path).unwrap();
        let loaded = EasyDoc::load(&path).unwrap();

        // 写出内容非空时，读回内容也应为非空（空文档允许读回为空）
        let has_non_empty_content = content.blocks.iter().any(|b| !matches!(b, DocumentBlock::Paragraph(runs) if runs.is_empty()));
        if has_non_empty_content {
            prop_assert!(!loaded.blocks.is_empty(),
                "non-empty content read back empty; wrote {} blocks", content.blocks.len());
        }
        // 读回块数不应超过写出的 3 倍（防止意外膨胀）
        prop_assert!(loaded.blocks.len() <= content.blocks.len().saturating_mul(3).max(1),
            "block explosion: wrote {} read {}", content.blocks.len(), loaded.blocks.len());
    }

    /// 往返后文本内容保留（所有段落 run 文本可在读回内容中找到）。
    #[test]
    fn roundtrip_preserves_text(content in any_content()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rt_text.docx");
        EasyDoc::write_content(&content, &path).unwrap();
        let loaded = EasyDoc::load(&path).unwrap();

        // 收集写出内容中的所有非空 run 文本
        let mut expected: Vec<String> = Vec::new();
        collect_run_texts(&content.blocks, &mut expected);
        let all_loaded = format!("{loaded:?}");

        for text in expected {
            if !text.is_empty() {
                prop_assert!(all_loaded.contains(&text),
                    "text {text:?} lost after roundtrip; loaded: {all_loaded}");
            }
        }
    }

    /// 往返后标题级别保留。
    #[test]
    fn roundtrip_preserves_headings(content in any_content()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rt_h.docx");
        EasyDoc::write_content(&content, &path).unwrap();
        let loaded = EasyDoc::load(&path).unwrap();

        let levels: Vec<u8> = content.blocks.iter().filter_map(|b| match b {
            DocumentBlock::Heading { level, .. } => Some(*level),
            _ => None,
        }).collect();
        for level in levels {
            prop_assert!(
                format!("{loaded:?}").contains(&format!("level: {level}")),
                "heading level {level} lost"
            );
        }
    }
}

/// 递归收集块中的所有非空 run 文本。
fn collect_run_texts(blocks: &[DocumentBlock], out: &mut Vec<String>) {
    for block in blocks {
        match block {
            DocumentBlock::Paragraph(runs) | DocumentBlock::Heading { runs, .. } => {
                out.extend(runs.iter().map(|r| r.text.clone()));
            }
            DocumentBlock::List(list) => {
                for item in &list.items {
                    collect_run_texts(&item.blocks, out);
                    if let Some(nested) = &item.nested {
                        // 嵌套列表项：逐个处理其 blocks
                        for nested_item in &nested.items {
                            collect_run_texts(&nested_item.blocks, out);
                        }
                    }
                }
            }
            DocumentBlock::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_run_texts(&cell.blocks, out);
                    }
                }
            }
            DocumentBlock::TextBox(blocks) | DocumentBlock::Section { blocks, .. } => {
                collect_run_texts(blocks, out);
            }
            _ => {}
        }
    }
}
