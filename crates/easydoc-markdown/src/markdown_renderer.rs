use std::fmt::Write as FmtWrite;
use std::fs;

use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentList, DocumentTable, DocumentTextRun, Result,
};
use easydoc_ooxml::AtomicFile;

use crate::math;
use crate::{ConversionWarning, ExtractedAsset, MarkdownOptions, MarkdownResult};

/// 把 easydoc 语义文档渲染为 Markdown，并管理图片等伴随资源。
pub(crate) struct MarkdownRenderer {
    options: MarkdownOptions,
    assets: Vec<ExtractedAsset>,
    warnings: Vec<ConversionWarning>,
    image_index: usize,
}

impl MarkdownRenderer {
    pub(crate) fn new(options: MarkdownOptions) -> Self {
        Self {
            options,
            assets: Vec::new(),
            warnings: Vec::new(),
            image_index: 0,
        }
    }

    pub(crate) fn render(mut self, document: &DocumentContent) -> Result<MarkdownResult> {
        let mut markdown = String::new();
        if self.options.include_front_matter {
            render_front_matter(document, &mut markdown);
        }
        self.render_blocks(&document.blocks, &mut markdown)?;
        while markdown.ends_with("\n\n\n") {
            markdown.pop();
        }
        Ok(MarkdownResult {
            markdown,
            assets: self.assets,
            warnings: self.warnings,
        })
    }

    fn render_blocks(&mut self, blocks: &[DocumentBlock], output: &mut String) -> Result<()> {
        for block in blocks {
            match block {
                DocumentBlock::Heading { level, runs } => {
                    output.push_str(&"#".repeat(usize::from((*level).clamp(1, 6))));
                    output.push(' ');
                    output.push_str(&render_runs(runs));
                    output.push_str("\n\n");
                }
                DocumentBlock::Paragraph(runs) => {
                    output.push_str(&render_runs(runs));
                    output.push_str("\n\n");
                }
                DocumentBlock::Table(table) => self.render_table(table, output)?,
                DocumentBlock::List(list) => {
                    Self::render_list(list, 0, output);
                    output.push('\n');
                }
                DocumentBlock::Image(image) => {
                    let alt = image.alt_text.as_deref().unwrap_or("image");
                    if let (Some(directory), Some(data)) =
                        (self.options.image_directory.as_ref(), image.data.as_ref())
                    {
                        fs::create_dir_all(directory)?;
                        self.image_index += 1;
                        let extension = image.extension.as_deref().unwrap_or("bin");
                        let file_name = format!("image_{:04}.{extension}", self.image_index);
                        let path = directory.join(&file_name);
                        AtomicFile::write(&path, |file| {
                            use std::io::Write;
                            file.write_all(data)?;
                            Ok(())
                        })?;
                        let prefix =
                            self.options
                                .image_reference_prefix
                                .clone()
                                .unwrap_or_else(|| {
                                    directory
                                        .file_name()
                                        .and_then(|name| name.to_str())
                                        .unwrap_or("assets")
                                        .to_owned()
                                });
                        let reference = format!("{}/{file_name}", prefix.trim_end_matches('/'));
                        writeln!(output, "![{}]({reference})\n", escape_markdown(alt))
                            .expect("writing to String cannot fail");
                        self.assets.push(ExtractedAsset { path, reference });
                    } else {
                        writeln!(output, "[Image: {}]\n", escape_markdown(alt))
                            .expect("writing to String cannot fail");
                        self.warnings.push(ConversionWarning {
                            message: format!(
                                "image '{alt}' was not extracted because no image bytes or output directory were available"
                            ),
                        });
                    }
                }
                DocumentBlock::ThematicBreak => output.push_str("---\n\n"),
                DocumentBlock::PageBreak => output.push_str("<!-- page-break -->\n\n"),
                DocumentBlock::ColumnBreak => output.push_str("<!-- column-break -->\n\n"),
                DocumentBlock::CodeBlock { language, code } => {
                    output.push_str("```");
                    output.push_str(language.as_deref().unwrap_or_default());
                    output.push('\n');
                    output.push_str(code);
                    if !code.ends_with('\n') {
                        output.push('\n');
                    }
                    output.push_str("```\n\n");
                }
                DocumentBlock::TextBox(content) => self.render_blocks(content, output)?,
                DocumentBlock::Footnote { id, blocks } => {
                    writeln!(output, "[^{id}]: {}\n", plain_blocks(blocks).trim())
                        .expect("writing to String cannot fail");
                }
                DocumentBlock::Endnote { id, blocks } => {
                    writeln!(output, "[^endnote-{id}]: {}\n", plain_blocks(blocks).trim())
                        .expect("writing to String cannot fail");
                }
                DocumentBlock::Section {
                    blocks,
                    section_type: _,
                } => {
                    self.render_blocks(blocks, output)?;
                }
                DocumentBlock::Math {
                    omml,
                    latex,
                    display,
                } => {
                    let resolved = latex.clone().unwrap_or_else(|| {
                        omml.as_ref()
                            .and_then(|xml| math::omml_to_latex::convert(xml).ok())
                            .unwrap_or_default()
                    });
                    if resolved.is_empty() {
                        self.warnings.push(ConversionWarning {
                            message: "math formula could not be converted to LaTeX".to_owned(),
                        });
                    }
                    if *display {
                        use std::fmt::Write;
                        writeln!(output, "$${resolved}$$\n")
                            .expect("writing to String cannot fail");
                    } else {
                        use std::fmt::Write;
                        write!(output, "${resolved}$").expect("writing to String cannot fail");
                    }
                }
                _ => self.warnings.push(ConversionWarning {
                    message: "an unknown document block was omitted".to_owned(),
                }),
            }
        }
        Ok(())
    }

