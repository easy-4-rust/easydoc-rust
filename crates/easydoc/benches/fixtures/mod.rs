//! Fidelity fixtures for byte-for-byte roundtrip verification.
//!
//! Each fixture is a programmatically generated DOCX document stored as
//! in-memory bytes.  The expected output (plain text from `view_as(Plain)`)
//! is captured at initialization time by writing the bytes to a temp file
//! and reading them back.
//!
//! These fixtures verify that performance optimizations do not introduce
//! data drift -- every byte of the expected output must match.

use std::sync::LazyLock;

use easydoc::EasyDoc;
use easydoc::prelude::*;
use easydoc_core::metadata::TableColumn;
use easydoc_core::{
    CellData, ConverterRegistry, DocumentBlock, DocumentContent, DocumentList, DocumentListItem,
    DocumentTextRun, DocxRow, Result, RowData,
};

// ---------------------------------------------------------------------------
// Fidelity fixture descriptor
// ---------------------------------------------------------------------------

/// A single fidelity test case: known DOCX bytes plus expected plain-text output.
pub(crate) struct FidelityFixture {
    /// Human-readable name (used as Criterion parameter).
    pub name: &'static str,
    /// The DOCX file as bytes.
    pub docx_bytes: Vec<u8>,
    /// Size of the DOCX in bytes.
    pub original_size: u64,
    /// Expected plain-text output from `view_as(Plain)`.
    pub expected_text: String,
}

impl FidelityFixture {
    /// Writes the DOCX bytes to a [`tempfile::NamedTempFile`] with a `.docx`
    /// suffix and returns it.
    ///
    /// The file is automatically deleted when the returned handle is dropped.
    pub fn write_to_temp(&self) -> tempfile::NamedTempFile {
        let file = tempfile::Builder::new()
            .suffix(".docx")
            .tempfile()
            .expect("create temp file for fixture");
        std::fs::write(file.path(), &self.docx_bytes).expect("write fixture docx to temp");
        file
    }
}

// ---------------------------------------------------------------------------
// Table row type for fixture 2 (3 columns)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct FixtureRow {
    name: String,
    value: String,
    score: f64,
}

impl DocxRow for FixtureRow {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: LazyLock<Vec<TableColumn>> = LazyLock::new(|| {
            vec![
                TableColumn::new("Name", "name", 0),
                TableColumn::new("Value", "value", 1),
                TableColumn::new("Score", "score", 2),
            ]
        });
        &SCHEMA
    }

    fn from_row(_row: &RowData) -> Result<Self> {
        unimplemented!("not used in fidelity fixtures")
    }

    fn from_row_with_converters(_row: &RowData, _registry: &ConverterRegistry) -> Result<Self> {
        unimplemented!("not used in fidelity fixtures")
    }

    fn to_row(&self) -> Result<Vec<CellData>> {
        Ok(vec![
            CellData::new(self.name.clone()),
            CellData::new(self.value.clone()),
            CellData::new(self.score),
        ])
    }

    fn to_row_with_converters(&self, _registry: &ConverterRegistry) -> Result<Vec<CellData>> {
        self.to_row()
    }
}

// ---------------------------------------------------------------------------
// PNG generation helpers (for image fixture)
// ---------------------------------------------------------------------------

/// Computes CRC-32 (IEEE / Ethernet polynomial) over the given data.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Builds a PNG chunk: 4-byte length + 4-byte type + data + 4-byte CRC.
fn png_chunk(chunk_type: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + data.len());
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&chunk_type);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(&chunk_type);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

/// Computes Adler-32 checksum over the given data.
fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

/// Creates a minimal valid 1x1 red pixel PNG (RGB, 8-bit depth).
///
/// The image is constructed from raw IHDR/IDAT/IEND chunks with a
/// deflate "stored" block (no compression), so no external crate is needed.
fn create_red_png() -> Vec<u8> {
    let mut png = Vec::with_capacity(128);

    // PNG 8-byte signature
    png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR: width=1, height=1, bit_depth=8, color_type=2 (RGB),
    //       compression=0, filter=0, interlace=0
    let ihdr_data: Vec<u8> = [
        1u32.to_be_bytes().as_slice(), // width
        1u32.to_be_bytes().as_slice(), // height
        [0x08_u8, 0x02, 0x00, 0x00, 0x00].as_slice(),
    ]
    .concat();
    png.extend_from_slice(&png_chunk(*b"IHDR", &ihdr_data));

    // IDAT: zlib-wrapped deflate stored block containing one scanline.
    //   Scanline = [filter=None(0x00), R=0xFF, G=0x00, B=0x00]
    let raw_scanline: [u8; 4] = [0x00, 0xFF, 0x00, 0x00];
    let adler = adler32(&raw_scanline);
    let idat_data: Vec<u8> = [
        [0x78_u8, 0x01].as_slice(),       // zlib header (deflate, no dict)
        [0x01].as_slice(),                // BFINAL=1, BTYPE=00 (stored)
        4u16.to_le_bytes().as_slice(),    // LEN = 4
        (!4u16).to_le_bytes().as_slice(), // NLEN = bitwise complement of LEN
        raw_scanline.as_slice(),          // literal bytes
        adler.to_be_bytes().as_slice(),   // Adler-32 checksum
    ]
    .concat();
    png.extend_from_slice(&png_chunk(*b"IDAT", &idat_data));

    // IEND (empty data)
    png.extend_from_slice(&png_chunk(*b"IEND", &[]));

    png
}

