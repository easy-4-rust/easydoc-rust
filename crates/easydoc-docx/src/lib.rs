//! DOCX rendering backend for easydoc-rs.

use std::fs::File;
use std::io::{Seek, Write};
use std::path::Path;

use docx_rs::{
    AlignmentType, BreakType, Docx, PageMargin, Paragraph as DocxParagraph, Pic, Run, RunFonts,
    SpecialIndentType, Table as DocxTable, TableCell, TableRow,
};
use easydoc_core::{
    Alignment, Block, Cell, Document, Error, FontFamily, Image, Inline, Paragraph, ParagraphStyle,
    Result, Table, TextStyle,
};

/// Renders backend-neutral documents as Office Open XML DOCX packages.
#[derive(Clone, Copy, Debug, Default)]
pub struct DocxRenderer;

impl DocxRenderer {
    /// Renders a document to a file path.
    ///
    /// # Errors
    ///
    /// Returns an I/O, style resolution, or DOCX packaging error.
    pub fn render_to_path(document: &Document, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let file = File::create(path).map_err(|source| Error::io(path, source))?;
        Self::render(document, file)
    }

    /// Renders a document to a seekable output stream.
    ///
    /// # Errors
    ///
    /// Returns a style resolution or DOCX packaging error.
    pub fn render<W>(document: &Document, writer: W) -> Result<()>
    where
        W: Write + Seek,
    {
        let config = &document.config;
        let margins = config.margins;
        let mut output = Docx::new()
            .page_size(
                non_negative(config.page_size.width.twips()),
                non_negative(config.page_size.height.twips()),
            )
            .page_margin(
                PageMargin::new()
                    .top(margins.top.twips())
                    .right(margins.right.twips())
                    .bottom(margins.bottom.twips())
                    .left(margins.left.twips()),
            );

        for block in &document.blocks {
            output = render_top_level_block(output, block, document)?;
        }

        output
            .build()
            .pack(writer)
            .map_err(|error| Error::Backend(error.to_string()))
    }
}

fn non_negative(value: i32) -> u32 {
    u32::try_from(value.max(0)).unwrap_or_default()
}

fn render_top_level_block(output: Docx, block: &Block, document: &Document) -> Result<Docx> {
    match block {
        Block::Paragraph(paragraph) => Ok(output.add_paragraph(render_paragraph(
            paragraph,
            document,
            &TextStyle::default(),
        )?)),
        Block::Table(table) => Ok(output.add_table(render_table(table, document)?)),
        Block::Image(image) => Ok(output.add_paragraph(image_paragraph(image))),
        Block::PageBreak => Ok(output
            .add_paragraph(DocxParagraph::new().add_run(Run::new().add_break(BreakType::Page)))),
    }
}

fn render_paragraph(
    paragraph: &Paragraph,
    document: &Document,
    inherited_text: &TextStyle,
) -> Result<DocxParagraph> {
    let named = match paragraph.style_name.as_deref() {
        Some(name) => document.style(name)?.paragraph.clone(),
        None => ParagraphStyle::default(),
    };
    let style = named.overlay(&paragraph.style);
    let paragraph_text = inherited_text.overlay(&style.text);
    let mut output = DocxParagraph::new();

    if let Some(alignment) = style.alignment {
        output = output.align(render_alignment(alignment));
    }
    if style.keep_next == Some(true) {
        output = output.keep_next(true);
    }
    if style.left_indent.is_some() || style.first_line_indent.is_some() {
        output = output.indent(
            style.left_indent.map(easydoc_core::Length::twips),
            style
                .first_line_indent
                .map(|length| SpecialIndentType::FirstLine(length.twips())),
            None,
            None,
        );
    }

    for child in &paragraph.children {
        output = match child {
            Inline::Text(text) => output.add_run(render_text_run(
                &text.text,
                &default_text_style(document)
                    .overlay(&paragraph_text)
                    .overlay(&text.style),
            )),
            Inline::Image(image) => output.add_run(image_run(image)),
            Inline::LineBreak => output.add_run(Run::new().add_break(BreakType::TextWrapping)),
        };
    }
    Ok(output)
}

fn default_text_style(document: &Document) -> TextStyle {
    TextStyle {
        font: Some(document.config.default_font.clone()),
        size: Some(document.config.default_font_size),
        ..TextStyle::default()
    }
}