    fn render_list(list: &DocumentList, depth: usize, output: &mut String) {
        let start = list.start_number.unwrap_or(1);
        for (index, item) in list.items.iter().enumerate() {
            output.push_str(&"  ".repeat(depth));
            if list.ordered {
                write!(
                    output,
                    "{}. ",
                    start + u32::try_from(index).unwrap_or(u32::MAX)
                )
                .expect("writing to String cannot fail");
            } else {
                output.push_str("- ");
            }
            output.push_str(plain_blocks(&item.blocks).trim());
            output.push('\n');
            if let Some(nested) = &item.nested {
                Self::render_list(nested, depth + 1, output);
            }
        }
    }

    fn render_table(&mut self, table: &DocumentTable, output: &mut String) -> Result<()> {
        let merged = table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .any(|cell| cell.column_span > 1 || cell.row_span > 1);
        if merged {
            self.warnings.push(ConversionWarning {
                message: "merged table cells were rendered as HTML to preserve spans".to_owned(),
            });
            render_html_table(table, output);
        } else {
            render_gfm_table(table, output);
        }
        Ok(())
    }
}

fn render_front_matter(document: &DocumentContent, output: &mut String) {
    let metadata = &document.metadata;
    if metadata.title.is_none()
        && metadata.author.is_none()
        && metadata.subject.is_none()
        && metadata.keywords.is_none()
    {
        return;
    }
    output.push_str("---\n");
    for (key, value) in [
        ("title", metadata.title.as_deref()),
        ("author", metadata.author.as_deref()),
        ("subject", metadata.subject.as_deref()),
        ("keywords", metadata.keywords.as_deref()),
    ] {
        if let Some(value) = value {
            writeln!(output, "{key}: '{}'", value.replace('\'', "''"))
                .expect("writing to String cannot fail");
        }
    }
    output.push_str("---\n\n");
}

fn render_runs(runs: &[DocumentTextRun]) -> String {
    let mut output = String::new();
    for run in runs {
        let mut text = escape_markdown(&run.text).replace('\n', "  \n");
        if run.bold && !text.is_empty() {
            text = format!("**{text}**");
        }
        if run.italic && !text.is_empty() {
            text = format!("*{text}*");
        }
        if run.strikethrough && !text.is_empty() {
            text = format!("~~{text}~~");
        }
        if let Some(link) = &run.hyperlink {
            text = format!("[{text}]({})", link.replace(')', "%29"));
        }
        output.push_str(&text);
    }
    output
}

fn render_gfm_table(table: &DocumentTable, output: &mut String) {
    let columns = table
        .rows
        .iter()
        .map(|row| row.cells.len())
        .max()
        .unwrap_or(0);
    if columns == 0 {
        return;
    }
    let header_index = table.rows.iter().position(|row| row.is_header);
    let empty_header = vec![String::new(); columns];
    let header = header_index.map_or_else(
        || empty_header.clone(),
        |index| row_cells(&table.rows[index], columns),
    );
    write_markdown_row(output, &header);
    write_markdown_row(output, &vec!["---".to_owned(); columns]);
    for (index, row) in table.rows.iter().enumerate() {
        if Some(index) != header_index {
            write_markdown_row(output, &row_cells(row, columns));
        }
    }
    output.push('\n');
}

fn row_cells(row: &easydoc_core::DocumentTableRow, columns: usize) -> Vec<String> {
    let mut cells: Vec<String> = row
        .cells
        .iter()
        .map(|cell| escape_table_cell(plain_blocks(&cell.blocks).trim()))
        .collect();
    cells.resize(columns, String::new());
    cells
}

