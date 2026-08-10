//! Custom type converter: define `StatusConverter` for a `Status` enum,
//! register it in a `ConverterRegistry`, and verify round-trip conversion
//! via `to_row_with_converters` / `from_row_with_converters`.
#![allow(clippy::doc_markdown)]

use easydoc::prelude::*;
use easydoc::{CellData, ConverterRegistry, DocConverter, DocxRow, EasyDoc, RowData, TableColumn};
use std::any::TypeId;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// A simple status enum that cannot be represented by a single DocValue variant
/// without a custom converter.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Active,
    Inactive,
    Suspended,
}

/// Bidirectional converter: `Status` <-> `DocValue::String`.
///
/// Maps each variant to a human-readable label (e.g. "ACTIVE") for writing,
/// and parses it back on reading.
struct StatusConverter;

impl DocConverter<Status> for StatusConverter {
    fn support_type() -> TypeId {
        TypeId::of::<Status>()
    }

    fn to_doc_value(&self, value: &Status, _column: &TableColumn) -> easydoc::Result<DocValue> {
        match value {
            Status::Active => Ok(DocValue::String("ACTIVE".into())),
            Status::Inactive => Ok(DocValue::String("INACTIVE".into())),
            Status::Suspended => Ok(DocValue::String("SUSPENDED".into())),
        }
    }

    fn from_doc_value(&self, value: &DocValue, column: &TableColumn) -> easydoc::Result<Status> {
        match value {
            DocValue::String(s) => match s.as_str() {
                "ACTIVE" => Ok(Status::Active),
                "INACTIVE" => Ok(Status::Inactive),
                "SUSPENDED" => Ok(Status::Suspended),
                other => Err(easydoc::DocError::Conversion {
                    field: column.field_name.clone(),
                    value: other.to_owned(),
                    message: "unknown status value, expected ACTIVE/INACTIVE/SUSPENDED".into(),
                }),
            },
            other => Err(easydoc::DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "expected string for Status".into(),
            }),
        }
    }
}

/// A user record with a custom-converted `Status` field.
#[derive(Debug, Clone)]
struct User {
    name: String,
    age: u32,
    status: Status,
}

impl DocxRow for User {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| {
            vec![
                TableColumn::new("Name", "name", 0),
                TableColumn::new("Age", "age", 1),
                TableColumn::new("Status", "status", 2),
            ]
        });
        &SCHEMA
    }

    fn from_row(row: &RowData) -> easydoc::Result<Self> {
        Self::from_row_with_converters(row, &ConverterRegistry::new())
    }

    fn from_row_with_converters(
        row: &RowData,
        registry: &ConverterRegistry,
    ) -> easydoc::Result<Self> {
        let name = match &row.cells.first() {
            Some(cell) => match &cell.value {
                DocValue::String(s) => s.clone(),
                other => format!("{other:?}"),
            },
            None => String::new(),
        };
        let age: u32 = match row.cells.get(1) {
            Some(cell) => match &cell.value {
                DocValue::Int(n) => *n as u32,
                DocValue::String(s) => s.parse().unwrap_or(0),
                _ => 0,
            },
            None => 0,
        };
        let status: Status = match row.cells.get(2) {
            Some(cell) => registry
                .from_doc_value(&cell.value, &TableColumn::new("Status", "status", 2))
                .unwrap_or(Status::Inactive),
            None => Status::Inactive,
        };
        Ok(User { name, age, status })
    }

    fn to_row(&self) -> easydoc::Result<Vec<CellData>> {
        self.to_row_with_converters(&ConverterRegistry::new())
    }

    fn to_row_with_converters(
        &self,
        registry: &ConverterRegistry,
    ) -> easydoc::Result<Vec<CellData>> {
        let status_val =
            registry.to_doc_value(&self.status, &TableColumn::new("Status", "status", 2))?;
        Ok(vec![
            CellData::new(self.name.clone()),
            CellData::new(i64::from(self.age)),
            CellData::new(status_val),
        ])
    }
}

fn main() -> easydoc::Result<()> {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("custom_converter.docx");

    // Step 1: Build data with custom Status values.
    println!("Step 1: Creating User records with custom Status enum...");
    let users = vec![
        User {
            name: "Alice".into(),
            age: 30,
            status: Status::Active,
        },
        User {
            name: "Bob".into(),
            age: 25,
            status: Status::Inactive,
        },
        User {
            name: "Charlie".into(),
            age: 35,
            status: Status::Suspended,
        },
    ];

    for user in &users {
        println!(
            "  {} | Age: {} | Status: {:?}",
            user.name, user.age, user.status
        );
    }

    // Step 2: Build a ConverterRegistry and register StatusConverter.
    println!("\nStep 2: Registering StatusConverter in ConverterRegistry...");
    let mut registry = ConverterRegistry::new();
    registry.register_named::<Status, _>("StatusConverter", StatusConverter);
    println!(
        "  Registry contains Status: {}",
        registry.contains::<Status>()
    );
    println!(
        "  Find by name 'StatusConverter': {}",
        registry.find_converter_by_name("StatusConverter").is_some()
    );

    // Step 3: Serialize rows with the converter registry to verify the mapping.
    // Note: `EasyDoc::write_table` uses `to_row()` (empty registry), so to
    // demonstrate the converter we call `to_row_with_converters` directly.
    println!("\nStep 3: Serializing User rows with StatusConverter...");
    for user in &users {
        let cells = user.to_row_with_converters(&registry)?;
        let status_cell = &cells[2];
        println!(
            "  {} -> status cell value: {:?}",
            user.name, status_cell.value
        );
    }

    // Step 4: Write the table to DOCX and read back.
    println!("\nStep 4: Writing table to DOCX...");
    EasyDoc::write_table(&path, &users).do_write()?;
    println!("  Saved to: {}", path.display());

    let text = EasyDoc::read_text(&path)?;
    println!("  Contains 'Alice': {}", text.contains("Alice"));
    println!("  Contains 'Active': {}", text.contains("Active"));

    // Step 5: Direct registry round-trip — the core converter test.
    println!("\nStep 5: ConverterRegistry round-trip (to_doc_value <-> from_doc_value)...");
    let col = TableColumn::new("Status", "status", 0);
    for status in &[Status::Active, Status::Inactive, Status::Suspended] {
        let doc_val = registry.to_doc_value(status, &col)?;
        let back: Status = registry.from_doc_value(&doc_val, &col)?;
        println!("  {status:?} -> {doc_val:?} -> {back:?} (round-trip OK)");
        assert_eq!(*status, back);
    }

    // Step 6: Verify `from_row_with_converters` uses the converter.
    println!("\nStep 6: Round-trip via from_row_with_converters...");
    let sample_user = &users[0];
    let cells = sample_user.to_row_with_converters(&registry)?;
    let row_data = easydoc::RowData::new(cells);
    let recovered = User::from_row_with_converters(&row_data, &registry)?;
    println!(
        "  Original:  {} | {:?}",
        sample_user.name, sample_user.status
    );
    println!("  Recovered: {} | {:?}", recovered.name, recovered.status);
    assert_eq!(sample_user.name, recovered.name);
    assert_eq!(sample_user.status, recovered.status);
    println!("  Round-trip verified!");

    println!("\nDone.");
    Ok(())
}
