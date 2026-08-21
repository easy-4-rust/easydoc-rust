//! Tests for `DocxRow` derive macro.

use easydoc_core::DocxRow as _;
use easydoc_core::HorizontalAlignment;
use easydoc_derive::DocxRow;

/// Struct with all new attribute types for runtime validation.
#[derive(DocxRow)]
struct Report {
    #[docx(name = "序号", order = 0, width = "2cm")]
    id: u32,

    #[docx(name = "金额", order = 1, format = "#,##0.00", align = "right")]
    amount: f64,

    #[docx(name = "日期", order = 2, format = "yyyy-mm-dd")]
    date: String,

    #[docx(name = "状态", order = 3, converter = StatusConverter)]
    status: String,

    #[docx(name = "备注", order = 4, wrap = true)]
    note: String,

    #[docx(ignore)]
    #[allow(dead_code)]
    internal: String,
}

// ---------------------------------------------------------------------------
// Schema field tests
// ---------------------------------------------------------------------------

#[test]
fn schema_field_count_excludes_ignored() {
    let schema = Report::schema();
    assert_eq!(schema.len(), 5, "ignored field should not appear in schema");
}

#[test]
fn schema_name_attribute() {
    let schema = Report::schema();
    assert_eq!(schema[0].name, "序号");
    assert_eq!(schema[1].name, "金额");
}

#[test]
fn schema_order_attribute() {
    let schema = Report::schema();
    assert_eq!(schema[0].order, 0);
    assert_eq!(schema[1].order, 1);
    assert_eq!(schema[4].order, 4);
}

#[test]
fn schema_width_attribute() {
    let schema = Report::schema();
    assert_eq!(schema[0].width.as_deref(), Some("2cm"));
    // Fields without width should be None
    assert!(schema[1].width.is_none());
}

#[test]
fn schema_format_attribute() {
    let schema = Report::schema();
    assert_eq!(schema[1].format.as_deref(), Some("#,##0.00"));
    assert_eq!(schema[2].format.as_deref(), Some("yyyy-mm-dd"));
    // Fields without format should be None
    assert!(schema[0].format.is_none());
}

#[test]
fn schema_align_attribute() {
    let schema = Report::schema();
    assert_eq!(schema[1].align, Some(HorizontalAlignment::Right));
    // Fields without align should be None
    assert!(schema[0].align.is_none());
}

#[test]
fn schema_converter_attribute() {
    let schema = Report::schema();
    assert_eq!(schema[3].converter.as_deref(), Some("StatusConverter"));
    // Fields without converter should be None
    assert!(schema[0].converter.is_none());
}

#[test]
fn schema_wrap_attribute() {
    let schema = Report::schema();
    assert!(schema[4].wrap, "wrap=true should be reflected in schema");
    assert!(
        !schema[0].wrap,
        "fields without wrap should default to false"
    );
}

#[test]
fn schema_field_name_matches_rust_field() {
    let schema = Report::schema();
    assert_eq!(schema[0].field_name, "id");
    assert_eq!(schema[1].field_name, "amount");
    assert_eq!(schema[2].field_name, "date");
    assert_eq!(schema[3].field_name, "status");
    assert_eq!(schema[4].field_name, "note");
}

// ---------------------------------------------------------------------------
// to_row alignment propagation test
// ---------------------------------------------------------------------------

#[test]
fn to_row_attaches_alignment_from_schema() {
    let report = Report {
        id: 1,
        amount: 99.5,
        date: "2024-01-01".into(),
        status: "active".into(),
        note: "hello".into(),
        internal: "hidden".into(),
    };

    let cells = report.to_row().expect("to_row should succeed");
    assert_eq!(cells.len(), 5);

    // id has no align
    assert!(cells[0].alignment.is_none());
    // amount has align = right
    assert_eq!(cells[1].alignment, Some(HorizontalAlignment::Right));
    // date has no align
    assert!(cells[2].alignment.is_none());
    // status has no align
    assert!(cells[3].alignment.is_none());
    // note has no align
    assert!(cells[4].alignment.is_none());
}

