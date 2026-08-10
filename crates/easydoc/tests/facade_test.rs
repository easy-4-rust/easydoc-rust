//! Integration tests for the new `EasyDoc` facade methods:
//! - `read_events` (SAX streaming read)
//! - `view_as` (`ViewMode` rendering)

use easydoc::prelude::*;
use easydoc::{ContentCollector, EasyDoc, ViewMode};
use tempfile::TempDir;

// ============================================================================
// Helper: build a small DOCX with heading + paragraph + table
// ============================================================================

/// Helper struct for table writing in tests.
#[derive(Debug, Clone)]
struct Item {
    name: String,
    qty: String,
}

impl DocxRow for Item {
    fn schema() -> &'static [easydoc::TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<easydoc::TableColumn>> =
            std::sync::LazyLock::new(|| {
                vec![
                    easydoc::TableColumn::new("Name", "name", 0),
                    easydoc::TableColumn::new("Qty", "qty", 1),
                ]
            });
        &SCHEMA
    }
    fn from_row(_row: &easydoc::RowData) -> easydoc::Result<Self> {
        unimplemented!("not needed for write test")
    }
    fn from_row_with_converters(
        _row: &easydoc::RowData,
        _registry: &easydoc::ConverterRegistry,
    ) -> easydoc::Result<Self> {
        unimplemented!("not needed for write test")
    }
    fn to_row(&self) -> easydoc::Result<Vec<easydoc::CellData>> {
        Ok(vec![
            easydoc::CellData::new(self.name.clone()),
            easydoc::CellData::new(self.qty.clone()),
        ])
    }
    fn to_row_with_converters(
        &self,
        _registry: &easydoc::ConverterRegistry,
    ) -> easydoc::Result<Vec<easydoc::CellData>> {
        self.to_row()
    }
}

fn build_test_docx(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("test_facade.docx");

    let items = vec![
        Item {
            name: "Widget".into(),
            qty: "10".into(),
        },
        Item {
            name: "Gadget".into(),
            qty: "5".into(),
        },
    ];

    EasyDoc::document(&path)
        .add_heading("Test Document", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("This is a test paragraph."))
        .add_heading("Details", HeadingLevel::H2)
        .add_paragraph(Paragraph::new().add_text("Another paragraph with details."))
        .add_table(easydoc::Table::from_data(&items))
        .save()
        .expect("save should succeed");

    path
}

// ============================================================================
// Test: read_events via SAX streaming
// ============================================================================

#[test]
fn test_read_events_heading_paragraph_table() {
    let dir = TempDir::new().expect("tempdir");
    let path = build_test_docx(dir.path());

    let mut collector = ContentCollector::new();
    EasyDoc::read_events(&path, &mut collector).expect("read_events should succeed");
    let content = collector.into_content();

    // Should have: H1 heading, paragraph, H2 heading, paragraph, table
    assert!(
        content.blocks.len() >= 4,
        "expected at least 4 blocks, got {}",
        content.blocks.len()
    );

    // First block: H1 heading
    match &content.blocks[0] {
        DocumentBlock::Heading { level, runs } => {
            assert_eq!(*level, 1, "first heading should be level 1");
            let text: String = runs.iter().map(|r| r.text.as_str()).collect();
            assert!(
                text.contains("Test Document"),
                "heading text should contain 'Test Document', got: {text}"
            );
        }
        other => panic!("expected Heading as first block, got: {other:?}"),
    }

    // Second block: paragraph
    match &content.blocks[1] {
        DocumentBlock::Paragraph(runs) => {
            let text: String = runs.iter().map(|r| r.text.as_str()).collect();
            assert!(
                text.contains("test paragraph"),
                "paragraph text should contain 'test paragraph', got: {text}"
            );
        }
        other => panic!("expected Paragraph as second block, got: {other:?}"),
    }

    // Third block: H2 heading
    match &content.blocks[2] {
        DocumentBlock::Heading { level, runs } => {
            assert_eq!(*level, 2, "second heading should be level 2");
            let text: String = runs.iter().map(|r| r.text.as_str()).collect();
            assert!(
                text.contains("Details"),
                "heading text should contain 'Details', got: {text}"
            );
        }
        other => panic!("expected Heading as third block, got: {other:?}"),
    }

    // Find the table block
    let table_block = content
        .blocks
        .iter()
        .find(|b| matches!(b, DocumentBlock::Table(_)));
    assert!(table_block.is_some(), "should contain a Table block");

    if let Some(DocumentBlock::Table(table)) = table_block {
        assert!(
            table.rows.len() >= 2,
            "table should have at least 2 rows, got {}",
            table.rows.len()
        );
    }
}

