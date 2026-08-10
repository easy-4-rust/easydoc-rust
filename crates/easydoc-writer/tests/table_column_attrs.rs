//! Integration tests for table column attributes (`width`, `wrap`, `format`, `align`).
//!
//! Each test constructs a typed table via `TableWriteBuilder`, writes it to bytes,
//! then re-opens the DOCX ZIP and inspects the raw `word/document.xml` for the
//! expected OOXML elements.

use std::io::{Cursor, Read as _};

use easydoc_core::metadata::TableColumn;
use easydoc_core::types::HorizontalAlignment;
use easydoc_core::{CellData, ConverterRegistry, DocxRow, Result, RowData};
use easydoc_writer::TableWriteBuilder;

// ---------------------------------------------------------------------------
// Test fixture: a struct whose `schema()` includes all column attributes.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Record {
    name: String,
    amount: f64,
    note: String,
}

impl DocxRow for Record {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| {
            vec![
                TableColumn::new("Name", "name", 0).width("2cm"),
                TableColumn::new("Amount", "amount", 1)
                    .width("80px")
                    .format("#,##0.00")
                    .align(HorizontalAlignment::Right),
                TableColumn::new("Note", "note", 2).width("50%"),
            ]
        });
        &SCHEMA
    }

    fn from_row(_row: &RowData) -> Result<Self> {
        unimplemented!()
    }
    fn from_row_with_converters(_row: &RowData, _registry: &ConverterRegistry) -> Result<Self> {
        unimplemented!()
    }
    fn to_row(&self) -> Result<Vec<CellData>> {
        Ok(vec![
            CellData::new(self.name.clone()),
            CellData::new(self.amount),
            CellData::new(self.note.clone()),
        ])
    }
    fn to_row_with_converters(&self, _registry: &ConverterRegistry) -> Result<Vec<CellData>> {
        self.to_row()
    }
}

// A fixture with wrap=true (explicit opt-in to wrapping).
#[derive(Debug, Clone)]
struct WrapRecord {
    label: String,
}

impl DocxRow for WrapRecord {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| {
            vec![TableColumn::new("Label", "label", 0).width("3cm").wrap()]
        });
        &SCHEMA
    }

    fn from_row(_row: &RowData) -> Result<Self> {
        unimplemented!()
    }
    fn from_row_with_converters(_row: &RowData, _registry: &ConverterRegistry) -> Result<Self> {
        unimplemented!()
    }
    fn to_row(&self) -> Result<Vec<CellData>> {
        Ok(vec![CellData::new(self.label.clone())])
    }
    fn to_row_with_converters(&self, _registry: &ConverterRegistry) -> Result<Vec<CellData>> {
        self.to_row()
    }
}

// A fixture with wrap=false (explicit no-wrap).
#[derive(Debug, Clone)]
struct NoWrapRecord {
    label: String,
}

impl DocxRow for NoWrapRecord {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| {
            // wrap defaults to false in TableColumn::new
            vec![TableColumn::new("Label", "label", 0).width("3cm")]
        });
        &SCHEMA
    }

    fn from_row(_row: &RowData) -> Result<Self> {
        unimplemented!()
    }
    fn from_row_with_converters(_row: &RowData, _registry: &ConverterRegistry) -> Result<Self> {
        unimplemented!()
    }
    fn to_row(&self) -> Result<Vec<CellData>> {
        Ok(vec![CellData::new(self.label.clone())])
    }
    fn to_row_with_converters(&self, _registry: &ConverterRegistry) -> Result<Vec<CellData>> {
        self.to_row()
    }
}

// A fixture specifically for testing alignment on data cells.
#[derive(Debug, Clone)]
struct AlignRecord {
    value: String,
}

