//! 文档编辑器 -- 打开已有 DOCX 进行修改。
//!
//! 对应 Hutool 的 `Word07Writer(File)` 模式：如果文件存在，打开编辑而非创建新文档。

use std::path::{Path, PathBuf};

use easydoc_core::{DocError, Result};
use office_oxide::edit::EditableDocument;

/// 已打开的 DOCX 文件，准备进行修改。
///
/// 通过门面 `EasyDoc::edit()` 方法创建。包装 `office_oxide` 的 `EditableDocument`
/// 以支持文本替换和保存。
///
/// # 示例
///
/// ```ignore
/// EasyDoc::edit("existing.docx")?
///     .replace_text("{name}", "Alice")
///     .replace_text("{date}", "2026-07-21")
///     .save()?;
/// ```
pub struct DocEditor {
    path: PathBuf,
    doc: EditableDocument,
    /// `replace_text` 的延迟错误（错误消息文本）。`office_oxide` 0.1.10 起，
    /// XLSX 上的文本替换返回命名错误而非静默空成功；为保持 builder 链式
    /// API 与结构体 auto trait（UnwindSafe 系）不变，这里只存 `String`，
    /// `save()` / `save_as()` 时再重建 `DocError` 如实报出。
    replace_error: Option<String>,
}

impl DocEditor {
    /// Opens an existing DOCX file for editing.
    ///
    /// # Errors
    ///
    /// Returns I/O or format errors.
    pub fn open(path: &Path) -> Result<Self> {
        let doc = EditableDocument::open(path)
            .map_err(|e| DocError::Document(format!("cannot open document: {e}")))?;
        Ok(Self {
            path: path.to_path_buf(),
            doc,
            replace_error: None,
        })
    }

    /// Replaces all occurrences of `find` with `replace` in the document text.
    ///
    /// Corresponds to Hutool's placeholder replacement pattern
    /// (which Hutool itself does not provide — users must use raw POI).
    ///
    /// XLSX 不支持文本替换（`office_oxide` 0.1.10 起）。替换失败不中断链式调用，
    /// 错误在 `save()` / `save_as()` 时报出，避免静默写回未修改的文档。
    #[must_use]
    pub fn replace_text(mut self, find: &str, replace: &str) -> Self {
        if let Err(e) = self.doc.replace_text(find, replace) {
            self.replace_error = Some(e.to_string());
        }
        self
    }

    /// Saves the modified document, overwriting the original file.
    ///
    /// # Errors
    ///
    /// Returns deferred `replace_text` errors or I/O errors.
    pub fn save(self) -> Result<()> {
        if let Some(msg) = self.replace_error {
            return Err(DocError::Document(msg));
        }
        self.doc
            .save(&self.path)
            .map_err(|e| DocError::Document(format!("cannot save document: {e}")))
    }

    /// Saves the modified document to a new path.
    ///
    /// # Errors
    ///
    /// Returns deferred `replace_text` errors or I/O errors.
    pub fn save_as(self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(msg) = self.replace_error {
            return Err(DocError::Document(msg));
        }
        self.doc
            .save(path.as_ref())
            .map_err(|e| DocError::Document(format!("cannot save document: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use office_oxide::xlsx::write::{CellData, XlsxWriter};

    /// XLSX 不支持文本替换：错误不中断链式调用，`save()` 时如实报出
    /// （对应 `office_oxide` 0.1.10 的"静默空成功改命名错误"语义）。
    #[test]
    fn replace_text_on_xlsx_defers_error_to_save() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("book.xlsx");

        let mut writer = XlsxWriter::new();
        {
            let mut sheet = writer.add_sheet("Sheet1");
            sheet.set_cell(0, 0, CellData::String("value".into()));
        }
        let mut file = std::fs::File::create(&path).expect("create xlsx");
        writer.write_to(&mut file).expect("write xlsx");

        let editor = DocEditor::open(&path).expect("open xlsx");
        let editor = editor.replace_text("value", "replaced");
        let err = editor.save().expect_err("deferred replace error");
        assert!(
            err.to_string().contains("not supported"),
            "unexpected error: {err}"
        );
    }
}