#[test]
fn test_read_events_nonexistent_file() {
    let result = EasyDoc::read_events(
        "/nonexistent/path/to/file.docx",
        &mut ContentCollector::new(),
    );
    assert!(result.is_err(), "should fail for nonexistent file");
}

// ============================================================================
// Test: view_as with ViewMode::Plain
// ============================================================================

#[test]
fn test_view_as_plain() {
    let dir = TempDir::new().expect("tempdir");
    let path = build_test_docx(dir.path());

    let text = EasyDoc::view_as(&path, &ViewMode::Plain).expect("view_as Plain should succeed");

    assert!(
        text.contains("Test Document"),
        "plain view should contain heading text: {text}"
    );
    assert!(
        text.contains("test paragraph"),
        "plain view should contain paragraph text: {text}"
    );
    // Plain mode should NOT contain annotation markers
    assert!(
        !text.contains("[段落"),
        "plain view should not contain annotation markers: {text}"
    );
}

// ============================================================================
// Test: view_as with ViewMode::Annotated
// ============================================================================

#[test]
fn test_view_as_annotated() {
    let dir = TempDir::new().expect("tempdir");
    let path = build_test_docx(dir.path());

    let text =
        EasyDoc::view_as(&path, &ViewMode::Annotated).expect("view_as Annotated should succeed");

    assert!(
        text.contains("[标题1]") || text.contains("[标题 1]"),
        "annotated view should contain heading annotation: {text}"
    );
    assert!(
        text.contains("[段落") || text.contains("[表格"),
        "annotated view should contain structural annotations: {text}"
    );
}

// ============================================================================
// Test: view_as with ViewMode::Outline
// ============================================================================

#[test]
fn test_view_as_outline() {
    let dir = TempDir::new().expect("tempdir");
    let path = build_test_docx(dir.path());

    let text = EasyDoc::view_as(&path, &ViewMode::Outline { max_level: 3 })
        .expect("view_as Outline should succeed");

    // Outline should contain headings with Markdown-style # markers
    assert!(
        text.contains("# Test Document") || text.contains('#'),
        "outline view should contain heading markers: {text}"
    );
    // Outline should NOT contain paragraph text
    assert!(
        !text.contains("test paragraph"),
        "outline view should not contain paragraph body text: {text}"
    );
}

// ============================================================================
// Test: view_as with ViewMode::Stats
// ============================================================================

#[test]
fn test_view_as_stats() {
    let dir = TempDir::new().expect("tempdir");
    let path = build_test_docx(dir.path());

    let text = EasyDoc::view_as(&path, &ViewMode::Stats).expect("view_as Stats should succeed");

    // Stats mode should contain count information
    assert!(
        text.contains("段落数") || text.contains("段落"),
        "stats view should contain paragraph count: {text}"
    );
    assert!(
        text.contains("标题") || text.contains("heading"),
        "stats view should contain heading count: {text}"
    );
}

// ============================================================================
// Test: roundtrip — write → read_events → view_as
// ============================================================================

