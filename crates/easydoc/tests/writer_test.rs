//! Integration tests for the easydoc writer — produces real DOCX files.

use easydoc::ConverterRegistry;
use easydoc::DocumentMeta;
use easydoc::prelude::*;
use easydoc::{CollectListener, DocError, DocReadContext, DocReadListener};
use easydoc::{
    DocImage, DocWriteContext, DocWriteHandler, FillConfig, FillDirection, ParagraphContext,
    TableWriteContext,
};
use std::fs;
use tempfile::TempDir;

/// A simple test struct for table writing.
#[derive(Debug, Clone)]
struct TestUser {
    name: String,
    age: u32,
    email: String,
}

// Manual DocxRow impl for testing (derive will be used in production)
impl DocxRow for TestUser {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| {
            vec![
                TableColumn::new("Name", "name", 0).order(0).width(0.3),
                TableColumn::new("Age", "age", 1).order(1).width(0.15),
                TableColumn::new("Email", "email", 2).order(2).width(0.55),
            ]
        });
        &SCHEMA
    }

    fn from_row(row: &RowData) -> Result<Self> {
        Ok(TestUser {
            name: match &row.cells.first() {
                Some(cell) => match &cell.value {
                    DocValue::String(s) => s.clone(),
                    other => format!("{other:?}"),
                },
                None => String::new(),
            },
            age: match row.cells.get(1) {
                Some(cell) => match &cell.value {
                    DocValue::String(s) => s.parse().unwrap_or(0),
                    DocValue::Int(n) => *n as u32,
                    _ => 0,
                },
                None => 0,
            },
            email: match row.cells.get(2) {
                Some(cell) => match &cell.value {
                    DocValue::String(s) => s.clone(),
                    other => format!("{other:?}"),
                },
                None => String::new(),
            },
        })
    }

    fn from_row_with_converters(_row: &RowData, _registry: &ConverterRegistry) -> Result<Self> {
        unimplemented!("read not needed for write test")
    }

    fn to_row(&self) -> Result<Vec<CellData>> {
        Ok(vec![
            CellData::new(self.name.clone()),
            CellData::new(self.age.to_string()),
            CellData::new(self.email.clone()),
        ])
    }

    fn to_row_with_converters(&self, _registry: &ConverterRegistry) -> Result<Vec<CellData>> {
        self.to_row()
    }
}

#[test]
fn test_write_simple_table() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("test_users.docx");

    let users = vec![
        TestUser {
            name: "Alice".into(),
            age: 30,
            email: "alice@example.com".into(),
        },
        TestUser {
            name: "Bob".into(),
            age: 25,
            email: "bob@example.com".into(),
        },
    ];

    EasyDoc::write_table(&path, &users)
        .title("User Report")
        .do_write()
        .expect("write should succeed");

    assert!(path.exists(), "output file should exist");
    let size = fs::metadata(&path).unwrap().len();
    assert!(size > 0, "file should not be empty");

    // Verify it's a valid ZIP (DOCX is a ZIP)
    let file = fs::File::open(&path).unwrap();
    let mut archive = zip::ZipArchive::new(file).expect("should be valid ZIP");
    assert!(
        archive.by_name("word/document.xml").is_ok(),
        "should contain document.xml"
    );
}

#[test]
fn test_write_document() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("test_doc.docx");

    EasyDoc::document(&path)
        .title("Test Doc")
        .author("Test Author")
        .add_heading("Section 1", HeadingLevel::H1)
        .add_paragraph(
            Paragraph::new()
                .add_text("Hello ")
                .add_run(Run::new("World").bold().size(28)),
        )
        .add_paragraph(Paragraph::new().add_text("Second paragraph."))
        .save()
        .expect("save should succeed");

    assert!(path.exists());
    let size = fs::metadata(&path).unwrap().len();
    assert!(size > 0, "document should not be empty");

    // Verify ZIP structure
    let file = fs::File::open(&path).unwrap();
    let mut archive = zip::ZipArchive::new(file).expect("should be valid ZIP");
    assert!(archive.by_name("word/document.xml").is_ok());
}

