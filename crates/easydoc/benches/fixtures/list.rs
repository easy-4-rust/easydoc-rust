//! Fixture 3：列表（无序 + 有序 + 嵌套）。

use easydoc::EasyDoc;
use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentList, DocumentListItem, DocumentTextRun,
};

use super::types::FidelityFixture;

/// 创建单条目列表项，包含纯文本段落。
fn list_item(text: &str) -> DocumentListItem {
    DocumentListItem {
        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
            text: text.into(),
            ..DocumentTextRun::default()
        }])],
        nested: None,
    }
}

/// 构建列表 fixture。
pub(super) fn build() -> FidelityFixture {
    // DocBuilder 没有 add_list；直接构造 DocumentContent。
    let content = DocumentContent {
        blocks: vec![
            // 3 项无序列表
            DocumentBlock::List(DocumentList {
                ordered: false,
                start_number: None,
                items: vec![
                    list_item("Unordered item one"),
                    list_item("Unordered item two"),
                    list_item("Unordered item three"),
                ],
            }),
            // 2 项有序列表，第二项含嵌套无序列表
            DocumentBlock::List(DocumentList {
                ordered: true,
                start_number: Some(1),
                items: vec![
                    list_item("Ordered item one"),
                    DocumentListItem {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "Ordered item two".into(),
                            ..DocumentTextRun::default()
                        }])],
                        nested: Some(Box::new(DocumentList {
                            ordered: false,
                            start_number: None,
                            items: vec![list_item("Nested unordered item")],
                        })),
                    },
                ],
            }),
        ],
        ..DocumentContent::default()
    };

    let bytes = EasyDoc::write_content_to_bytes(&content).expect("build list fixture");
    let expected = super::types::Fixtures::roundtrip_text(&bytes);

    FidelityFixture {
        name: "list",
        original_size: bytes.len() as u64,
        expected_text: expected,
        docx_bytes: bytes,
    }
}
