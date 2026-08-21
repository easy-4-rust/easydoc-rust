//! core types（DocValue / `CellData` / `RowData` / TableData）深度测试。

use easydoc_core::DocValue as DV;
use easydoc_core::types::{CellData, HorizontalAlignment, RowData, TableData};

// ===========================================================================
// DocValue From 转换
// ===========================================================================

#[test]
fn docvalue_from_string() {
    let v = DV::from("hello".to_owned());
    assert!(matches!(v, DV::String(s) if s == "hello"));
}

#[test]
fn docvalue_from_str_ref() {
    let v = DV::from("world");
    assert!(matches!(v, DV::String(s) if s == "world"));
}

#[test]
fn docvalue_from_bool() {
    assert!(matches!(DV::from(true), DV::Bool(true)));
    assert!(matches!(DV::from(false), DV::Bool(false)));
}

#[test]
fn docvalue_from_int() {
    assert!(matches!(DV::from(42i64), DV::Int(42)));
    assert!(matches!(DV::from(-7i64), DV::Int(-7)));
    assert!(matches!(DV::from(0i64), DV::Int(0)));
}

#[test]
fn docvalue_from_float() {
    // 用非 PI 近似值（2.5 避免 clippy::approx_constant）
    assert!(matches!(DV::from(2.5f64), DV::Float(f) if (f - 2.5).abs() < 1e-9));
    assert!(matches!(DV::from(0.0f64), DV::Float(_)));
}

#[test]
fn docvalue_debug_output() {
    assert_eq!(format!("{:?}", DV::String("x".into())), "String(\"x\")");
    assert_eq!(format!("{:?}", DV::Bool(true)), "Bool(true)");
    assert_eq!(format!("{:?}", DV::Int(5)), "Int(5)");
    assert_eq!(format!("{:?}", DV::Float(1.5)), "Float(1.5)");
}

#[test]
fn docvalue_clone() {
    let v = DV::String("clone me".into());
    let c = v.clone();
    assert!(matches!(c, DV::String(s) if s == "clone me"));
}

// ===========================================================================
// CellData
// ===========================================================================

#[test]
fn celldata_new_defaults() {
    let cell = CellData::new("text");
    assert!(matches!(cell.value, DV::String(s) if s == "text"));
    assert_eq!(cell.alignment, None);
    assert_eq!(cell.col_span, 1);
    assert_eq!(cell.row_span, 1);
}

#[test]
fn celldata_alignment() {
    let cell = CellData::new(5).alignment(HorizontalAlignment::Right);
    assert_eq!(cell.alignment, Some(HorizontalAlignment::Right));
    assert!(matches!(cell.value, DV::Int(5)));
}

#[test]
fn celldata_alignment_override() {
    let cell = CellData::new("x").alignment(HorizontalAlignment::Center);
    assert_eq!(cell.alignment, Some(HorizontalAlignment::Center));
}

#[test]
fn celldata_span_fields_mutable() {
    let mut cell = CellData::new("v");
    cell.col_span = 3;
    cell.row_span = 2;
    assert_eq!(cell.col_span, 3);
    assert_eq!(cell.row_span, 2);
}

#[test]
fn celldata_clone() {
    let cell = CellData::new(1.5);
    let c = cell.clone();
    assert!(matches!(c.value, DV::Float(_)));
}

// ===========================================================================
// RowData
// ===========================================================================

#[test]
fn rowdata_new() {
    let row = RowData::new(vec![CellData::new("a"), CellData::new("b")]);
    assert_eq!(row.cells.len(), 2);
    assert_eq!(row.height, None);
}

#[test]
fn rowdata_empty() {
    let row = RowData::new(Vec::new());
    assert!(row.cells.is_empty());
}

#[test]
fn rowdata_height_field() {
    let mut row = RowData::new(Vec::new());
    row.height = Some(240);
    assert_eq!(row.height, Some(240));
}

#[test]
fn rowdata_clone() {
    let row = RowData::new(vec![CellData::new(1)]);
    let c = row.clone();
    assert_eq!(c.cells.len(), 1);
}

// ===========================================================================
// TableData
// ===========================================================================

#[test]
fn tabledata_default() {
    let t = TableData {
        headers: None,
        rows: Vec::new(),
    };
    assert!(t.headers.is_none());
    assert!(t.rows.is_empty());
}

#[test]
fn tabledata_with_headers() {
    let t = TableData {
        headers: Some(vec!["A".into(), "B".into()]),
        rows: vec![vec!["1".into(), "2".into()]],
    };
    assert_eq!(t.headers.as_ref().unwrap().len(), 2);
    assert_eq!(t.rows.len(), 1);
}

#[test]
fn tabledata_multiple_rows() {
    let t = TableData {
        headers: None,
        rows: vec![vec!["1".into()], vec!["2".into()], vec!["3".into()]],
    };
    assert_eq!(t.rows.len(), 3);
}

#[test]
fn tabledata_clone() {
    let t = TableData {
        headers: Some(vec!["H".into()]),
        rows: vec![vec!["r".into()]],
    };
    let c = t.clone();
    assert_eq!(c.rows, t.rows);
    assert_eq!(c.headers, t.headers);
}

// ===========================================================================
// 组合
// ===========================================================================

#[test]
fn cell_row_table_composition() {
    let cells = vec![CellData::new("x"), CellData::new(42)];
    let row = RowData::new(cells);
    let table = TableData {
        headers: None,
        rows: vec![row.cells.iter().map(|c| format!("{:?}", c.value)).collect()],
    };
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.rows[0].len(), 2);
}

// ===========================================================================
// DocValue 日期时间转换
// ===========================================================================

#[test]
fn docvalue_from_datetime_utc() {
    let dt = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    assert!(matches!(DV::from(dt), DV::DateTime(_)));
}

#[test]
fn docvalue_from_naive_date() {
    let d = chrono::NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();
    assert!(matches!(DV::from(d), DV::Date(_)));
}

#[test]
fn docvalue_from_naive_datetime() {
    let dt = chrono::NaiveDateTime::new(
        chrono::NaiveDate::from_ymd_opt(2026, 8, 21).unwrap(),
        chrono::NaiveTime::from_hms_opt(12, 30, 0).unwrap(),
    );
    assert!(matches!(DV::from(dt), DV::NaiveDateTime(_)));
}

#[test]
fn docvalue_datetime_debug() {
    let dt = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    let v = DV::from(dt);
    let dbg = format!("{v:?}");
    assert!(dbg.starts_with("DateTime("), "dbg: {dbg}");
}

#[test]
fn docvalue_date_debug() {
    let d = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let v = DV::from(d);
    let dbg = format!("{v:?}");
    assert!(dbg.starts_with("Date("), "dbg: {dbg}");
}

#[test]
fn docvalue_datetime_roundtrip() {
    let dt = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    let v = DV::from(dt);
    match &v {
        DV::DateTime(d) => {
            // 时间戳应一致
            assert_eq!(d.timestamp(), dt.timestamp());
        }
        _ => panic!("expected DateTime"),
    }
}