// ---------------------------------------------------------------------------
// Collection of all five fidelity fixtures
// ---------------------------------------------------------------------------

/// All five fidelity fixtures, generated lazily on first access.
pub(crate) struct Fixtures {
    /// Fixture 1: simple text (1 heading + 3 paragraphs).
    pub simple: FidelityFixture,
    /// Fixture 2: table (header + 5 rows x 3 columns).
    pub table: FidelityFixture,
    /// Fixture 3: lists (unordered + ordered + nested).
    pub list: FidelityFixture,
    /// Fixture 4: rich text (bold, italic, underline, color, size).
    pub rich: FidelityFixture,
    /// Fixture 5: embedded 1x1 red PNG image.
    pub image: FidelityFixture,
}

impl Fixtures {
    /// Returns a reference to the lazily-initialized fixture collection.
    pub fn load() -> &'static Self {
        static INSTANCE: LazyLock<Fixtures> = LazyLock::new(Fixtures::generate);
        &INSTANCE
    }

    /// Returns references to all five fixtures in a fixed order.
    pub fn all(&self) -> Vec<&FidelityFixture> {
        vec![
            &self.simple,
            &self.table,
            &self.list,
            &self.rich,
            &self.image,
        ]
    }

    fn generate() -> Self {
        let simple = Self::build_simple();
        let table = Self::build_table();
        let list = Self::build_list();
        let rich = Self::build_rich();
        let image = Self::build_image();
        Self {
            simple,
            table,
            list,
            rich,
            image,
        }
    }

    // -----------------------------------------------------------------------
    // Fixture builders
    // -----------------------------------------------------------------------

    /// Fixture 1: 1 heading + 3 paragraphs.
    fn build_simple() -> FidelityFixture {
        let bytes = EasyDoc::document_to_bytes(|doc| {
            doc.title("Simple Fixture")
                .add_heading("Introduction", HeadingLevel::H1)
                .add_paragraph(Paragraph::new().add_text("First paragraph of the simple document."))
                .add_paragraph(
                    Paragraph::new().add_text("Second paragraph with additional content."),
                )
                .add_paragraph(Paragraph::new().add_text("Third and final paragraph."))
        })
        .expect("build simple fixture");

        let expected = Self::roundtrip_text(&bytes);

        FidelityFixture {
            name: "simple",
            original_size: bytes.len() as u64,
            expected_text: expected,
            docx_bytes: bytes,
        }
    }

    /// Fixture 2: table with header row + 5 data rows, 3 columns.
    fn build_table() -> FidelityFixture {
        let rows = vec![
            FixtureRow {
                name: "Alice".into(),
                value: "alpha".into(),
                score: 95.5,
            },
            FixtureRow {
                name: "Bob".into(),
                value: "beta".into(),
                score: 87.0,
            },
            FixtureRow {
                name: "Charlie".into(),
                value: "gamma".into(),
                score: 72.3,
            },
            FixtureRow {
                name: "Diana".into(),
                value: "delta".into(),
                score: 91.8,
            },
            FixtureRow {
                name: "Eve".into(),
                value: "epsilon".into(),
                score: 68.4,
            },
        ];

        let bytes = EasyDoc::document_to_bytes(|doc| {
            doc.title("Table Fixture")
                .add_heading("Data Table", HeadingLevel::H1)
                .add_paragraph(Paragraph::new().add_text("Below is a table with 5 rows of data."))
                .add_table(Table::from_data(&rows))
        })
        .expect("build table fixture");

        let expected = Self::roundtrip_text(&bytes);

        FidelityFixture {
            name: "table",
            original_size: bytes.len() as u64,
            expected_text: expected,
            docx_bytes: bytes,
        }
    }

    /// Fixture 3: lists -- 3 unordered + 2 ordered (one with nested unordered).
    fn build_list() -> FidelityFixture {
        // DocBuilder has no add_list; construct DocumentContent directly.
        let content = DocumentContent {
            blocks: vec![
                // Unordered list with 3 items
                DocumentBlock::List(DocumentList {
                    ordered: false,
                    start_number: None,
                    items: vec![
                        Self::list_item("Unordered item one"),
                        Self::list_item("Unordered item two"),
                        Self::list_item("Unordered item three"),
                    ],
                }),
                // Ordered list with 2 items, second one has a nested unordered list
                DocumentBlock::List(DocumentList {
                    ordered: true,
                    start_number: Some(1),
                    items: vec![
                        Self::list_item("Ordered item one"),
                        DocumentListItem {
                            blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                                text: "Ordered item two".into(),
                                ..DocumentTextRun::default()
                            }])],
                            nested: Some(Box::new(DocumentList {
                                ordered: false,
                                start_number: None,
                                items: vec![Self::list_item("Nested unordered item")],
                            })),
                        },
                    ],
                }),
            ],
            ..DocumentContent::default()
        };

        let bytes = EasyDoc::write_content_to_bytes(&content).expect("build list fixture");
        let expected = Self::roundtrip_text(&bytes);

        FidelityFixture {
            name: "list",
            original_size: bytes.len() as u64,
            expected_text: expected,
            docx_bytes: bytes,
        }
    }

    /// Fixture 4: rich text -- bold, italic, underline, color, font size.
    fn build_rich() -> FidelityFixture {
        let bytes = EasyDoc::document_to_bytes(|doc| {
            doc.title("Rich Text Fixture")
                .add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new("Bold text").bold())
                        .add_run(Run::new(" and "))
                        .add_run(Run::new("italic text").italic())
                        .add_run(Run::new(" and "))
                        .add_run(Run::new("underlined").underline())
                        .add_run(Run::new(" and "))
                        .add_run(Run::new("colored red").color(0xFF_0000))
                        .add_run(Run::new(" and "))
                        .add_run(Run::new("large size").size(36)),
                )
                .add_paragraph(
                    Paragraph::new().add_run(
                        Run::new("All styles combined")
                            .bold()
                            .italic()
                            .underline()
                            .color(0x00_00FF)
                            .size(28)
                            .font("Arial"),
                    ),
                )
        })
        .expect("build rich fixture");

        let expected = Self::roundtrip_text(&bytes);

        FidelityFixture {
            name: "rich",
            original_size: bytes.len() as u64,
            expected_text: expected,
            docx_bytes: bytes,
        }
    }

    /// Fixture 5: document with an embedded 1x1 red PNG image.
    fn build_image() -> FidelityFixture {
        let png_bytes = create_red_png();

        // Write the PNG to a temp file so DocImage can read it.
        let tmp_dir = tempfile::tempdir().expect("temp dir for image fixture");
        let png_path = tmp_dir.path().join("red.png");
        std::fs::write(&png_path, &png_bytes).expect("write temp png");

        let bytes = EasyDoc::document_to_bytes(|doc| {
            doc.title("Image Fixture")
                .add_heading("Embedded Image", HeadingLevel::H1)
                .add_paragraph(Paragraph::new().add_text("Below is a tiny red pixel PNG image."))
                .add_image(easydoc::DocImage::new(&png_path).alt_text("Red pixel"))
        })
        .expect("build image fixture");

        let expected = Self::roundtrip_text(&bytes);

        // tmp_dir is dropped here -- the PNG file is deleted, but the DOCX
        // bytes already contain an embedded copy.
        drop(tmp_dir);

        FidelityFixture {
            name: "image",
            original_size: bytes.len() as u64,
            expected_text: expected,
            docx_bytes: bytes,
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Creates a single-item list entry containing a plain-text paragraph.
    fn list_item(text: &str) -> DocumentListItem {
        DocumentListItem {
            blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                text: text.into(),
                ..DocumentTextRun::default()
            }])],
            nested: None,
        }
    }

    /// Writes DOCX bytes to a temp file, reads them back with `view_as(Plain)`,
    /// and returns the rendered text.  This is the "expected output" used for
    /// fidelity comparison.
    fn roundtrip_text(docx_bytes: &[u8]) -> String {
        let tmp = tempfile::Builder::new()
            .suffix(".docx")
            .tempfile()
            .expect("temp file for roundtrip");
        std::fs::write(tmp.path(), docx_bytes).expect("write roundtrip docx");
        EasyDoc::view_as(tmp.path(), &ViewMode::Plain).expect("view_as for roundtrip")
    }
}
