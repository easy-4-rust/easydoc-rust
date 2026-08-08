//! Markdown 语义渲染与真实 DOCX 端到端转换测试。

use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentImage, DocumentList, DocumentListItem, DocumentMeta,
    DocumentTable, DocumentTableCell, DocumentTableRow, DocumentTextRun, HeadingLevel,
};
use easydoc_markdown::{MarkdownBuilder, MarkdownOptions, render_document};
use easydoc_writer::{DocBuilder, Paragraph, Run};

fn paragraph(text: &str) -> DocumentBlock {
    DocumentBlock::Paragraph(vec![DocumentTextRun {
        text: text.to_owned(),
        ..DocumentTextRun::default()
    }])
}

fn cell(text: &str, column_span: u32) -> DocumentTableCell {
    DocumentTableCell {
        blocks: vec![paragraph(text)],
        column_span,
        row_span: 1,
    }
}

#[test]
fn renders_semantics_tables_lists_and_images() {
    let directory = tempfile::tempdir().expect("tempdir");
    let image_directory = directory.path().join("assets");
    let document = DocumentContent {
        metadata: DocumentMeta::default()
            .title("Quarterly Report")
            .author("Alice"),
        blocks: vec![
            DocumentBlock::Heading {
                level: 2,
                runs: vec![DocumentTextRun {
                    text: "Summary".to_owned(),
                    bold: true,
                    ..DocumentTextRun::default()
                }],
            },
            DocumentBlock::Table(DocumentTable {
                rows: vec![
                    DocumentTableRow {
                        cells: vec![cell("Name", 1), cell("Value", 1)],
                        is_header: true,
                    },
                    DocumentTableRow {
                        cells: vec![cell("A|B", 1), cell("42", 1)],
                        is_header: false,
                    },
                ],
            }),
            DocumentBlock::Table(DocumentTable {
                rows: vec![DocumentTableRow {
                    cells: vec![cell("Merged", 2)],
                    is_header: false,
                }],
            }),
            DocumentBlock::List(DocumentList {
                ordered: true,
                start_number: Some(3),
                items: vec![DocumentListItem {
                    blocks: vec![paragraph("First")],
                    nested: Some(Box::new(DocumentList {
                        ordered: false,
                        start_number: None,
                        items: vec![DocumentListItem {
                            blocks: vec![paragraph("Nested")],
                            nested: None,
                        }],
                    })),
                }],
            }),
            DocumentBlock::Image(DocumentImage {
                alt_text: Some("Chart".to_owned()),
                data: Some(vec![0, 1, 2, 255]),
                extension: Some("png".to_owned()),
            }),
        ],
    };

    let result = render_document(
        &document,
        MarkdownOptions {
            image_directory: Some(image_directory),
            image_reference_prefix: Some("media".to_owned()),
            include_front_matter: true,
        },
    )
    .expect("render Markdown");

    assert!(result.markdown.contains("title: 'Quarterly Report'"));
    assert!(result.markdown.contains("## **Summary**"));
    assert!(result.markdown.contains("| A\\|B | 42 |"));
    assert!(result.markdown.contains("<td colspan=\"2\">Merged</td>"));
    assert!(result.markdown.contains("3. First"));
    assert!(result.markdown.contains("  - Nested"));
    assert!(result.markdown.contains("![Chart](media/image_0001.png)"));
    assert_eq!(result.assets.len(), 1);
    assert_eq!(
        std::fs::read(&result.assets[0].path).expect("read asset"),
        vec![0, 1, 2, 255]
    );
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn converts_a_real_generated_docx_and_writes_atomically() {
    let directory = tempfile::tempdir().expect("tempdir");
    let source = directory.path().join("source.docx");
    let output = directory.path().join("source.md");
    DocBuilder::new(&source)
        .add_heading("Generated Report", HeadingLevel::H2)
        .add_paragraph(
            Paragraph::new()
                .add_text("Hello ")
                .add_run(Run::new("DOCX").bold()),
        )
        .save()
        .expect("write DOCX");

    let result = MarkdownBuilder::new(&source)
        .write_to(&output)
        .expect("convert DOCX");

    assert!(result.markdown.contains("Hello"));
    assert!(
        result.markdown.contains("## **Generated Report**"),
        "{}",
        result.markdown
    );
    assert!(result.markdown.contains("**DOCX**"));
    assert_eq!(
        std::fs::read_to_string(output).expect("read Markdown"),
        result.markdown
    );
}