impl DocxRow for AlignRecord {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| {
            vec![TableColumn::new("Value", "value", 0).align(HorizontalAlignment::Center)]
        });
        &SCHEMA
    }

    fn from_row(_row: &RowData) -> Result<Self> {
        unimplemented!()
    }
    fn from_row_with_converters(_row: &RowData, _registry: &ConverterRegistry) -> Result<Self> {
        unimplemented!()
    }
    fn to_row(&self) -> Result<Vec<CellData>> {
        Ok(vec![CellData::new(self.value.clone())])
    }
    fn to_row_with_converters(&self, _registry: &ConverterRegistry) -> Result<Vec<CellData>> {
        self.to_row()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Writes a `Vec<T>` to a DOCX in memory and extracts `word/document.xml`.
fn extract_document_xml<T: DocxRow>(data: &[T]) -> String {
    let bytes = TableWriteBuilder::new("test.docx", data)
        .do_write_to_bytes()
        .expect("write failed");

    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("open zip failed");

    let mut xml_file = archive
        .by_name("word/document.xml")
        .expect("word/document.xml not found");

    let mut xml = String::new();
    xml_file
        .read_to_string(&mut xml)
        .expect("read document.xml failed");
    xml
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn width_2cm_applied_to_cells() {
    let data = vec![Record {
        name: "A".into(),
        amount: 1.0,
        note: "x".into(),
    }];
    let xml = extract_document_xml(&data);

    // "2cm" = 1134 twips.  Expect <w:tcW w:w="1134" w:type="dxa" /> in the output.
    assert!(
        xml.contains(r#"w:w="1134""#),
        "expected 1134 twips for 2cm width, XML snippet: ...{}...",
        &xml[xml.find("tcW").unwrap_or(0)..xml.find("tcW").map_or(0, |i| i + 80)]
    );
    assert!(xml.contains(r#"w:type="dxa""#), "expected dxa width type");
}

#[test]
fn width_80px_applied_to_cells() {
    let data = vec![Record {
        name: "A".into(),
        amount: 1.0,
        note: "x".into(),
    }];
    let xml = extract_document_xml(&data);

    // "80px" = 1200 twips.
    assert!(
        xml.contains(r#"w:w="1200""#),
        "expected 1200 twips for 80px"
    );
}

#[test]
fn width_50pct_applied_to_cells() {
    let data = vec![Record {
        name: "A".into(),
        amount: 1.0,
        note: "x".into(),
    }];
    let xml = extract_document_xml(&data);

    // "50%" = 2500 OOXML pct units (50 * 50).
    assert!(xml.contains(r#"w:w="2500""#), "expected 2500 for 50%");
    assert!(xml.contains(r#"w:type="pct""#), "expected pct width type");
}

#[test]
fn format_numfmt_applied_to_data_cells() {
    let data = vec![Record {
        name: "A".into(),
        amount: 1234.5,
        note: "x".into(),
    }];
    let xml = extract_document_xml(&data);

    // The "Amount" column has format="#,##0.00".
    // Expect <w:numFmt w:val="#,##0.00"/> in the document XML.
    assert!(
        xml.contains("w:numFmt w:val=\"#,##0.00\""),
        "expected numFmt #,##0.00 in document XML"
    );
}

#[test]
fn format_numfmt_not_on_header_cells() {
    let data = vec![Record {
        name: "A".into(),
        amount: 1.0,
        note: "x".into(),
    }];
    let xml = extract_document_xml(&data);

    // The numFmt should only appear ONCE (for the single data row's Amount cell),
    // NOT in the header row.
    let count = xml.matches("w:numFmt").count();
    assert_eq!(
        count, 1,
        "expected exactly 1 numFmt (data cell only), found {count}"
    );
}

#[test]
fn nowrap_inserted_when_wrap_is_false() {
    let data = vec![NoWrapRecord {
        label: "test".into(),
    }];
    let xml = extract_document_xml(&data);

    // wrap defaults to false -> expect <w:noWrap/> in the output.
    assert!(
        xml.contains("<w:noWrap/>"),
        "expected <w:noWrap/> when wrap=false, XML length: {}",
        xml.len()
    );
}

#[test]
fn no_nowrap_when_wrap_is_true() {
    let data = vec![WrapRecord {
        label: "test".into(),
    }];
    let xml = extract_document_xml(&data);

    // wrap=true -> no <w:noWrap/> should be emitted.
    assert!(
        !xml.contains("<w:noWrap/>"),
        "expected NO <w:noWrap/> when wrap=true"
    );
}

#[test]
fn alignment_applied_to_data_cell_paragraph() {
    let data = vec![AlignRecord {
        value: "hello".into(),
    }];
    let xml = extract_document_xml(&data);

    // The "Value" column has align=Center.
    // Expect <w:jc w:val="center"/> in the cell's paragraph properties.
    assert!(
        xml.contains(r#"w:jc w:val="center""#),
        "expected jc=center for alignment"
    );
}

#[test]
fn width_auto_produces_auto_type() {
    #[derive(Debug, Clone)]
    struct AutoRec {
        v: String,
    }
    impl DocxRow for AutoRec {
        fn schema() -> &'static [TableColumn] {
            static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> =
                std::sync::LazyLock::new(|| vec![TableColumn::new("V", "v", 0).width("auto")]);
            &SCHEMA
        }
        fn from_row(_: &RowData) -> Result<Self> {
            unimplemented!()
        }
        fn from_row_with_converters(_: &RowData, _: &ConverterRegistry) -> Result<Self> {
            unimplemented!()
        }
        fn to_row(&self) -> Result<Vec<CellData>> {
            Ok(vec![CellData::new(self.v.clone())])
        }
        fn to_row_with_converters(&self, _: &ConverterRegistry) -> Result<Vec<CellData>> {
            self.to_row()
        }
    }

    let data = vec![AutoRec { v: "x".into() }];
    let xml = extract_document_xml(&data);

    assert!(
        xml.contains(r#"w:type="auto""#),
        "expected auto width type for 'auto' width"
    );
}

#[test]
fn no_width_no_tcw_in_xml() {
    // A column with no width set should NOT produce a <w:tcW> element.
    #[derive(Debug, Clone)]
    struct BareRec {
        v: String,
    }
    impl DocxRow for BareRec {
        fn schema() -> &'static [TableColumn] {
            static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> =
                std::sync::LazyLock::new(|| vec![TableColumn::new("V", "v", 0)]);
            &SCHEMA
        }
        fn from_row(_: &RowData) -> Result<Self> {
            unimplemented!()
        }
        fn from_row_with_converters(_: &RowData, _: &ConverterRegistry) -> Result<Self> {
            unimplemented!()
        }
        fn to_row(&self) -> Result<Vec<CellData>> {
            Ok(vec![CellData::new(self.v.clone())])
        }
        fn to_row_with_converters(&self, _: &ConverterRegistry) -> Result<Vec<CellData>> {
            self.to_row()
        }
    }

    let data = vec![BareRec { v: "x".into() }];
    let xml = extract_document_xml(&data);

    // No width set -> no <w:tcW in the document XML.
    assert!(
        !xml.contains("<w:tcW"),
        "expected no <w:tcW when no width is set"
    );
}
