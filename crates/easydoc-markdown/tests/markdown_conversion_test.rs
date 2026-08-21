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

#[test]
fn markdown_import_to_docx_preserves_math_and_footnote() {
    // MD → DocumentContent（导入）→ DOCX（writer 渲染）
    let md = "# Formulas\n\nInline $x^2$ and block:\n\n$$\\sum_{i=1}^n i$$\n\nNote here[^1].\n\n[^1]: Details.";
    let imported = easydoc_markdown::MarkdownImportBuilder::new(md)
        .do_import()
        .expect("import markdown");

    // 数学块存在
    assert!(
        imported
            .content
            .blocks
            .iter()
            .any(|b| matches!(b, DocumentBlock::Math { .. })),
        "expected Math block"
    );
    // 脚注块存在
    let footnote = imported
        .content
        .blocks
        .iter()
        .find(|b| matches!(b, DocumentBlock::Footnote { .. }))
        .expect("expected Footnote block");
    let DocumentBlock::Footnote { id, blocks } = footnote else {
        unreachable!()
    };
    assert_eq!(*id, 1);
    assert_eq!(blocks.len(), 1);

    // 渲染为 DOCX 再读回 Markdown，数学/脚注往返保留
    let dir = tempfile::tempdir().expect("tempdir");
    let docx_path = dir.path().join("roundtrip.docx");
    let docx = easydoc_writer::content_renderer::render_document_content(&imported.content)
        .expect("render docx");
    let math = easydoc_writer::content_renderer::take_rendered_math();
    let mut xml_docx = docx.build();
    let xml = String::from_utf8_lossy(&xml_docx.document).into_owned();
    xml_docx.document = easydoc_writer::math_omml::postprocess_math_xml(&xml, &math).into_bytes();
    xml_docx
        .pack(std::fs::File::create(&docx_path).expect("create file"))
        .expect("pack docx");

    // 公式经 OMML 写入 DOCX。office_oxide 语义路径不识别 OMML，
    // 用 sax 流式读取验证 Math 块还原 + 脚注文本保留。
    let mut reader = easydoc_reader::DocxSaxReader::from_path(&docx_path).expect("sax reader");
    let readback = reader.read_blocks().expect("sax read");
    // Math 块存在且 omml 可转回 LaTeX
    let math_blocks: Vec<&DocumentBlock> = readback
        .iter()
        .filter(|b| matches!(b, DocumentBlock::Math { .. }))
        .collect();
    assert!(
        !math_blocks.is_empty(),
        "sax should restore Math blocks: {readback:?}"
    );
    for b in &math_blocks {
        if let DocumentBlock::Math {
            omml: Some(omml), ..
        } = b
        {
            let latex =
                easydoc_markdown::math::omml_to_latex::convert(omml).expect("omml to latex");
            assert!(!latex.is_empty(), "latex should not be empty");
        }
    }
    // 脚注定义文本保留
    let all_text: String = readback.iter().fold(String::new(), |mut acc, b| {
        acc.push_str(&format!("{b:?}"));
        acc
    });
    assert!(
        all_text.contains("Details.") && all_text.contains("^1"),
        "footnote should survive, got: {all_text}"
    );
}

#[test]
fn markdown_html_tags_roundtrip_through_docx() {
    // MD（含 HTML 标签）→ DocumentContent → DOCX → Markdown，内容保留
    let md = "<strong>bold</strong> and <em>italic</em> and <a href=\"https://e.com\">link</a>";
    let imported = easydoc_markdown::MarkdownImportBuilder::new(md)
        .do_import()
        .expect("import");

    // 解析出 bold/italic/hyperlink run
    let DocumentBlock::Paragraph(runs) = &imported.content.blocks[0] else {
        panic!("expected Paragraph");
    };
    assert!(
        runs.iter().any(|r| r.bold && r.text == "bold"),
        "runs: {runs:?}"
    );
    assert!(
        runs.iter().any(|r| r.italic && r.text == "italic"),
        "runs: {runs:?}"
    );
    assert!(
        runs.iter()
            .any(|r| r.hyperlink.as_deref() == Some("https://e.com") && r.text == "link"),
        "runs: {runs:?}"
    );

    // 渲染为 DOCX 再读回，文本保留
    let dir = tempfile::tempdir().expect("tempdir");
    let docx_path = dir.path().join("html_rt.docx");
    easydoc_writer::content_renderer::render_document_content(&imported.content)
        .expect("render docx")
        .build()
        .pack(std::fs::File::create(&docx_path).expect("create file"))
        .expect("pack docx");

    let back = MarkdownBuilder::new(&docx_path)
        .do_convert()
        .expect("convert back");
    assert!(
        back.markdown.contains("bold") && back.markdown.contains("italic"),
        "text should survive roundtrip, got: {}",
        back.markdown
    );
}

#[test]
fn markdown_html_br_roundtrip() {
    let md = "line one<br>line two";
    let imported = easydoc_markdown::MarkdownImportBuilder::new(md)
        .do_import()
        .expect("import");
    let DocumentBlock::Paragraph(runs) = &imported.content.blocks[0] else {
        panic!("expected Paragraph");
    };
    let all: String = runs.iter().map(|r| r.text.as_str()).collect();
    assert!(
        all.contains("line one") && all.contains("line two"),
        "all: {all}"
    );
}

#[test]
fn markdown_html_hr_and_img_blocks() {
    let md = "before\n\n<hr>\n\n<img src=\"pic.png\" alt=\"A pic\">\n\nafter";
    let imported = easydoc_markdown::MarkdownImportBuilder::new(md)
        .do_import()
        .expect("import");
    assert!(
        imported
            .content
            .blocks
            .iter()
            .any(|b| matches!(b, DocumentBlock::ThematicBreak)),
        "expected ThematicBreak"
    );
    assert!(
        imported
            .content
            .blocks
            .iter()
            .any(|b| matches!(b, DocumentBlock::Image(_))),
        "expected Image"
    );
}

#[test]
fn markdown_nested_blockquote_roundtrip() {
    // 嵌套引用：MD → DocumentContent → DOCX → Markdown，内容保留
    let md = "> outer\n>> inner\n> back";
    let imported = easydoc_markdown::MarkdownImportBuilder::new(md)
        .do_import()
        .expect("import");

    // 解析为嵌套 TextBox
    let DocumentBlock::TextBox(blocks) = &imported.content.blocks[0] else {
        panic!("expected TextBox for nested blockquote");
    };
    assert_eq!(blocks.len(), 3, "outer + nested + outer: {blocks:?}");

    // 渲染为 DOCX 再读回，文本保留
    let dir = tempfile::tempdir().expect("tempdir");
    let docx_path = dir.path().join("bq_rt.docx");
    easydoc_writer::content_renderer::render_document_content(&imported.content)
        .expect("render docx")
        .build()
        .pack(std::fs::File::create(&docx_path).expect("create file"))
        .expect("pack docx");

    let back = MarkdownBuilder::new(&docx_path)
        .do_convert()
        .expect("convert back");
    assert!(
        back.markdown.contains("outer")
            && back.markdown.contains("inner")
            && back.markdown.contains("back"),
        "nested blockquote text should survive roundtrip, got: {}",
        back.markdown
    );
}
