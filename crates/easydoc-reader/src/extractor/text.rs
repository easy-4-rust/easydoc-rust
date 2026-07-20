//! Plain text extraction from DOCX/DOC files via office_oxide.

use std::path::Path;

use easydoc_core::{DocError, Result};

/// Extracts all plain text from a document using office_oxide.
///
/// Supports DOCX, DOC, and all other formats office_oxide can read.
///
/// # Errors
///
/// Returns I/O or format errors if the file cannot be opened or parsed.
pub fn extract_text(path: &Path) -> Result<String> {
    let doc = office_oxide::Document::open(path)
        .map_err(|e| DocError::Document(format!("failed to open document: {e}")))?;
    Ok(doc.plain_text())
}
