//! 快速表格写入执行器 -- 将 `Vec<T>` 直接渲染为 DOCX 表格。
//!
//! 从 [`TableColumn`] schema 应用逐列属性（`width`、`wrap`、`format`、`align`），
//! 使生成的 OOXML 忠实表示 `#[derive(DocxRow)]` 产生的列元数据。
//!
//! 对应 Java: `com.alibaba.excel.write.ExcelBuilderImpl` 中的表格写入逻辑

use std::fs::File;
use std::io::{Cursor, Seek, Write};
use std::path::PathBuf;

use docx_rs::Docx;
use easydoc_core::metadata::TableColumn;
use easydoc_core::style::TableStyle;
use easydoc_core::{DocError, DocxRow, Result};

use crate::util::{insert_many_after_nth, parse_width};

/// Executor for one-shot table writes.
pub struct TableWriteExecutor<'a, T: DocxRow> {
    path: PathBuf,
    data: &'a [T],
    title: Option<String>,
    style: TableStyle,
    need_header: bool,
}

impl<'a, T: DocxRow> TableWriteExecutor<'a, T> {
    /// Creates a new table write executor.
    pub(crate) fn new(
        path: PathBuf,
        data: &'a [T],
        title: Option<String>,
        style: TableStyle,
        need_header: bool,
    ) -> Self {
        Self {
            path,
            data,
            title,
            style,
            need_header,
        }
    }

    /// Builds the `docx_rs` document from stored data.
    fn build_docx(&self) -> Result<Docx> {
        let mut docx = Docx::new();

        if let Some(ref title) = self.title {
            docx = docx.add_paragraph(
                docx_rs::Paragraph::new()
                    .add_run(docx_rs::Run::new().add_text(title.as_str()).bold().size(28)),
            );
        }

        // Schema columns sorted by index so they align with `to_row()` cell order.
        let mut schema: Vec<&TableColumn> = T::schema().iter().collect();
        schema.sort_by_key(|c| c.index);
        let schema = schema;

        let mut rows: Vec<docx_rs::TableRow> = Vec::new();

        // ---- Header row ----
        if self.need_header {
            let header_cells: Vec<docx_rs::TableCell> = schema
                .iter()
                .filter(|c| !c.ignored)
                .map(|col| {
                    let mut run = docx_rs::Run::new().add_text(col.name.as_str());
                    if self.style.header_font.bold {
                        run = run.bold();
                    }
                    let mut cell = docx_rs::TableCell::new()
                        .add_paragraph(docx_rs::Paragraph::new().add_run(run));
                    cell = apply_cell_width(cell, col);
                    cell
                })
                .collect();
            rows.push(docx_rs::TableRow::new(header_cells));
        }

        // ---- Data rows ----
        for item in self.data {
            let cells = item.to_row()?;
            let visible_cols: Vec<&&TableColumn> = schema.iter().filter(|c| !c.ignored).collect();

            let data_cells: Vec<docx_rs::TableCell> = cells
                .iter()
                .zip(visible_cols.iter())
                .map(|(cell, col)| {
                    let text = doc_value_str(&cell.value);
                    let mut para =
                        docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text(text));

                    // Apply column-level or cell-level alignment.
                    let alignment = col.align.or(cell.alignment);
                    if let Some(align) = alignment {
                        para = para.align(to_docx_alignment(align));
                    }

                    let mut tc = docx_rs::TableCell::new().add_paragraph(para);
                    tc = apply_cell_width(tc, col);
                    tc
                })
                .collect();
            rows.push(docx_rs::TableRow::new(data_cells));
        }