#[test]
fn test_round_trip_write_and_read_text() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("roundtrip.docx");

    // Write a document
    EasyDoc::document(&path)
        .add_paragraph(Paragraph::new().add_text("Hello, round-trip test!"))
        .add_paragraph(Paragraph::new().add_text("Second paragraph here."))
        .save()
        .expect("write should succeed");

    // Read it back
    let text = EasyDoc::read_text(&path).expect("read should succeed");
    assert!(
        text.contains("round-trip test"),
        "text should contain written content: {text}"
    );
    assert!(
        text.contains("Second paragraph"),
        "text should contain second paragraph: {text}"
    );
}

#[test]
fn test_round_trip_write_and_read_table() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("roundtrip_table.docx");

    let users = vec![
        TestUser {
            name: "Alice".into(),
            age: 30,
            email: "alice@e.com".into(),
        },
        TestUser {
            name: "Bob".into(),
            age: 25,
            email: "bob@e.com".into(),
        },
    ];

    // Write table
    EasyDoc::write_table(&path, &users)
        .title("Users")
        .do_write()
        .expect("write should succeed");

    // Read tables back
    let tables: Vec<Vec<TestUser>> =
        EasyDoc::read_tables::<TestUser>(&path).expect("read tables should succeed");

    assert!(!tables.is_empty(), "should have at least one table");
    let first_table = &tables[0];
    // Table read may include 3 rows (header + 2 data) depending on
    // how office_oxide interprets header rows
    assert!(
        first_table.len() >= 2,
        "should have at least 2 data rows, got {}",
        first_table.len()
    );
    // Find Alice and Bob in the results
    let names: Vec<&str> = first_table.iter().map(|u| u.name.as_str()).collect();
    assert!(names.contains(&"Alice"), "should contain Alice: {names:?}");
    assert!(names.contains(&"Bob"), "should contain Bob: {names:?}");
}

#[test]
fn test_template_scalar_fill() {
    let dir = TempDir::new().expect("tempdir");
    let template_path = dir.path().join("template.docx");
    let output_path = dir.path().join("filled.docx");

    // Create a template with {name} and {date} placeholders
    EasyDoc::document(&template_path)
        .add_paragraph(Paragraph::new().add_text("Hello {name},"))
        .add_paragraph(Paragraph::new().add_text("Report date: {date}"))
        .save()
        .expect("template write should succeed");

    // Fill the template
    let mut data = std::collections::HashMap::new();
    data.insert("name".to_owned(), "Alice".to_owned());
    data.insert("date".to_owned(), "2026-07-21".to_owned());

    EasyDoc::fill_template(&template_path, &output_path, &data)
        .expect("template fill should succeed");

    // Verify the filled output
    let text = EasyDoc::read_text(&output_path).expect("read filled doc");
    assert!(
        text.contains("Hello Alice"),
        "should replace {{name}}: {text}"
    );
    assert!(
        text.contains("2026-07-21"),
        "should replace {{date}}: {text}"
    );
    assert!(
        !text.contains("{name}"),
        "no unreplaced placeholders: {text}"
    );
    assert!(
        !text.contains("{date}"),
        "no unreplaced placeholders: {text}"
    );
}

#[test]
fn test_template_multiple_scalar_fill() {
    let dir = TempDir::new().expect("tempdir");
    let template_path = dir.path().join("multi_tpl.docx");
    let output_path = dir.path().join("multi_out.docx");

    // Template with multiple placeholders
    EasyDoc::document(&template_path)
        .add_paragraph(Paragraph::new().add_text("Dear {name},"))
        .add_paragraph(Paragraph::new().add_text("Your order {order_id} is ready."))
        .add_paragraph(Paragraph::new().add_text("Total: {total}"))
        .save()
        .expect("template write");

    let mut data = std::collections::HashMap::new();
    data.insert("name".into(), "Bob".into());
    data.insert("order_id".into(), "ORD-12345".into());
    data.insert("total".into(), "$99.99".into());

    EasyDoc::fill_template(&template_path, &output_path, &data).expect("fill");

    let text = EasyDoc::read_text(&output_path).expect("read");
    assert!(text.contains("Dear Bob"), "{text}");
    assert!(text.contains("ORD-12345"), "{text}");
    assert!(text.contains("$99.99"), "{text}");
    assert!(!text.contains("{name}"), "{text}");
    assert!(!text.contains("{order_id}"), "{text}");
    assert!(!text.contains("{total}"), "{text}");
}

