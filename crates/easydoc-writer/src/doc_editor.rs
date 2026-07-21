//! Document editor — open existing DOCX for modification.
//!
//! Corresponds to Hutool's `Word07Writer(File)` pattern:
//! if the file exists, open it for editing rather than creating a new document.

use std::path::{Path, PathBuf};

use easydoc_core::{DocError, Result};
use office_oxide::edit::EditableDocument;

/// An open DOCX file ready for modification.
///
/// Created via [`EasyDoc::edit()`].
/// Wraps `office_oxide`'s `EditableDocument` for text replacement and saving.
///
/// # Example
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
        })
    }

    /// Replaces all occurrences of `find` with `replace` in the document text.
    ///
    /// Corresponds to Hutool's placeholder replacement pattern
    /// (which Hutool itself does not provide — users must use raw POI).
    ///
    /// Returns the number of replacements made.
    #[must_use]
    pub fn replace_text(mut self, find: &str, replace: &str) -> Self {
        self.doc.replace_text(find, replace);
        self
    }

    /// Saves the modified document, overwriting the original file.
    ///
    /// # Errors
    ///
    /// Returns I/O errors.
    pub fn save(self) -> Result<()> {
        self.doc
            .save(&self.path)
            .map_err(|e| DocError::Document(format!("cannot save document: {e}")))
    }

    /// Saves the modified document to a new path.
    ///
    /// # Errors
    ///
    /// Returns I/O errors.
    pub fn save_as(self, path: impl AsRef<Path>) -> Result<()> {
        self.doc
            .save(path.as_ref())
            .map_err(|e| DocError::Document(format!("cannot save document: {e}")))
    }
}