#[test]
fn test_end_to_end_write_read_events_view() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("e2e.docx");

    // 1. Write a document
    EasyDoc::document(&path)
        .add_heading("Chapter 1", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("Introduction text."))
        .add_heading("Section 1.1", HeadingLevel::H2)
        .add_paragraph(
            Paragraph::new()
                .add_text("Body with ")
                .add_run(easydoc::Run::new("bold").bold()),
        )
        .save()
        .expect("write should succeed");

    // 2. Read via SAX streaming
    let mut collector = ContentCollector::new();
    EasyDoc::read_events(&path, &mut collector).expect("read_events should succeed");
    let content = collector.into_content();

    assert!(
        content.blocks.len() >= 3,
        "should have at least 3 blocks, got {}",
        content.blocks.len()
    );

    // 3. Render as plain text
    let plain = EasyDoc::view_as(&path, &ViewMode::Plain).expect("view_as Plain should succeed");
    assert!(
        plain.contains("Chapter 1"),
        "plain should contain heading: {plain}"
    );
    assert!(
        plain.contains("Introduction text"),
        "plain should contain paragraph: {plain}"
    );

    // 4. Render as outline
    let outline = EasyDoc::view_as(&path, &ViewMode::Outline { max_level: 2 })
        .expect("view_as Outline should succeed");
    assert!(
        outline.contains("Chapter 1"),
        "outline should contain H1: {outline}"
    );
    assert!(
        outline.contains("Section 1.1"),
        "outline should contain H2: {outline}"
    );
}

// ============================================================================
// Test: re-exported types are accessible
// ============================================================================

#[test]
fn test_re_exported_types_accessible() {
    // Verify that DocxSaxReader, ViewMode, render_view are accessible
    // from the easydoc crate without depending on easydoc_reader directly.
    let _ = std::any::TypeId::of::<easydoc::DocxSaxReader<std::io::Cursor<Vec<u8>>>>();
    let _ = std::any::TypeId::of::<easydoc::ViewMode>();
    // render_view is a function, not a type — just verify it's callable
    let content = DocumentContent::default();
    let result = easydoc::render_view(&content, &ViewMode::Plain);
    assert!(
        result.is_ok(),
        "render_view should succeed on empty content"
    );
}

// ============================================================================
// Test: ContentCollector from prelude
// ============================================================================

#[test]
fn test_content_collector_from_prelude() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("collector_test.docx");

    EasyDoc::document(&path)
        .add_paragraph(Paragraph::new().add_text("Collector test."))
        .save()
        .expect("save should succeed");

    let mut collector = ContentCollector::new();
    EasyDoc::read_events(&path, &mut collector).expect("read_events should succeed");
    let content = collector.into_content();

    assert!(
        !content.blocks.is_empty(),
        "collector should have collected blocks"
    );
}

// ============================================================================
// Test: read_events with empty document
// ============================================================================

#[test]
fn test_read_events_empty_doc() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("empty.docx");

    // Create a minimal document (no content blocks)
    EasyDoc::document(&path)
        .save()
        .expect("save should succeed");

    let mut collector = ContentCollector::new();
    EasyDoc::read_events(&path, &mut collector).expect("read_events should succeed");
    let content = collector.into_content();

    // Empty document should produce no content blocks
    // (DocumentStart/DocumentEnd are consumed by ContentCollector without producing blocks)
    assert!(
        content.blocks.is_empty(),
        "empty document should produce no blocks, got {}",
        content.blocks.len()
    );
}

// ============================================================================
// Test: view_as with all four modes on the same document
// ============================================================================

#[test]
fn test_view_as_all_modes_consistent() {
    let dir = TempDir::new().expect("tempdir");
    let path = build_test_docx(dir.path());

    let modes = [
        ViewMode::Plain,
        ViewMode::Annotated,
        ViewMode::Outline { max_level: 3 },
        ViewMode::Stats,
    ];

    for mode in &modes {
        let result = EasyDoc::view_as(&path, mode);
        assert!(
            result.is_ok(),
            "view_as should succeed for mode {:?}: {:?}",
            mode,
            result.err()
        );
        let text = result.unwrap();
        assert!(
            !text.is_empty(),
            "view_as should return non-empty text for mode {mode:?}"
        );
    }
}
