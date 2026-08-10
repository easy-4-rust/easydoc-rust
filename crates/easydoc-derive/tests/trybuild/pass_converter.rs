//! Trybuild compile-pass test: converter attribute with real converter types.
//!
//! Verifies that `#[docx(converter = StatusConverter)]` generates code that
//! compiles against `ConverterRegistry`, `ErasedConverter`, and `DocConverter`.

use easydoc_core::{
    CellData, ConverterRegistry, DocConverter, DocError, DocValue, DocxRow as _, Result,
    RowData, TableColumn,
};
use easydoc_derive::DocxRow;
use std::any::TypeId;

// ---------------------------------------------------------------------------
// Custom enum and converter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Active,
    Inactive,
}

/// FromStr so the base `from_row` (which uses `.parse()`) compiles.
impl std::str::FromStr for Status {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "ACTIVE" | "Active" => Ok(Self::Active),
            "INACTIVE" | "Inactive" => Ok(Self::Inactive),
            _ => Err(format!("unknown status: {s}")),
        }
    }
}

/// Into<DocValue> so the base `to_row` (which uses CellData::new) compiles.
impl From<Status> for DocValue {
    fn from(val: Status) -> Self {
        match val {
            Status::Active => DocValue::String("Active".into()),
            Status::Inactive => DocValue::String("Inactive".into()),
        }
    }
}

struct StatusConverter;

impl DocConverter<Status> for StatusConverter {
    fn support_type() -> TypeId {
        TypeId::of::<Status>()
    }

    fn to_doc_value(&self, value: &Status, _column: &TableColumn) -> Result<DocValue> {
        match value {
            Status::Active => Ok(DocValue::String("ACTIVE".into())),
            Status::Inactive => Ok(DocValue::String("INACTIVE".into())),
        }
    }

    fn from_doc_value(&self, value: &DocValue, column: &TableColumn) -> Result<Status> {
        match value {
            DocValue::String(s) => match s.as_str() {
                "ACTIVE" => Ok(Status::Active),
                "INACTIVE" => Ok(Status::Inactive),
                _ => Err(DocError::Conversion {
                    field: column.field_name.clone(),
                    value: s.clone(),
                    message: "unknown status".into(),
                }),
            },
            _ => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{value:?}"),
                message: "expected string for Status".into(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Struct using the converter attribute
// ---------------------------------------------------------------------------

#[derive(DocxRow)]
struct User {
    #[docx(name = "Name", order = 0)]
    name: String,

    #[docx(name = "Status", order = 1, converter = StatusConverter)]
    status: Status,

    #[docx(name = "Age", order = 2)]
    age: i32,
}

fn main() {
    // Schema must include the converter name
    let schema = User::schema();
    assert_eq!(schema[1].converter.as_deref(), Some("StatusConverter"));

    // Registry setup
    let mut registry = ConverterRegistry::new();
    registry.register_named::<Status, _>("StatusConverter", StatusConverter);

    // to_row_with_converters: Status::Active -> DocValue::String("ACTIVE")
    let user = User {
        name: "Alice".into(),
        status: Status::Active,
        age: 30,
    };
    let cells: Vec<CellData> = user.to_row_with_converters(&registry).unwrap();
    assert_eq!(cells.len(), 3);
    assert!(matches!(&cells[1].value, DocValue::String(s) if s == "ACTIVE"));

    // from_row_with_converters: DocValue::String("INACTIVE") -> Status::Inactive
    let row = RowData::new(vec![
        CellData::new(DocValue::String("Bob".into())),
        CellData::new(DocValue::String("INACTIVE".into())),
        CellData::new(DocValue::Int(25)),
    ]);
    let user2: User = User::from_row_with_converters(&row, &registry).unwrap();
    assert_eq!(user2.name, "Bob");
    assert_eq!(user2.status, Status::Inactive);
    assert_eq!(user2.age, 25);
}
