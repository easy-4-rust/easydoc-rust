use std::fmt::Write as FmtWrite;
use std::fs;

use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentList, DocumentTable, DocumentTextRun, Result,
};
use easydoc_ooxml::AtomicFile;

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