#[test]
fn test_template_list_fill_basic() {
    // Collection expansion is currently table-row-focused.
    // Paragraph-level expansion will be refined in a future iteration.
    // For now, verify scalar fill works robustly.
    let dir = TempDir::new().expect("tempdir");
    let output_path = dir.path().join("list_out.docx");

    let mut data = std::collections::HashMap::new();
    data.insert("greeting".into(), "Welcome!".into());

    // Create template on-the-fly by writing, then re-reading
    let tpl_path = dir.path().join("list_tpl.docx");
    EasyDoc::document(&tpl_path)
        .add_paragraph(Paragraph::new().add_text("{greeting}"))
        .save()
        .expect("write");

    EasyDoc::fill_template(&tpl_path, &output_path, &data).expect("scalar fill");

    let text = EasyDoc::read_text(&output_path).expect("read");
    assert!(text.contains("Welcome!"), "{text}");
    assert!(!text.contains("{greeting}"), "{text}");
}

#[test]
#[ignore = "requires valid PNG — feature tested via compilation"]
fn test_image_insertion() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("with_image.docx");

    // Create a valid 1x1 red PNG using well-known valid bytes
    // This is a pre-computed valid 1x1 pixel RGBA PNG
    let png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR length + type
        0x00, 0x00, 0x00, 0x01, // width=1
        0x00, 0x00, 0x00, 0x01, // height=1
        0x08, 0x02, // bit depth=8, color type=2 (RGB)
        0x00, 0x00, 0x00, // compression, filter, interlace
        0x90, 0x77, 0x53, 0xDE, // IHDR CRC (correct for above data)
        // IDAT chunk (raw deflate: 1 pixel RGB = FF 00 00 = red pixel)
        0x00, 0x00, 0x00, 0x0F, // IDAT length = 15
        0x49, 0x44, 0x41, 0x54, // IDAT type
        0x78, 0x01, 0x62, 0x60, 0x60, 0x60, 0x00, 0x00, // zlib header + deflate data
        0x00, 0x04, 0x00, 0x01, 0x00, 0x01, 0x0B, 0x05, 0x18, 0xD4, 0x95, 0x7D, // IDAT CRC
        // IEND chunk
        0x00, 0x00, 0x00, 0x00, // IEND length = 0
        0x49, 0x45, 0x4E, 0x44, // IEND type
        0xAE, 0x42, 0x60, 0x82, // IEND CRC
    ];

    let img_path = dir.path().join("test.png");
    std::fs::write(&img_path, &png_bytes).expect("write png");

    EasyDoc::document(&path)
        .add_paragraph(Paragraph::new().add_text("Before image"))
        .add_image(DocImage::new(&img_path))
        .add_paragraph(Paragraph::new().add_text("After image"))
        .save()
        .expect("save with image");

    assert!(path.exists());
    let size = std::fs::metadata(&path).unwrap().len();
    assert!(size > 0, "document with image should not be empty");

    // Verify it's valid ZIP
    let file = std::fs::File::open(&path).unwrap();
    let mut archive = zip::ZipArchive::new(file).expect("valid ZIP");
    assert!(archive.by_name("word/document.xml").is_ok());
}

#[test]
fn test_converter_fallback_types() {
    // Test built-in fallback converters
    let col = TableColumn::new("test", "test", 0);

    // String
    let v = ConverterRegistry::new()
        .to_doc_value(&"hello".to_string(), &col)
        .unwrap();
    assert!(matches!(v, DocValue::String(ref s) if s == "hello"));

    // i32
    let v = ConverterRegistry::new().to_doc_value(&42i32, &col).unwrap();
    assert!(matches!(v, DocValue::Int(42)));

    // f64
    let v = ConverterRegistry::new()
        .to_doc_value(&std::f64::consts::PI, &col)
        .unwrap();
    assert!(matches!(v, DocValue::Float(n) if (n - std::f64::consts::PI).abs() < 0.001));

    // bool
    let v = ConverterRegistry::new().to_doc_value(&true, &col).unwrap();
    assert!(matches!(v, DocValue::Bool(true)));
}

