//! Property-based fuzzing tests for `easydoc-reader`.
//!
//! Uses `proptest` to verify that the reader and security guards do not panic
//! on corrupted, truncated, or adversarial inputs. Each test runs at least 256
//! cases (proptest default) by default.

use proptest::prelude::*;
use std::io::{Cursor, Write};

// ---------------------------------------------------------------------------
// Strategies: generators for adversarial byte sequences
// ---------------------------------------------------------------------------

/// Strategy that generates arbitrary byte sequences (0--4 KB).
///
/// Covers completely random bytes that are unlikely to be valid XML or ZIP.
fn corrupted_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..4096)
}

/// Strategy that generates bytes starting with the ZIP magic number
/// (`PK\x03\x04`) followed by random garbage.
///
/// Simulates a file that "looks like" a ZIP but is corrupted beyond the
/// magic bytes.
fn fake_zip_with_garbage() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..2048).prop_map(|mut extra| {
        let mut bytes = vec![0x50, 0x4B, 0x03, 0x04]; // ZIP local file header magic
        bytes.append(&mut extra);
        bytes
    })
}

/// Strategy that generates truncated ZIP archives.
///
/// Starts with a valid minimal DOCX ZIP (containing `word/document.xml`) and
/// truncates it at a random point, simulating a partially downloaded or
/// corrupted file.
fn truncated_docx_zip() -> impl Strategy<Value = Vec<u8>> {
    let valid_zip = make_minimal_docx_zip();
    let len = valid_zip.len();
    // Truncate at a random position within the valid ZIP.
    (0..len).prop_map(move |cut| valid_zip[..cut].to_vec())
}

/// Builds a minimal valid DOCX ZIP archive in memory.
///
/// The archive contains a single entry `word/document.xml` with a basic
/// OOXML document skeleton.
fn make_minimal_docx_zip() -> Vec<u8> {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>test</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(xml).unwrap();
        zip.finish().unwrap();
    }
    buf
}

/// Writes bytes to a temporary file and returns the file handle (kept alive
/// so the file is not deleted before the caller is done).
fn write_temp_file(data: &[u8]) -> tempfile::NamedTempFile {
    let mut tmp = tempfile::NamedTempFile::new().expect("failed to create temp file");
    tmp.write_all(data).expect("failed to write temp file");
    tmp
}

// ---------------------------------------------------------------------------
// Test 1: Garbage bytes to from_reader (raw XML reader)
// ---------------------------------------------------------------------------

proptest! {
    /// Arbitrary random bytes fed to [`DocxSaxReader::from_reader`] must not
    /// cause a panic. The reader operates on raw XML (not ZIP), so garbage
    /// input should produce an `Err` (XML parse error) or an empty result,
    /// but never unwind.
    #[test]
    fn fuzz_from_reader_does_not_panic_on_garbage(bytes in corrupted_bytes()) {
        let cursor = Cursor::new(bytes);
        let mut reader = easydoc_reader::DocxSaxReader::from_reader(cursor);
        // read_blocks triggers the full XML parsing pipeline.
        // We only care that it does not panic; errors are expected.
        let _ = reader.read_blocks();
    }

    // -----------------------------------------------------------------------
    // Test 2: Garbage bytes to from_path (ZIP reader)
    // -----------------------------------------------------------------------

    /// Arbitrary random bytes written to a temp file and opened via
    /// [`DocxSaxReader::from_path`] must not cause a panic. The ZIP layer
    /// should reject invalid archives with an error.
    #[test]
    fn fuzz_from_path_does_not_panic_on_garbage(bytes in corrupted_bytes()) {
        let tmp = write_temp_file(&bytes);
        let _ = easydoc_reader::DocxSaxReader::from_path(tmp.path());
    }

    // -----------------------------------------------------------------------
    // Test 3: Fake ZIP (valid magic + garbage) to from_path
    // -----------------------------------------------------------------------

    /// Bytes starting with the ZIP magic number (`PK\x03\x04`) followed by
    /// random data must not cause a panic in the ZIP parser or the DOCX
    /// reader.
    #[test]
    fn fuzz_from_path_does_not_panic_on_fake_zip(bytes in fake_zip_with_garbage()) {
        let tmp = write_temp_file(&bytes);
        let _ = easydoc_reader::DocxSaxReader::from_path(tmp.path());
    }

    // -----------------------------------------------------------------------
    // Test 4: Truncated ZIP to from_path
    // -----------------------------------------------------------------------

    /// A valid DOCX ZIP archive truncated at a random offset must not cause
    /// a panic. The ZIP or XML layer should detect the truncation and return
    /// an error.
    #[test]
    fn fuzz_from_path_does_not_panic_on_truncated_zip(bytes in truncated_docx_zip()) {
        let tmp = write_temp_file(&bytes);
        let _ = easydoc_reader::DocxSaxReader::from_path(tmp.path());
    }

    // -----------------------------------------------------------------------
    // Test 5: Arbitrary paths to read_document
    // -----------------------------------------------------------------------

    /// Arbitrary path strings passed to [`easydoc_reader::read_document`] must
    /// not cause a panic. Non-existent or inaccessible paths should return
    /// `Err`.
    #[test]
    fn fuzz_read_document_does_not_panic_on_arbitrary_path(path in "\\PC{1,200}") {
        let _ = easydoc_reader::read_document(std::path::Path::new(&path));
    }

    // -----------------------------------------------------------------------
    // Test 6: Arbitrary URL strings to SsrfGuard::check_url
    // -----------------------------------------------------------------------

    /// Arbitrary strings passed to [`SsrfGuard::check_url`] must not cause a
    /// panic. Malformed inputs should return `Err`.
    #[test]
    fn fuzz_ssrf_guard_does_not_panic_on_arbitrary_url(url in "\\PC{0,512}") {
        let guard = easydoc_reader::security::SsrfGuard::new();
        let _ = guard.check_url(&url);
    }

    // -----------------------------------------------------------------------
    // Test 7: Permissive SSRF guard on arbitrary URLs
    // -----------------------------------------------------------------------

    /// Same as above but with the permissive guard, which skips DNS
    /// resolution and private-IP checks.
    #[test]
    fn fuzz_ssrf_guard_permissive_does_not_panic(url in "\\PC{0,512}") {
        let guard = easydoc_reader::security::SsrfGuard::permissive();
        let _ = guard.check_url(&url);
    }

    // -----------------------------------------------------------------------
    // Test 8: Garbage bytes through the full view_as pipeline
    // -----------------------------------------------------------------------

    /// Garbage bytes written to a temp file and passed through the
    /// `read_document` + `render_view` pipeline must not panic.
    #[test]
    fn fuzz_view_as_does_not_panic_on_garbage(
        bytes in corrupted_bytes(),
        mode in prop_oneof![
            Just(easydoc_reader::ViewMode::Plain),
            Just(easydoc_reader::ViewMode::Annotated),
            Just(easydoc_reader::ViewMode::Stats),
            (0u8..7).prop_map(|l| easydoc_reader::ViewMode::Outline { max_level: l }),
        ],
    ) {
        let tmp = write_temp_file(&bytes);
        if let Ok(content) = easydoc_reader::read_document(tmp.path()) {
            let _ = easydoc_reader::render_view(&content, &mode);
        }
    }
}
