//! Content extractors for DOCX/DOC files.
//!
//! Both text and table extraction work transparently across
//! DOCX and DOC formats via office_oxide's unified API.

pub mod table;
pub mod text;

use std::path::Path;

/// Detects the document format from file extension and magic bytes.
///
/// # Returns
///
/// - `Some(DocumentFormat::Docx)` for `.docx` or ZIP-magic files
/// - `Some(DocumentFormat::Doc)` for `.doc` or OLE2-magic files
/// - `None` if the format cannot be determined
#[must_use]
pub fn detect_format(path: &Path) -> Option<DocumentFormat> {
    // Check extension first
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "docx" => return Some(DocumentFormat::Docx),
            "doc" => return Some(DocumentFormat::Doc),
            _ => {}
        }
    }

    // Fallback: check magic bytes
    if let Ok(bytes) = std::fs::read(path) {
        if bytes.len() >= 8 {
            // DOCX: PK\x03\x04 (ZIP magic)
            if &bytes[0..4] == b"PK\x03\x04" {
                return Some(DocumentFormat::Docx);
            }
            // DOC: OLE2/CFB magic bytes
            if &bytes[0..8] == b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1" {
                return Some(DocumentFormat::Doc);
            }
        }
    }

    None
}

/// Supported document formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    /// Office Open XML (.docx).
    Docx,
    /// Legacy Word Binary (.doc).
    Doc,
}

impl std::fmt::Display for DocumentFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Docx => write!(f, "DOCX"),
            Self::Doc => write!(f, "DOC"),
        }
    }
}