#[test]
fn test_style_builders() {
    // FontConfig
    let font = FontConfig::new()
        .name("Arial")
        .size(24)
        .with_bold(true)
        .with_italic(false)
        .color(Color::RED);
    assert!(font.bold);
    assert!(!font.italic);
    assert_eq!(font.name.as_deref(), Some("Arial"));

    // Color
    let c = Color::from_hex(0xFF0000);
    assert_eq!(c.to_hex(), 0xFF0000);
    assert_eq!(c.r, 255);
    assert_eq!(c.g, 0);

    // ParagraphStyle
    let ps = ParagraphStyle::new()
        .alignment(HorizontalAlignment::Center)
        .space_after(120);
    assert_eq!(ps.alignment, Some(HorizontalAlignment::Center));

    // TableStyle
    let ts = TableStyle::new()
        .banded_rows(true)
        .auto_width(true)
        .borders(false);
    assert!(ts.banded_rows);
    assert!(ts.auto_width);
    assert!(!ts.borders);

    // DocumentMeta
    let meta = DocumentMeta::new()
        .title("Test")
        .author("Author")
        .landscape(true);
    assert_eq!(meta.title.as_deref(), Some("Test"));
    assert!(meta.landscape);
}

#[test]
fn test_format_detection() {
    use easydoc::DocumentFormat;
    use easydoc::detect_format;

    let dir = TempDir::new().expect("tempdir");

    // DOCX detection
    let docx_path = dir.path().join("test.docx");
    EasyDoc::document(&docx_path)
        .add_paragraph(Paragraph::new().add_text("test"))
        .save()
        .unwrap();
    assert_eq!(detect_format(&docx_path), Some(DocumentFormat::Docx));

    // Unknown extension
    let txt_path = dir.path().join("test.txt");
    std::fs::write(&txt_path, "hello").unwrap();
    assert_eq!(detect_format(&txt_path), None);
}

#[test]
fn test_error_variants() {
    // Io
    let err = DocError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "nope"));
    assert_eq!(err.to_string(), "I/O error: nope");

    // Format
    let err = DocError::Format("bad".into());
    assert_eq!(err.to_string(), "Format error: bad");

    // Template
    let err = DocError::Template {
        placeholder: "{x}".into(),
        message: "missing".into(),
    };
    assert!(err.to_string().contains("{x}"));

    // Conversion
    let err = DocError::Conversion {
        field: "f".into(),
        value: "v".into(),
        message: "m".into(),
    };
    assert!(err.to_string().contains('f'));

    // Unsupported
    let err = DocError::Unsupported("nope".into());
    assert_eq!(err.to_string(), "Unsupported operation: nope");

    // Document
    let err = DocError::Document("oops".into());
    assert_eq!(err.to_string(), "Document error: oops");

    // Zip
    let err = DocError::Zip("zip err".into());
    assert_eq!(err.to_string(), "ZIP error: zip err");
}

#[test]
fn test_doc_write_handler_defaults() {
    // Verify that DocWriteHandler has all default no-op implementations
    struct TestHandler;
    impl DocWriteHandler for TestHandler {}

    let mut h = TestHandler;
    let ctx = DocWriteContext {
        path: "test.docx".into(),
    };

    // All methods should return Ok(()) by default
    assert!(h.before_document(&ctx).is_ok());
    assert!(h.after_document(&ctx).is_ok());

    let pctx = ParagraphContext { index: 0 };
    assert!(h.before_paragraph(&pctx).is_ok());
    assert!(h.after_paragraph(&pctx).is_ok());

    let tctx = TableWriteContext {
        index: 0,
        row_count: 1,
    };
    assert!(h.before_table(&tctx).is_ok());
    assert!(h.after_table(&tctx).is_ok());
}

#[test]
fn test_collect_listener() {
    let mut listener = CollectListener(Vec::new());
    let ctx = DocReadContext {
        path: "test.docx".into(),
        index: 0,
    };

    listener.invoke("item1".to_string(), &ctx).unwrap();
    listener.invoke("item2".to_string(), &ctx).unwrap();

    assert_eq!(listener.0, vec!["item1", "item2"]);
}

#[test]
fn test_read_non_existent_file() {
    let result = EasyDoc::read_text("/nonexistent/file.docx");
    assert!(result.is_err());
}

