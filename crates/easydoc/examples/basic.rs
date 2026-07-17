//! Generates a small Chinese business report with styled text and a table.

use easydoc::{
    Alignment, Cell, EasyDoc, Paragraph, ParagraphStyle, Pt, Row, Table, TextRun, TextStyle,
};

fn main() -> Result<(), easydoc::Error> {
    let mut writer = EasyDoc::write("report.docx")
        .default_font("宋体")
        .default_font_size(Pt(12.0))
        .build()?;

    writer.add_heading("年度经营报告", 1);
    writer.add_paragraph(
        Paragraph::new()
            .format(ParagraphStyle::default().align(Alignment::Center))
            .push(TextRun::new("2026 年度").format(TextStyle::default().bold())),
    );
    writer.add_text("以下为本年度经营数据。");
    writer.add_table(
        Table::new()
            .push_row(Row::new([
                Cell::text("项目"),
                Cell::text("数量"),
                Cell::text("金额"),
            ]))
            .push_row(Row::new([
                Cell::text("订单"),
                Cell::text("120"),
                Cell::text("￥32,000"),
            ]))
            .first_row_as_header(),
    );

    writer.finish()
}