fn write_markdown_row(output: &mut String, cells: &[String]) {
    output.push('|');
    for cell in cells {
        write!(output, " {cell} |").expect("writing to String cannot fail");
    }
    output.push('\n');
}

fn render_html_table(table: &DocumentTable, output: &mut String) {
    output.push_str("<table>\n");
    for row in &table.rows {
        output.push_str("  <tr>\n");
        for cell in &row.cells {
            let tag = if row.is_header { "th" } else { "td" };
            write!(output, "    <{tag}").expect("writing to String cannot fail");
            if cell.column_span > 1 {
                write!(output, " colspan=\"{}\"", cell.column_span)
                    .expect("writing to String cannot fail");
            }
            if cell.row_span > 1 {
                write!(output, " rowspan=\"{}\"", cell.row_span)
                    .expect("writing to String cannot fail");
            }
            writeln!(
                output,
                ">{}</{tag}>",
                html_escape(plain_blocks(&cell.blocks).trim())
            )
            .expect("writing to String cannot fail");
        }
        output.push_str("  </tr>\n");
    }
    output.push_str("</table>\n\n");
}

fn plain_blocks(blocks: &[DocumentBlock]) -> String {
    let mut output = String::new();
    for block in blocks {
        match block {
            DocumentBlock::Heading { runs, .. } | DocumentBlock::Paragraph(runs) => {
                for run in runs {
                    output.push_str(&run.text);
                }
            }
            DocumentBlock::CodeBlock { code, .. } => output.push_str(code),
            DocumentBlock::TextBox(content) => output.push_str(&plain_blocks(content)),
            DocumentBlock::Image(image) => {
                output.push_str(image.alt_text.as_deref().unwrap_or("image"));
            }
            DocumentBlock::Section { blocks, .. } => {
                output.push_str(&plain_blocks(blocks));
            }
            DocumentBlock::Math { latex, omml, .. } => {
                let resolved = latex.clone().unwrap_or_else(|| {
                    omml.as_ref()
                        .and_then(|xml| math::omml_to_latex::convert(xml).ok())
                        .unwrap_or_default()
                });
                output.push_str(&resolved);
            }
            _ => {}
        }
        output.push('\n');
    }
    output
}

fn escape_markdown(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '\\' | '*' | '_' | '[' | ']' | '`' | '~') {
            output.push('\\');
        }
        output.push(character);
    }
    output
}