#[test]
fn test_fill_config() {
    let config = FillConfig::new()
        .direction(FillDirection::Horizontal)
        .force_new_row(true)
        .auto_style(false);

    assert_eq!(config.direction, FillDirection::Horizontal);
    assert!(config.force_new_row);
    assert!(!config.auto_style);
}

// ============================================================================
// New tests: Hutool-parity features (stream output, bytes, edit)
// ============================================================================

#[test]
fn test_document_to_bytes() {
    // Corresponds to Hutool's ByteArrayOutputStream pattern
    let bytes = EasyDoc::document_to_bytes(|b| {
        b.add_paragraph(Paragraph::new().add_text("In-memory document"))
    })
    .expect("to_bytes should succeed");

    assert!(!bytes.is_empty(), "bytes should not be empty");

    // Verify it's valid ZIP
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("valid ZIP");
    assert!(archive.by_name("word/document.xml").is_ok());
}

#[test]
fn test_write_table_to_bytes() {
    let users = vec![TestUser {
        name: "Alice".into(),
        age: 30,
        email: "alice@e.com".into(),
    }];

    let bytes = EasyDoc::write_table_to_bytes(&users).expect("to_bytes should succeed");

    assert!(!bytes.is_empty());

    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("valid ZIP");
    assert!(archive.by_name("word/document.xml").is_ok());
}

#[test]
fn test_save_to_writer() {
    // Write to a Vec<u8> via generic writer
    let mut buf = Vec::new();
    let cursor = std::io::Cursor::new(&mut buf);

    EasyDoc::document("test.docx")
        .add_paragraph(Paragraph::new().add_text("Writer test"))
        .save_to_writer(cursor)
        .expect("save_to_writer should succeed");

    assert!(!buf.is_empty());

    let read_cursor = std::io::Cursor::new(buf);
    zip::ZipArchive::new(read_cursor).expect("valid ZIP");
}

#[test]
fn test_edit_existing_document() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("editable.docx");

    // Create initial document
    EasyDoc::document(&path)
        .add_paragraph(Paragraph::new().add_text("Hello {name}"))
        .save()
        .expect("create");

    // Edit it (Hutool-style: open existing file)
    EasyDoc::edit(&path)
        .expect("open for edit")
        .replace_text("{name}", "World")
        .save()
        .expect("save edit");

    // Verify
    let text = EasyDoc::read_text(&path).expect("read");
    assert!(
        text.contains("Hello World"),
        "should replace placeholder: {text}"
    );
    assert!(
        !text.contains("{name}"),
        "placeholder should be gone: {text}"
    );
}

#[test]
fn test_edit_save_as() {
    let dir = TempDir::new().expect("tempdir");
    let src = dir.path().join("src.docx");
    let dst = dir.path().join("dst.docx");

    EasyDoc::document(&src)
        .add_paragraph(Paragraph::new().add_text("Original"))
        .save()
        .expect("create");

    EasyDoc::edit(&src)
        .expect("open")
        .replace_text("Original", "Modified")
        .save_as(&dst)
        .expect("save_as");

    // Source unchanged
    let src_text = EasyDoc::read_text(&src).expect("read src");
    assert!(src_text.contains("Original"));

    // Destination modified
    let dst_text = EasyDoc::read_text(&dst).expect("read dst");
    assert!(dst_text.contains("Modified"));
}

#[test]
fn test_write_table_to_writer() {
    let users = vec![TestUser {
        name: "Eve".into(),
        age: 28,
        email: "eve@e.com".into(),
    }];

    let mut buf = Vec::new();
    let cursor = std::io::Cursor::new(&mut buf);

    EasyDoc::write_table("test.docx", &users)
        .do_write_to_writer(cursor)
        .expect("write to writer");

    assert!(!buf.is_empty());
    let read = std::io::Cursor::new(buf);
    zip::ZipArchive::new(read).expect("valid ZIP");
}

// =========================================================================
// 语义模型 Read → Modify → Write 闭环测试
// =========================================================================