        docx = docx.add_table(docx_rs::Table::new(rows));
        Ok(docx)
    }

    /// Post-processes the raw document XML to inject `noWrap` and `numFmt`
    /// attributes that `docx-rs` does not natively support.
    fn apply_xml_extras(&self, document_xml: &mut Vec<u8>) -> Result<()> {
        let mut schema: Vec<&TableColumn> = T::schema().iter().collect();
        schema.sort_by_key(|c| c.index);

        let visible: Vec<&TableColumn> = schema.iter().filter(|c| !c.ignored).copied().collect();
        let num_visible = visible.len();

        // Fast path: skip XML post-processing entirely when no columns need
        // noWrap or numFmt injection (avoids O(cells) string allocations).
        let needs_no_wrap = visible.iter().any(|c| !c.wrap);
        let needs_num_fmt = visible.iter().any(|c| c.format.is_some());
        if !needs_no_wrap && !needs_num_fmt {
            return Ok(());
        }

        let xml = String::from_utf8_lossy(document_xml).to_string();
        let mut modified = xml;

        // Apply wrap (noWrap) to all visible cells -- header + data.
        // 线性优化：一次扫描收集所有需要插入的 noWrap 片段，
        // 单次批量插入（避免逐 cell 调用 insert_after_nth 的 O(n²) 扫描）。
        let total_cells = if self.need_header {
            num_visible * (1 + self.data.len())
        } else {
            num_visible * self.data.len()
        };

        let tcw_count = modified.matches("<w:tcW").count();
        let tcpr_count = modified.matches("<w:tcPr").count();
        let rpr_count = modified.matches("<w:pPr><w:rPr").count();

        let mut no_wrap_inserts: Vec<String> = Vec::new();
        for cell_idx in 0..total_cells {
            let col_idx = cell_idx % num_visible;
            let col = visible[col_idx];
            if !col.wrap {
                no_wrap_inserts.push("<w:noWrap/>".to_owned());
            }
        }
        if !no_wrap_inserts.is_empty() {
            // 优先在 <w:tcW ... /> 后插入；数量不足时回退到 <w:tcPr>。
            let pattern = if tcw_count >= no_wrap_inserts.len() {
                "<w:tcW"
            } else {
                "<w:tcPr"
            };
            modified = insert_many_after_nth(&modified, pattern, &no_wrap_inserts);
        }

        // Apply numFmt to data cells only (skip header cells).
        let data_offset = if self.need_header { num_visible } else { 0 };
        let mut num_fmt_inserts: Vec<String> = Vec::new();
        for (i, item) in self.data.iter().enumerate() {
            let cells = item.to_row()?;
            for (j, col) in visible.iter().enumerate() {
                if let Some(ref fmt) = col.format {
                    let cell_idx = data_offset + i * num_visible + j;
                    // 与 insert_num_fmt 的语义一致：仅在对应位置存在
                    // <w:pPr><w:rPr> 时才插入（先收集，统一判断）
                    if cell_idx < rpr_count {
                        num_fmt_inserts.push(format!("<w:numFmt w:val=\"{fmt}\"/>"));
                    }
                }
            }
            let _ = cells; // keep ownership for potential future use
        }
        if !num_fmt_inserts.is_empty() {
            modified = insert_many_after_nth(&modified, "<w:pPr><w:rPr", &num_fmt_inserts);
        }

        *document_xml = modified.into_bytes();
        Ok(())
    }

    /// Executes the write to disk.
    pub fn execute(self) -> Result<()> {
        let file = File::create(&self.path)?;
        let docx = self.build_docx()?;
        let mut xml_docx = docx.build();
        self.apply_xml_extras(&mut xml_docx.document)?;
        xml_docx
            .pack(file)
            .map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(())
    }

    /// Executes the write to a generic writer.
    ///
    /// Corresponds to Hutool's `flush(OutputStream)` pattern.
    pub fn execute_to_writer<W: Write + Seek>(self, writer: W) -> Result<()> {
        let docx = self.build_docx()?;
        let mut xml_docx = docx.build();
        self.apply_xml_extras(&mut xml_docx.document)?;
        xml_docx
            .pack(writer)
            .map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(())
    }

    /// Executes the write and returns bytes.
    pub fn execute_to_bytes(self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let cursor = Cursor::new(&mut buf);
        let docx = self.build_docx()?;
        let mut xml_docx = docx.build();
        self.apply_xml_extras(&mut xml_docx.document)?;
        xml_docx
            .pack(cursor)
            .map_err(|e| DocError::Zip(e.to_string()))?;
        Ok(buf)
    }
}

/// Applies the column's `width` attribute to a `docx_rs::TableCell`.
///
/// Uses [`parse_width`] to convert the CSS-like width string to OOXML twips
/// or percentage units.  No-op when the column has no width set.
fn apply_cell_width(cell: docx_rs::TableCell, col: &TableColumn) -> docx_rs::TableCell {
    if let Some(ref w) = col.width
        && let Some(parsed) = parse_width(w)
    {
        return cell.width(parsed.value, parsed.width_type);
    }
    cell
}

/// Converts our domain [`HorizontalAlignment`] to the `docx-rs` equivalent.
fn to_docx_alignment(
    alignment: easydoc_core::types::HorizontalAlignment,
) -> docx_rs::AlignmentType {
    // `_` 通配与显式分支体相同是 #[non_exhaustive] 的必然结果
    #[allow(clippy::match_same_arms)]
    match alignment {
        easydoc_core::types::HorizontalAlignment::Left => docx_rs::AlignmentType::Left,
        easydoc_core::types::HorizontalAlignment::Center => docx_rs::AlignmentType::Center,
        easydoc_core::types::HorizontalAlignment::Right => docx_rs::AlignmentType::Right,
        easydoc_core::types::HorizontalAlignment::Both => docx_rs::AlignmentType::Both,
        // #[non_exhaustive]：未来新增的对齐方式默认左对齐
        _ => docx_rs::AlignmentType::Left,
    }
}

fn doc_value_str(value: &easydoc_core::DocValue) -> String {
    match value {
        easydoc_core::DocValue::String(s) => s.clone(),
        easydoc_core::DocValue::Int(n) => n.to_string(),
        easydoc_core::DocValue::Float(n) => n.to_string(),
        easydoc_core::DocValue::Bool(b) => b.to_string(),
        easydoc_core::DocValue::Empty => String::new(),
        other => format!("{other:?}"),
    }
}