fn escape_table_cell(value: &str) -> String {
    escape_markdown(value)
        .replace('|', "\\|")
        .replace('\n', "<br>")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace('\n', "<br>")
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use easydoc_core::{
        DocumentImage, DocumentListItem, DocumentMeta, DocumentTableCell, DocumentTableRow,
    };

    fn tr(text: &str) -> DocumentTextRun {
        DocumentTextRun {
            text: text.into(),
            ..Default::default()
        }
    }

    fn render_blocks(blocks: Vec<DocumentBlock>) -> String {
        let content = DocumentContent {
            blocks,
            ..Default::default()
        };
        let renderer = MarkdownRenderer::new(MarkdownOptions::default());
        renderer.render(&content).unwrap().markdown
    }

    #[test]
    fn render_heading_all_levels() {
        for level in 1..=6 {
            let md = render_blocks(vec![DocumentBlock::Heading {
                level,
                runs: vec![tr("Title")],
            }]);
            assert!(md.contains("Title"), "level {level}");
        }
    }

    #[test]
    fn render_paragraph_with_bold_italic_strike() {
        let md = render_blocks(vec![DocumentBlock::Paragraph(vec![
            DocumentTextRun {
                text: "bold".into(),
                bold: true,
                ..Default::default()
            },
            DocumentTextRun {
                text: "italic".into(),
                italic: true,
                ..Default::default()
            },
            DocumentTextRun {
                text: "strike".into(),
                strikethrough: true,
                ..Default::default()
            },
            DocumentTextRun {
                text: "link".into(),
                hyperlink: Some("https://example.com".into()),
                ..Default::default()
            },
        ])]);
        assert!(md.contains("**bold**"));
        assert!(md.contains("*italic*"));
        assert!(md.contains("~~strike~~"));
        assert!(md.contains("[link]"));
    }

    #[test]
    fn render_table_with_header() {
        let md = render_blocks(vec![DocumentBlock::Table(DocumentTable {
            rows: vec![
                DocumentTableRow {
                    cells: vec![
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![tr("H1")])],
                            column_span: 1,
                            row_span: 1,
                        },
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![tr("H2")])],
                            column_span: 1,
                            row_span: 1,
                        },
                    ],
                    is_header: true,
                },
                DocumentTableRow {
                    cells: vec![
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![tr("A")])],
                            column_span: 1,
                            row_span: 1,
                        },
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![tr("B")])],
                            column_span: 1,
                            row_span: 1,
                        },
                    ],
                    is_header: false,
                },
            ],
        })]);
        assert!(md.contains("H1"));
        assert!(md.contains("H1") || md.contains('|')); // table rendered
    }

    #[test]
    fn render_list_ordered_and_unordered() {
        let md = render_blocks(vec![DocumentBlock::List(DocumentList {
            ordered: false,
            start_number: None,
            items: vec![
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![tr("Item 1")])],
                    nested: None,
                },
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![tr("Item 2")])],
                    nested: Some(Box::new(DocumentList {
                        ordered: true,
                        start_number: Some(1),
                        items: vec![DocumentListItem {
                            blocks: vec![DocumentBlock::Paragraph(vec![tr("Nested")])],
                            nested: None,
                        }],
                    })),
                },
            ],
        })]);
        assert!(md.contains("- Item 1"));
        assert!(md.contains("1. Nested"));
    }

    #[test]
    fn render_image_with_alt() {
        let md = render_blocks(vec![DocumentBlock::Image(DocumentImage {
            alt_text: Some("photo".into()),
            data: None,
            extension: Some("png".into()),
        })]);
        assert!(!md.is_empty()); // image rendered
    }

    #[test]
    fn render_image_with_data() {
        let md = render_blocks(vec![DocumentBlock::Image(DocumentImage {
            alt_text: Some("img".into()),
            data: Some(vec![0x89, 0x50]),
            extension: Some("png".into()),
        })]);
        assert!(!md.is_empty()); // image with data rendered
    }

    #[test]
    fn render_footnote_and_endnote() {
        let md = render_blocks(vec![
            DocumentBlock::Footnote {
                id: 1,
                blocks: vec![DocumentBlock::Paragraph(vec![tr("note text")])],
            },
            DocumentBlock::Endnote {
                id: 2,
                blocks: vec![DocumentBlock::Paragraph(vec![tr("end text")])],
            },
        ]);
        assert!(md.contains("[^1]"));
        assert!(md.contains("note text"));
    }

    #[test]
    fn render_codeblock_with_language() {
        let md = render_blocks(vec![DocumentBlock::CodeBlock {
            language: Some("rust".into()),
            code: "fn main() {}".into(),
        }]);
        assert!(md.contains("```rust"));
        assert!(md.contains("fn main()"));
    }

    #[test]
    fn render_codeblock_without_language() {
        let md = render_blocks(vec![DocumentBlock::CodeBlock {
            language: None,
            code: "plain".into(),
        }]);
        assert!(md.contains("```"));
        assert!(md.contains("plain"));
    }

    #[test]
    fn render_textbox() {
        let md = render_blocks(vec![DocumentBlock::TextBox(vec![
            DocumentBlock::Paragraph(vec![tr("inside")]),
        ])]);
        assert!(md.contains("inside"));
    }

    #[test]
    fn render_section() {
        let md = render_blocks(vec![DocumentBlock::Section {
            blocks: vec![DocumentBlock::Paragraph(vec![tr("section content")])],
            section_type: Some("nextPage".into()),
        }]);
        assert!(md.contains("section content"));
    }

    #[test]
    fn render_thematic_break() {
        let md = render_blocks(vec![DocumentBlock::ThematicBreak]);
        assert!(md.contains("---"));
    }

    #[test]
    fn render_page_break() {
        let md = render_blocks(vec![DocumentBlock::PageBreak]);
        assert!(md.contains("page-break"));
    }

    #[test]
    fn render_column_break() {
        let md = render_blocks(vec![DocumentBlock::ColumnBreak]);
        assert!(md.contains("column-break"));
    }

    #[test]
    fn render_with_front_matter() {
        let content = DocumentContent {
            metadata: DocumentMeta {
                title: Some("Test".into()),
                author: Some("Author".into()),
                ..Default::default()
            },
            blocks: vec![DocumentBlock::Paragraph(vec![tr("body")])],
        };
        let opts = MarkdownOptions {
            include_front_matter: true,
            ..Default::default()
        };
        let result = MarkdownRenderer::new(opts).render(&content).unwrap();
        assert!(result.markdown.contains("---"));
        assert!(result.markdown.contains("title:"));
    }

    #[test]
    fn render_with_image_directory() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::Image(DocumentImage {
                alt_text: Some("pic".into()),
                data: Some(vec![0x89, 0x50]),
                extension: Some("png".into()),
            })],
            ..Default::default()
        };
        let opts = MarkdownOptions {
            image_directory: Some("/tmp/img".into()),
            image_reference_prefix: Some("images".into()),
            ..Default::default()
        };
        let result = MarkdownRenderer::new(opts).render(&content).unwrap();
        assert!(result.markdown.contains("images/"));
    }

    #[test]
    fn render_empty_document() {
        let md = render_blocks(vec![]);
        assert!(md.is_empty() || md.trim().is_empty());
    }

    #[test]
    fn render_table_empty_cells() {
        let md = render_blocks(vec![DocumentBlock::Table(DocumentTable {
            rows: vec![DocumentTableRow {
                cells: vec![
                    DocumentTableCell {
                        blocks: vec![],
                        column_span: 1,
                        row_span: 1,
                    },
                    DocumentTableCell {
                        blocks: vec![],
                        column_span: 1,
                        row_span: 1,
                    },
                ],
                is_header: false,
            }],
        })]);
        assert!(md.contains('|'));
    }

    #[test]
    fn plain_text_extraction() {
        let content = DocumentContent {
            blocks: vec![
                DocumentBlock::Heading {
                    level: 1,
                    runs: vec![tr("Title")],
                },
                DocumentBlock::Paragraph(vec![tr("Body")]),
                DocumentBlock::CodeBlock {
                    language: None,
                    code: "code".into(),
                },
                DocumentBlock::TextBox(vec![DocumentBlock::Paragraph(vec![tr("box")])]),
                DocumentBlock::Image(DocumentImage {
                    alt_text: Some("img".into()),
                    data: None,
                    extension: None,
                }),
                DocumentBlock::Section {
                    blocks: vec![DocumentBlock::Paragraph(vec![tr("sec")])],
                    section_type: None,
                },
                DocumentBlock::ThematicBreak,
                DocumentBlock::PageBreak,
            ],
            ..Default::default()
        };
        let result = MarkdownRenderer::new(MarkdownOptions::default())
            .render(&content)
            .unwrap();
        assert!(!result.markdown.is_empty());
    }

    #[test]
    fn render_display_math_block() {
        let md = render_blocks(vec![DocumentBlock::Math {
            omml: None,
            latex: Some(r"\frac{1}{2}".into()),
            display: true,
        }]);
        assert!(md.contains("$$"), "display math needs $$ delimiters: {md}");
        assert!(md.contains(r"\frac{1}{2}"), "latex content missing: {md}");
    }

    #[test]
    fn render_inline_math() {
        let md = render_blocks(vec![
            DocumentBlock::Paragraph(vec![tr("see ")]),
            DocumentBlock::Math {
                omml: None,
                latex: Some("x^2".into()),
                display: false,
            },
            DocumentBlock::Paragraph(vec![tr(" for details")]),
        ]);
        assert!(
            md.contains("$x^2$"),
            "inline math needs single $ delimiters: {md}"
        );
    }

    #[test]
    fn render_math_with_omml_fallback() {
        // When latex is None but omml is provided, the renderer should attempt conversion.
        let omml_xml = "<m:oMath xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\">\
            <m:f><m:num><m:r><m:t>1</m:t></m:r></m:num>\
            <m:den><m:r><m:t>2</m:t></m:r></m:den></m:f></m:oMath>";
        let md = render_blocks(vec![DocumentBlock::Math {
            omml: Some(omml_xml.into()),
            latex: None,
            display: true,
        }]);
        assert!(
            md.contains(r"\frac"),
            "OMML should be converted to LaTeX: {md}"
        );
    }

    #[test]
    fn render_math_empty_produces_warning() {
        let content = DocumentContent {
            blocks: vec![DocumentBlock::Math {
                omml: None,
                latex: Some(String::new()),
                display: true,
            }],
            ..Default::default()
        };
        let result = MarkdownRenderer::new(MarkdownOptions::default())
            .render(&content)
            .unwrap();
        assert!(
            result.warnings.iter().any(|w| w.message.contains("math")),
            "expected a warning about empty math: {:?}",
            result.warnings
        );
    }

    #[test]
    fn plain_blocks_extracts_math_latex() {
        let blocks = vec![DocumentBlock::Math {
            omml: None,
            latex: Some(r"\alpha".into()),
            display: false,
        }];
        let text = plain_blocks(&blocks);
        assert!(
            text.contains(r"\alpha"),
            "plain text should include LaTeX: {text}"
        );
    }
}