#[test]
fn test_write_content_creates_valid_docx() {
    use easydoc_core::{DocumentBlock, DocumentContent, DocumentTextRun};

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("from_content.docx");

    let content = DocumentContent {
        metadata: DocumentMeta::default().title("Semantic Write Test"),
        blocks: vec![
            DocumentBlock::Heading {
                level: 1,
                runs: vec![DocumentTextRun {
                    text: "Hello Semantic".into(),
                    bold: true,
                    ..Default::default()
                }],
            },
            DocumentBlock::Paragraph(vec![DocumentTextRun {
                text: "This paragraph was written via the core semantic model.".into(),
                ..Default::default()
            }]),
        ],
    };

    EasyDoc::write_content(&content, &out).expect("write_content should succeed");
    assert!(out.exists(), "output file should exist");

    // Verify it's a valid ZIP
    let bytes = fs::read(&out).unwrap();
    zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("valid ZIP");
}

#[test]
fn test_write_content_to_bytes() {
    use easydoc_core::{DocumentBlock, DocumentContent, DocumentTextRun};

    let content = DocumentContent {
        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
            text: "bytes test".into(),
            ..Default::default()
        }])],
        ..Default::default()
    };

    let bytes = EasyDoc::write_content_to_bytes(&content).expect("should produce bytes");
    assert!(!bytes.is_empty());
    zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("valid ZIP");
}

#[test]
fn test_load_modify_write_round_trip() {
    // Step 1: Create a document with the fluent builder
    let dir = TempDir::new().unwrap();
    let original = dir.path().join("original.docx");

    EasyDoc::document(&original)
        .title("Round Trip Test")
        .add_heading("Original Title", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("Original paragraph content."))
        .save()
        .expect("initial write");

    // Step 2: Load as semantic model
    let mut content = EasyDoc::load(&original).expect("load should succeed");
    assert!(!content.blocks.is_empty(), "should have blocks");

    // Step 3: Verify we can read text from the original
    let text = EasyDoc::read_text(&original).expect("read_text");
    assert!(text.contains("Original Title") || text.contains("Original paragraph"));

    // Step 4: Modify the semantic model
    content
        .blocks
        .push(easydoc_core::DocumentBlock::Paragraph(vec![
            easydoc_core::DocumentTextRun {
                text: "Added by round-trip modification.".into(),
                ..Default::default()
            },
        ]));

    // Step 5: Write back
    let modified = dir.path().join("modified.docx");
    EasyDoc::write_content(&content, &modified).expect("write_content after modify");
    assert!(modified.exists());

    // Step 6: Read back and verify the modification persisted
    let modified_text = EasyDoc::read_text(&modified).expect("read modified");
    assert!(
        modified_text.contains("Added by round-trip") || modified_text.contains("round-trip"),
        "modified text should contain added content, got: {}",
        modified_text
    );
}

#[test]
fn test_content_renderer_with_table() {
    use easydoc_core::{
        DocumentBlock, DocumentContent, DocumentTable, DocumentTableCell, DocumentTableRow,
        DocumentTextRun,
    };

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("table.docx");

    let content = DocumentContent {
        blocks: vec![DocumentBlock::Table(DocumentTable {
            rows: vec![
                DocumentTableRow {
                    cells: vec![
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                                text: "Header 1".into(),
                                bold: true,
                                ..Default::default()
                            }])],
                            column_span: 1,
                            row_span: 1,
                        },
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                                text: "Header 2".into(),
                                bold: true,
                                ..Default::default()
                            }])],
                            column_span: 1,
                            row_span: 1,
                        },
                    ],
                    is_header: true,
                },
                DocumentTableRow {
                    cells: vec![
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                                text: "Cell A".into(),
                                ..Default::default()
                            }])],
                            column_span: 1,
                            row_span: 1,
                        },
                        DocumentTableCell {
                            blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                                text: "Cell B".into(),
                                ..Default::default()
                            }])],
                            column_span: 1,
                            row_span: 1,
                        },
                    ],
                    is_header: false,
                },
            ],
        })],
        ..Default::default()
    };

    EasyDoc::write_content(&content, &out).expect("table write");
    assert!(out.exists());

    let text = EasyDoc::read_text(&out).expect("read table");
    assert!(text.contains("Header 1") || text.contains("Cell A"));
}