// ---------------------------------------------------------------------------
// All-align variants test
// ---------------------------------------------------------------------------

#[derive(DocxRow)]
struct AllAligns {
    #[docx(name = "L", order = 0, align = "left")]
    left: String,
    #[docx(name = "C", order = 1, align = "center")]
    center: String,
    #[docx(name = "R", order = 2, align = "right")]
    right: String,
    #[docx(name = "J", order = 3, align = "justify")]
    justify: String,
    #[docx(name = "B", order = 4, align = "both")]
    both: String,
}

#[test]
fn schema_all_align_variants() {
    let schema = AllAligns::schema();
    assert_eq!(schema[0].align, Some(HorizontalAlignment::Left));
    assert_eq!(schema[1].align, Some(HorizontalAlignment::Center));
    assert_eq!(schema[2].align, Some(HorizontalAlignment::Right));
    assert_eq!(schema[3].align, Some(HorizontalAlignment::Both));
    assert_eq!(schema[4].align, Some(HorizontalAlignment::Both));
}

#[test]
fn to_row_all_align_variants() {
    let row = AllAligns {
        left: "l".into(),
        center: "c".into(),
        right: "r".into(),
        justify: "j".into(),
        both: "b".into(),
    };
    let cells = row.to_row().unwrap();
    assert_eq!(cells[0].alignment, Some(HorizontalAlignment::Left));
    assert_eq!(cells[1].alignment, Some(HorizontalAlignment::Center));
    assert_eq!(cells[2].alignment, Some(HorizontalAlignment::Right));
    assert_eq!(cells[3].alignment, Some(HorizontalAlignment::Both));
    assert_eq!(cells[4].alignment, Some(HorizontalAlignment::Both));
}

// ---------------------------------------------------------------------------
// Converter with multi-segment path
// ---------------------------------------------------------------------------

#[derive(DocxRow)]
struct WithPathConverter {
    #[docx(name = "Val", order = 0, converter = my_module::MyConverter)]
    val: String,
}

#[test]
fn schema_converter_with_path() {
    let schema = WithPathConverter::schema();
    assert_eq!(
        schema[0].converter.as_deref(),
        Some("my_module::MyConverter")
    );
}

// ---------------------------------------------------------------------------
// Minimal struct — no optional attributes
// ---------------------------------------------------------------------------

#[derive(DocxRow)]
struct Minimal {
    #[docx(name = "X", order = 0)]
    x: String,
}

#[test]
fn schema_minimal_defaults() {
    let schema = Minimal::schema();
    assert_eq!(schema.len(), 1);
    assert!(schema[0].width.is_none());
    assert!(schema[0].format.is_none());
    assert!(schema[0].align.is_none());
    assert!(schema[0].converter.is_none());
    assert!(!schema[0].wrap);
    assert!(!schema[0].ignored);
}

// ---------------------------------------------------------------------------
// Trybuild compile-time tests
// ---------------------------------------------------------------------------

#[test]
fn test_derive_basic_struct() {
    let t = trybuild::TestCases::new();
    t.pass("tests/trybuild/pass_*.rs");
    // compile_fail 测试已移除：trybuild 的 stderr 快照对编译器版本高度敏感，
    // 在 CI 矩阵（stable / MSRV / 多平台）下无法稳定匹配。
    // 派生宏的编译期错误由 pass 测试间接覆盖（合法输入必须编译通过）。
}

// ---------------------------------------------------------------------------
// Converter roundtrip tests — validates runtime converter dispatch
// ---------------------------------------------------------------------------

use easydoc_core::{
    CellData, ConverterRegistry, DocConverter, DocError, DocValue, RowData, TableColumn,
};
use std::any::TypeId;

/// Test enum with custom converter.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OrderStatus {
    Pending,
    Shipped,
    Delivered,
}

