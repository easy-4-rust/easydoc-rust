//! Plain text extraction from DOCX/DOC files via `office_oxide`.

use std::path::Path;

use easydoc_core::{DocError, Result};

use crate::extractor::{DocumentFormat, detect_format_from_bytes};

/// Extracts all plain text from a document using `office_oxide`.
///
/// Supports DOCX, DOC, and all other formats `office_oxide` can read.
///
/// # Errors
///
/// Returns I/O or format errors if the file cannot be opened or parsed.
pub fn extract_text(path: &Path) -> Result<String> {
    let doc = office_oxide::Document::open(path)
        .map_err(|e| DocError::Document(format!("failed to open document: {e}")))?;
    Ok(doc.plain_text())
}

/// Extracts all plain text from in-memory document bytes.
///
/// Detects DOCX/DOC from magic bytes and parses without touching the
/// filesystem — suitable for fuzzing and embedded/streaming callers.
///
/// # Errors
///
/// Returns format errors if the bytes are not a supported document, or
/// parse errors from `office_oxide`.
pub fn extract_text_from_bytes(bytes: &[u8]) -> Result<String> {
    let format = detect_format_from_bytes(bytes).ok_or_else(|| {
        DocError::Format("unsupported document: could not detect DOCX/DOC magic bytes".to_owned())
    })?;
    let doc = office_oxide::Document::from_reader(std::io::Cursor::new(bytes.to_vec()), to_oxide(format))
        .map_err(|e| DocError::Document(format!("failed to open document from bytes: {e}")))?;
    Ok(doc.plain_text())
}

/// Maps easydoc's format enum to `office_oxide`'s.
fn to_oxide(format: DocumentFormat) -> office_oxide::DocumentFormat {
    match format {
        DocumentFormat::Docx => office_oxide::DocumentFormat::Docx,
        DocumentFormat::Doc => office_oxide::DocumentFormat::Doc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use easydoc_writer::DocBuilder;

    fn minimal_docx_bytes() -> Vec<u8> {
        DocBuilder::new("memory.docx")
            .add_paragraph(easydoc_writer::Paragraph::new().add_text("hello bytes"))
            .save_to_bytes()
            .expect("writer should produce valid DOCX")
    }

    #[test]
    fn detect_format_from_bytes_docx() {
        let bytes = minimal_docx_bytes();
        assert_eq!(detect_format_from_bytes(&bytes), Some(DocumentFormat::Docx));
    }

    #[test]
    fn detect_format_from_bytes_doc_magic() {
        let ole2: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
        assert_eq!(detect_format_from_bytes(&ole2), Some(DocumentFormat::Doc));
    }

    #[test]
    fn detect_format_from_bytes_garbage() {
        assert_eq!(detect_format_from_bytes(b"not a document"), None);
        assert_eq!(detect_format_from_bytes(b""), None);
        assert_eq!(detect_format_from_bytes(&[0xd0, 0xcf]), None); // too short
    }

    #[test]
    fn extract_text_from_bytes_roundtrip() {
        let bytes = minimal_docx_bytes();
        let text = extract_text_from_bytes(&bytes).expect("should parse");
        assert!(text.contains("hello bytes"));
    }

    #[test]
    fn extract_text_from_bytes_garbage_errors() {
        assert!(extract_text_from_bytes(b"garbage").is_err());
    }
}