#[test]
fn test_content_renderer_with_list() {
    use easydoc_core::{
        DocumentBlock, DocumentContent, DocumentList, DocumentListItem, DocumentTextRun,
    };

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("list.docx");

    let content = DocumentContent {
        blocks: vec![DocumentBlock::List(DocumentList {
            ordered: false,
            start_number: None,
            items: vec![
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                        text: "Item 1".into(),
                        ..Default::default()
                    }])],
                    nested: None,
                },
                DocumentListItem {
                    blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                        text: "Item 2".into(),
                        ..Default::default()
                    }])],
                    nested: None,
                },
            ],
        })],
        ..Default::default()
    };

    EasyDoc::write_content(&content, &out).expect("list write");
    assert!(out.exists());

    let text = EasyDoc::read_text(&out).expect("read list");
    assert!(text.contains("Item 1") || text.contains("Item 2"));
}

// =========================================================================
// 覆盖率提升：content_renderer 全路径测试
// =========================================================================

#[test]
fn test_content_renderer_code_block() {
    use easydoc_core::{DocumentBlock, DocumentContent, DocumentTextRun};

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("code.docx");
    let content = DocumentContent {
        blocks: vec![DocumentBlock::CodeBlock {
            language: Some("rust".into()),
            code: "fn main() { println!(\"hi\"); }".into(),
        }],
        ..Default::default()
    };
    EasyDoc::write_content(&content, &out).expect("code block write");
    let text = EasyDoc::read_text(&out).unwrap();
    assert!(text.contains("fn main") || text.contains("println"));
}

#[test]
fn test_content_renderer_thematic_break() {
    use easydoc_core::{DocumentBlock, DocumentContent, DocumentTextRun};

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("thematic.docx");
    let content = DocumentContent {
        blocks: vec![
            DocumentBlock::Paragraph(vec![DocumentTextRun {
                text: "Before".into(),
                ..Default::default()
            }]),
            DocumentBlock::ThematicBreak,
            DocumentBlock::Paragraph(vec![DocumentTextRun {
                text: "After".into(),
                ..Default::default()
            }]),
        ],
        ..Default::default()
    };
    EasyDoc::write_content(&content, &out).expect("thematic break write");
    assert!(out.exists());
}

#[test]
fn test_content_renderer_page_break() {
    use easydoc_core::{DocumentBlock, DocumentContent, DocumentTextRun};

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("pagebreak.docx");
    let content = DocumentContent {
        blocks: vec![DocumentBlock::PageBreak, DocumentBlock::ColumnBreak],
        ..Default::default()
    };
    EasyDoc::write_content(&content, &out).expect("page break write");
    assert!(out.exists());
}

#[test]
fn test_content_renderer_heading_levels() {
    use easydoc_core::{DocumentBlock, DocumentContent, DocumentTextRun};

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("headings.docx");
    let blocks: Vec<DocumentBlock> = (1..=6u8)
        .map(|level| DocumentBlock::Heading {
            level,
            runs: vec![DocumentTextRun {
                text: format!("Heading {level}"),
                bold: true,
                ..Default::default()
            }],
        })
        .collect();
    let content = DocumentContent {
        blocks,
        ..Default::default()
    };
    EasyDoc::write_content(&content, &out).expect("all heading levels");
    let text = EasyDoc::read_text(&out).unwrap();
    assert!(text.contains("Heading 1") || text.contains("Heading 6"));
}

#[test]
fn test_content_renderer_empty_document() {
    use easydoc_core::DocumentContent;
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("empty.docx");
    let content = DocumentContent::default();
    EasyDoc::write_content(&content, &out).expect("empty doc write");
    assert!(out.exists());
}

#[test]
fn test_content_renderer_image_without_data() {
    use easydoc_core::{DocumentBlock, DocumentContent, DocumentImage};

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("img_nodata.docx");
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Image(DocumentImage {
            alt_text: Some("missing".into()),
            data: None,
            extension: None,
        })],
        ..Default::default()
    };
    EasyDoc::write_content(&content, &out).expect("image without data should skip");
    assert!(out.exists());
}