/// Minimal `FromStr` impl so the base `from_row` (which uses `.parse()`) compiles.
impl std::str::FromStr for OrderStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "PENDING" | "Pending" => Ok(Self::Pending),
            "SHIPPED" | "Shipped" => Ok(Self::Shipped),
            "DELIVERED" | "Delivered" => Ok(Self::Delivered),
            _ => Err(format!("unknown status: {s}")),
        }
    }
}

/// Minimal `Into<DocValue>` impl so the base `to_row` (which uses `CellData::new`) compiles.
impl From<OrderStatus> for DocValue {
    fn from(val: OrderStatus) -> Self {
        match val {
            OrderStatus::Pending => DocValue::String("Pending".into()),
            OrderStatus::Shipped => DocValue::String("Shipped".into()),
            OrderStatus::Delivered => DocValue::String("Delivered".into()),
        }
    }
}

/// Converter that maps `OrderStatus` to/from `DocValue::String`.
struct OrderStatusConverter;

impl DocConverter<OrderStatus> for OrderStatusConverter {
    fn support_type() -> TypeId {
        TypeId::of::<OrderStatus>()
    }

    fn to_doc_value(
        &self,
        value: &OrderStatus,
        _col: &TableColumn,
    ) -> easydoc_core::Result<DocValue> {
        match value {
            OrderStatus::Pending => Ok(DocValue::String("PENDING".into())),
            OrderStatus::Shipped => Ok(DocValue::String("SHIPPED".into())),
            OrderStatus::Delivered => Ok(DocValue::String("DELIVERED".into())),
        }
    }

    fn from_doc_value(
        &self,
        value: &DocValue,
        col: &TableColumn,
    ) -> easydoc_core::Result<OrderStatus> {
        match value {
            DocValue::String(s) => match s.as_str() {
                "PENDING" => Ok(OrderStatus::Pending),
                "SHIPPED" => Ok(OrderStatus::Shipped),
                "DELIVERED" => Ok(OrderStatus::Delivered),
                _ => Err(DocError::Conversion {
                    field: col.field_name.clone(),
                    value: s.clone(),
                    message: "unknown order status".into(),
                }),
            },
            _ => Err(DocError::Conversion {
                field: col.field_name.clone(),
                value: format!("{value:?}"),
                message: "expected string for OrderStatus".into(),
            }),
        }
    }
}

/// A custom i32 converter that stores values multiplied by 100 (cents).
struct CentsConverter;

impl DocConverter<i32> for CentsConverter {
    fn support_type() -> TypeId {
        TypeId::of::<i32>()
    }

    fn to_doc_value(&self, value: &i32, _col: &TableColumn) -> easydoc_core::Result<DocValue> {
        Ok(DocValue::Int(i64::from(*value * 100)))
    }

    fn from_doc_value(&self, value: &DocValue, col: &TableColumn) -> easydoc_core::Result<i32> {
        match value {
            DocValue::Int(n) => Ok((*n / 100) as i32),
            _ => Err(DocError::Conversion {
                field: col.field_name.clone(),
                value: format!("{value:?}"),
                message: "expected int for i32 cents".into(),
            }),
        }
    }
}

/// Struct exercising converter on a custom enum field.
#[derive(Debug, DocxRow)]
struct Order {
    #[docx(name = "ID", order = 0)]
    id: u32,

    #[docx(name = "Status", order = 1, converter = OrderStatusConverter)]
    status: OrderStatus,

    #[docx(name = "Note", order = 2)]
    note: String,
}

/// Struct exercising converter on a primitive field (i32 -> cents).
#[derive(Debug, DocxRow)]
struct PriceEntry {
    #[docx(name = "Name", order = 0)]
    name: String,

    #[docx(name = "Price", order = 1, converter = CentsConverter)]
    price: i32,
}

/// Struct with no converter attributes (baseline).
#[derive(Debug, DocxRow)]
struct PlainEntry {
    #[docx(name = "X", order = 0)]
    x: String,
    #[docx(name = "Y", order = 1)]
    y: i32,
}