fn render_text_run(text: &str, style: &TextStyle) -> Run {
    let mut run = Run::new().add_text(text);
    if let Some(font) = &style.font {
        run = run.fonts(render_font(font));
    }
    if let Some(size) = style.size {
        run = run.size(size.half_points());
    }
    if let Some(color) = &style.color {
        run = run.color(color);
    }
    if let Some(bold) = style.bold {
        run = if bold { run.bold() } else { run.disable_bold() };
    }
    if let Some(italic) = style.italic {
        run = if italic {
            run.italic()
        } else {
            run.disable_italic()
        };
    }
    if let Some(underline) = style.underline {
        run = run.underline(if underline { "single" } else { "none" });
    }
    run
}

fn render_font(font: &FontFamily) -> RunFonts {
    let mut output = RunFonts::new();
    if let Some(value) = &font.ascii {
        output = output.ascii(value);
    }
    if let Some(value) = &font.east_asia {
        output = output.east_asia(value);
    }
    if let Some(value) = &font.high_ansi {
        output = output.hi_ansi(value);
    }
    if let Some(value) = &font.complex_script {
        output = output.cs(value);
    }
    output
}

fn render_alignment(alignment: Alignment) -> AlignmentType {
    match alignment {
        Alignment::Left => AlignmentType::Left,
        Alignment::Center => AlignmentType::Center,
        Alignment::Right => AlignmentType::Right,
        Alignment::Justified => AlignmentType::Justified,
    }
}

fn image_run(image: &Image) -> Run {
    Run::new().add_image(Pic::new(&image.data).size(image.width.emu(), image.height.emu()))
}

fn image_paragraph(image: &Image) -> DocxParagraph {
    DocxParagraph::new()
        .align(AlignmentType::Center)
        .add_run(image_run(image))
}

fn render_table(table: &Table, document: &Document) -> Result<DocxTable> {
    let mut rows = Vec::with_capacity(table.rows.len());
    for (index, row) in table.rows.iter().enumerate() {
        let header_style = if table.first_row_as_header && index == 0 {
            TextStyle::default().bold()
        } else {
            TextStyle::default()
        };
        let cells = row
            .cells
            .iter()
            .map(|cell| render_cell(cell, document, &header_style))
            .collect::<Result<Vec<_>>>()?;
        rows.push(TableRow::new(cells).cant_split());
    }
    Ok(DocxTable::new(rows))
}

fn render_cell(cell: &Cell, document: &Document, inherited_text: &TextStyle) -> Result<TableCell> {
    let mut output = TableCell::new();
    if cell.colspan > 1 {
        output = output.grid_span(cell.colspan);
    }
    if cell.blocks.is_empty() {
        return Ok(output.add_paragraph(DocxParagraph::new()));
    }
    for block in &cell.blocks {
        output = match block {
            Block::Paragraph(paragraph) => {
                output.add_paragraph(render_paragraph(paragraph, document, inherited_text)?)
            }
            Block::Table(table) => output.add_table(render_table(table, document)?),
            Block::Image(image) => output.add_paragraph(image_paragraph(image)),
            Block::PageBreak => output
                .add_paragraph(DocxParagraph::new().add_run(Run::new().add_break(BreakType::Page))),
        };
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use easydoc_core::{Cell, Paragraph, Row, Table, TextRun};
    use zip::ZipArchive;

    use super::*;

    #[test]
    fn renders_text_and_table_into_a_valid_docx_package() {
        let mut document = Document::new();
        document.config.default_font = FontFamily::all("宋体");
        document.push(
            Paragraph::new().push(TextRun::new("年度报告").format(TextStyle::default().bold())),
        );
        document.push(
            Table::new()
                .push_row(Row::new([Cell::text("项目"), Cell::text("数量")]))
                .push_row(Row::new([Cell::text("订单"), Cell::text("120")]))
                .first_row_as_header(),
        );

        let mut buffer = Cursor::new(Vec::new());
        DocxRenderer::render(&document, &mut buffer).unwrap();
        buffer.set_position(0);
        let mut archive = ZipArchive::new(buffer).unwrap();
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();

        assert!(xml.contains("年度报告"));
        assert!(xml.contains("订单"));
        assert!(xml.contains("w:eastAsia=\"宋体\""));
        assert!(xml.contains("<w:tbl>"));
    }
}
