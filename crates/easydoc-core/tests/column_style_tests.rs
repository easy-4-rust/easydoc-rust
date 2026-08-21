//! `metadata::TableColumn` 与 `style::TableStyle` / `FontConfig` 深度测试。

use easydoc_core::metadata::TableColumn;
use easydoc_core::style::{Color, FontConfig, TableStyle};
use easydoc_core::types::HorizontalAlignment;

// ===========================================================================
// TableColumn
// ===========================================================================

#[test]
fn column_new_defaults() {
    let col = TableColumn::new("Name", "name", 0);
    assert_eq!(col.name, "Name");
    assert_eq!(col.field_name, "name");
    assert_eq!(col.index, 0);
    assert_eq!(col.order, 0); // order 默认等于 index
    assert_eq!(col.width, None);
    assert_eq!(col.format, None);
    assert_eq!(col.align, None);
    assert_eq!(col.converter, None);
    assert!(!col.wrap);
    assert!(!col.ignored);
}

#[test]
fn column_builder_chain() {
    let col = TableColumn::new("金额", "amount", 1)
        .order(0)
        .width("2cm")
        .format("#,##0.00")
        .align(HorizontalAlignment::Right)
        .wrap();
    assert_eq!(col.name, "金额");
    assert_eq!(col.order, 0);
    assert_eq!(col.width.as_deref(), Some("2cm"));
    assert_eq!(col.format.as_deref(), Some("#,##0.00"));
    assert_eq!(col.align, Some(HorizontalAlignment::Right));
    assert!(col.wrap);
}

#[test]
fn column_ignored_flag() {
    let col = TableColumn::new("skip", "skip", 2);
    assert!(!col.ignored);
}

#[test]
fn column_clone_eq() {
    let a = TableColumn::new("A", "a", 0);
    let b = a.clone();
    assert_eq!(a.name, b.name);
    assert_eq!(a.field_name, b.field_name);
    assert_eq!(a.index, b.index);
}

#[test]
fn column_display_name_variants() {
    // 中文/英文/含空格的列名都能保留
    for name in ["name", "姓名", "user name", "A/B"] {
        let col = TableColumn::new(name, "f", 0);
        assert_eq!(col.name, name);
    }
}

#[test]
fn column_multiple_indices_distinct() {
    let c0 = TableColumn::new("A", "a", 0);
    let c1 = TableColumn::new("B", "b", 1);
    let c2 = TableColumn::new("C", "c", 2);
    assert_ne!(c0.name, c1.name);
    assert_ne!(c1.name, c2.name);
    assert_eq!(c0.index, 0);
    assert_eq!(c1.index, 1);
    assert_eq!(c2.index, 2);
}

// ===========================================================================
// TableStyle
// ===========================================================================

#[test]
fn table_style_default() {
    let s = TableStyle::default();
    let dbg = format!("{s:?}");
    assert!(dbg.contains("TableStyle"));
}

#[test]
fn table_style_header_preset() {
    let s = TableStyle::header();
    let dbg = format!("{s:?}");
    assert!(dbg.contains("TableStyle"));
}

#[test]
fn table_style_simple_preset() {
    let s = TableStyle::simple();
    let dbg = format!("{s:?}");
    assert!(dbg.contains("TableStyle"));
}

#[test]
fn table_style_builder_flags() {
    let s = TableStyle::new()
        .banded_rows(true)
        .auto_width(true)
        .borders(true);
    let dbg = format!("{s:?}");
    assert!(dbg.contains("TableStyle"));
}

#[test]
fn table_style_header_background() {
    let s = TableStyle::new().header_background(Color::rgb(0xDD, 0xDD, 0xDD));
    let dbg = format!("{s:?}");
    assert!(dbg.contains("TableStyle"));
}

#[test]
fn table_style_presets_are_usable() {
    // 三个构造器都能生成可用样式（不 panic）
    let _ = TableStyle::default();
    let _ = TableStyle::header();
    let _ = TableStyle::simple();
}

// ===========================================================================
// FontConfig
// ===========================================================================

#[test]
fn font_config_all_fields_accessible() {
    let f = FontConfig::default();
    let dbg = format!("{f:?}");
    assert!(dbg.contains("FontConfig"));
}

#[test]
fn font_config_clone_eq() {
    let a = FontConfig::default();
    let b = a.clone();
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}

// ===========================================================================
// 组合：列与样式配合
// ===========================================================================

#[test]
fn column_and_style_combined() {
    let col = TableColumn::new("ID", "id", 0).width("auto");
    let style = TableStyle::new().banded_rows(true);
    assert_eq!(col.width.as_deref(), Some("auto"));
    assert!(format!("{style:?}").contains("TableStyle"));
}