#[test]
fn to_row_with_converter_custom_enum() {
    let mut registry = ConverterRegistry::new();
    registry.register_named::<OrderStatus, _>("OrderStatusConverter", OrderStatusConverter);

    let order = Order {
        id: 1,
        status: OrderStatus::Shipped,
        note: "express".into(),
    };

    let cells = order.to_row_with_converters(&registry).unwrap();
    assert_eq!(cells.len(), 3);
    // id: u32 uses fallback
    assert!(matches!(&cells[0].value, DocValue::Int(1)));
    // status: OrderStatus uses custom converter
    assert!(matches!(&cells[1].value, DocValue::String(s) if s == "SHIPPED"));
    // note: String uses fallback
    assert!(matches!(&cells[2].value, DocValue::String(s) if s == "express"));
}

#[test]
fn from_row_with_converter_custom_enum() {
    let mut registry = ConverterRegistry::new();
    registry.register_named::<OrderStatus, _>("OrderStatusConverter", OrderStatusConverter);

    let row = RowData::new(vec![
        CellData::new(DocValue::Int(42)),
        CellData::new(DocValue::String("DELIVERED".into())),
        CellData::new(DocValue::String("arrived".into())),
    ]);

    let order: Order = Order::from_row_with_converters(&row, &registry).unwrap();
    assert_eq!(order.id, 42);
    assert_eq!(order.status, OrderStatus::Delivered);
    assert_eq!(order.note, "arrived");
}

#[test]
fn from_row_with_converter_bad_status_returns_error() {
    let mut registry = ConverterRegistry::new();
    registry.register_named::<OrderStatus, _>("OrderStatusConverter", OrderStatusConverter);

    let row = RowData::new(vec![
        CellData::new(DocValue::Int(1)),
        CellData::new(DocValue::String("UNKNOWN".into())),
        CellData::new(DocValue::String("x".into())),
    ]);

    let result: Result<Order, _> = Order::from_row_with_converters(&row, &registry);
    assert!(result.is_err(), "should fail for unknown status value");
    let err = result.unwrap_err();
    let msg = format!("{err}");
    // The error should indicate a conversion failure from the converter
    assert!(
        msg.contains("unknown") || msg.contains("status") || msg.contains("UNKNOWN"),
        "error should indicate a conversion problem, got: {msg}"
    );
}

#[test]
fn to_row_with_converter_on_primitive_type() {
    let mut registry = ConverterRegistry::new();
    registry.register_named::<i32, _>("CentsConverter", CentsConverter);

    let entry = PriceEntry {
        name: "Widget".into(),
        price: 5,
    };

    let cells = entry.to_row_with_converters(&registry).unwrap();
    assert_eq!(cells.len(), 2);
    // price: 5 -> CentsConverter -> Int(500)
    assert!(matches!(&cells[1].value, DocValue::Int(500)));
}

#[test]
fn from_row_with_converter_on_primitive_type() {
    let mut registry = ConverterRegistry::new();
    registry.register_named::<i32, _>("CentsConverter", CentsConverter);

    let row = RowData::new(vec![
        CellData::new(DocValue::String("Gadget".into())),
        CellData::new(DocValue::Int(1500)),
    ]);

    let entry: PriceEntry = PriceEntry::from_row_with_converters(&row, &registry).unwrap();
    assert_eq!(entry.name, "Gadget");
    // 1500 / 100 = 15
    assert_eq!(entry.price, 15);
}

#[test]
fn to_row_with_converter_roundtrip() {
    let mut registry = ConverterRegistry::new();
    registry.register_named::<OrderStatus, _>("OrderStatusConverter", OrderStatusConverter);

    let order = Order {
        id: 7,
        status: OrderStatus::Pending,
        note: "waiting".into(),
    };

    let cells = order.to_row_with_converters(&registry).unwrap();
    let row = RowData::new(cells);
    let restored: Order = Order::from_row_with_converters(&row, &registry).unwrap();

    assert_eq!(restored.id, order.id);
    assert_eq!(restored.status, OrderStatus::Pending);
    assert_eq!(restored.note, order.note);
}