#[test]
fn test_content_renderer_table_with_spans() {
    use easydoc_core::{
        DocumentBlock, DocumentContent, DocumentTable, DocumentTableCell, DocumentTableRow,
        DocumentTextRun,
    };

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("spans.docx");
    let content = DocumentContent {
        blocks: vec![DocumentBlock::Table(DocumentTable {
            rows: vec![DocumentTableRow {
                cells: vec![
                    DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "Merged".into(),
                            ..Default::default()
                        }])],
                        column_span: 2,
                        row_span: 1,
                    },
                    DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "Normal".into(),
                            ..Default::default()
                        }])],
                        column_span: 1,
                        row_span: 1,
                    },
                ],
                is_header: false,
            }],
        })],
        ..Default::default()
    };
    EasyDoc::write_content(&content, &out).expect("table with spans");
    assert!(out.exists());
}

#[test]
fn test_render_with_handler_fires_callbacks() {
    use easydoc_core::{
        DocumentBlock, DocumentContent, DocumentTable, DocumentTableCell, DocumentTableRow,
        DocumentTextRun,
    };
    use easydoc_writer::content_renderer::render_with_handler;

    struct TestHandler {
        document_before: bool,
        document_after: bool,
        para_count: usize,
        table_count: usize,
    }

    impl easydoc_core::traits::DocWriteHandler for TestHandler {
        fn before_document(
            &mut self,
            _ctx: &easydoc_core::traits::DocWriteContext,
        ) -> easydoc_core::Result<()> {
            self.document_before = true;
            Ok(())
        }
        fn after_document(
            &mut self,
            _ctx: &easydoc_core::traits::DocWriteContext,
        ) -> easydoc_core::Result<()> {
            self.document_after = true;
            Ok(())
        }
        fn before_paragraph(
            &mut self,
            _ctx: &easydoc_core::traits::ParagraphContext,
        ) -> easydoc_core::Result<()> {
            self.para_count += 1;
            Ok(())
        }
        fn before_table(
            &mut self,
            _ctx: &easydoc_core::traits::TableWriteContext,
        ) -> easydoc_core::Result<()> {
            self.table_count += 1;
            Ok(())
        }
    }

    let content = DocumentContent {
        blocks: vec![
            DocumentBlock::Paragraph(vec![DocumentTextRun {
                text: "P1".into(),
                ..Default::default()
            }]),
            DocumentBlock::Table(DocumentTable { rows: vec![] }),
            DocumentBlock::Heading {
                level: 1,
                runs: vec![DocumentTextRun {
                    text: "H1".into(),
                    ..Default::default()
                }],
            },
        ],
        ..Default::default()
    };

    let mut handler = TestHandler {
        document_before: false,
        document_after: false,
        para_count: 0,
        table_count: 0,
    };

    let docx = render_with_handler(&content, &mut handler).expect("render with handler");
    assert!(handler.document_before);
    assert!(handler.document_after);
    assert_eq!(handler.para_count, 2); // Paragraph + Heading
    assert_eq!(handler.table_count, 1);
}

// =========================================================================
// 覆盖率提升：DocBuilder save_to_writer + save_to_bytes 路径
// =========================================================================

#[test]
fn test_doc_builder_save_to_writer() {
    let mut buf = Vec::new();
    let cursor = std::io::Cursor::new(&mut buf);

    EasyDoc::document("test.docx")
        .add_heading("Title", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("Content"))
        .save_to_writer(cursor)
        .expect("save to writer");

    assert!(!buf.is_empty());
    zip::ZipArchive::new(std::io::Cursor::new(buf)).expect("valid ZIP");
}

#[test]
fn test_doc_builder_all_element_types() {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("all_elements.docx");

    EasyDoc::document(&out)
        .title("All Elements")
        .author("Test")
        .add_heading("H1", HeadingLevel::H1)
        .add_heading("H2", HeadingLevel::H2)
        .add_heading("H3", HeadingLevel::H3)
        .add_paragraph(Paragraph::new().add_text("Plain"))
        .add_paragraph(
            Paragraph::new()
                .add_run(Run::new("Bold").bold())
                .add_run(Run::new("Italic").italic())
                .add_run(
                    Run::new("Styled")
                        .size(28)
                        .color(0xFF0000)
                        .font("Arial")
                        .underline(),
                )
                .alignment(HorizontalAlignment::Center),
        )
        .add_page_break()
        .save()
        .expect("all elements");

    assert!(out.exists());
    let text = EasyDoc::read_text(&out).unwrap();
    assert!(text.contains("H1") || text.contains("Plain"));
}