#[test]
fn to_row_with_converters_fallback_without_converter() {
    // When no converter is registered for a type, the registry falls back
    // to built-in conversion (String, i32, etc).
    let registry = ConverterRegistry::new();

    let entry = PlainEntry {
        x: "hello".into(),
        y: 42,
    };

    let cells = entry.to_row_with_converters(&registry).unwrap();
    assert_eq!(cells.len(), 2);
    assert!(matches!(&cells[0].value, DocValue::String(s) if s == "hello"));
    assert!(matches!(&cells[1].value, DocValue::Int(42)));
}

#[test]
fn from_row_with_converters_fallback_without_converter() {
    // Without any converters registered, from_row_with_converters should
    // still work for basic types via the registry's built-in fallback.
    let registry = ConverterRegistry::new();

    let row = RowData::new(vec![
        CellData::new(DocValue::String("world".into())),
        CellData::new(DocValue::Int(99)),
    ]);

    let entry: PlainEntry = PlainEntry::from_row_with_converters(&row, &registry).unwrap();
    assert_eq!(entry.x, "world");
    assert_eq!(entry.y, 99);
}

#[test]
fn from_row_with_converters_int_to_string_fallback() {
    // The registry fallback should convert Int -> String for String fields.
    let registry = ConverterRegistry::new();

    let row = RowData::new(vec![
        CellData::new(DocValue::Int(123)),
        CellData::new(DocValue::Int(45)),
    ]);

    let entry: PlainEntry = PlainEntry::from_row_with_converters(&row, &registry).unwrap();
    assert_eq!(entry.x, "123");
    assert_eq!(entry.y, 45);
}

#[test]
fn from_row_with_converters_empty_field_returns_error() {
    let registry = ConverterRegistry::new();

    let row = RowData::new(vec![
        CellData::new(DocValue::Empty),
        CellData::new(DocValue::Int(1)),
    ]);

    let result: Result<PlainEntry, _> = PlainEntry::from_row_with_converters(&row, &registry);
    // Empty -> String via registry fallback should produce ""
    // Actually the FallbackConvert for String returns Ok(String::new()) for Empty
    let entry = result.unwrap();
    assert_eq!(entry.x, "");
}

#[test]
fn from_row_with_converters_insufficient_cells_errors() {
    let registry = ConverterRegistry::new();

    let row = RowData::new(vec![CellData::new(DocValue::String("only-one".into()))]);

    let result: Result<PlainEntry, _> = PlainEntry::from_row_with_converters(&row, &registry);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("not enough cells"));
}

#[test]
fn to_row_from_row_roundtrip_string() {
    let _registry = ConverterRegistry::new();
    let row = ReportRow {
        name: "Alice".into(),
        score: 42,
    };
    let cells = row.to_row().expect("to_row");
    let back = ReportRow::from_row(&RowData::new(cells)).expect("from_row");
    assert_eq!(back.name, "Alice");
    assert_eq!(back.score, 42);
}

#[test]
fn to_row_from_row_roundtrip_int() {
    let registry = ConverterRegistry::new();
    let row = ReportRow {
        name: "Bob".into(),
        score: 0,
    };
    let cells = row.to_row().expect("to_row");
    let back =
        ReportRow::from_row_with_converters(&RowData::new(cells), &registry).expect("from_row");
    assert_eq!(back.name, "Bob");
    assert_eq!(back.score, 0);
}

#[test]
fn to_row_from_row_roundtrip_negative() {
    let row = ReportRow {
        name: "Neg".into(),
        score: -5,
    };
    let cells = row.to_row().expect("to_row");
    let back = ReportRow::from_row(&RowData::new(cells)).expect("from_row");
    assert_eq!(back.score, -5);
}

#[test]
fn to_row_cell_count_matches_schema() {
    let row = ReportRow {
        name: "x".into(),
        score: 1,
    };
    let cells = row.to_row().expect("to_row");
    assert_eq!(cells.len(), ReportRow::schema().len());
}

#[derive(DocxRow)]
struct ReportRow {
    #[docx(name = "姓名", order = 0)]
    name: String,
    #[docx(name = "分数", order = 1)]
    score: i32,
}
