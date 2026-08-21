//! writer `纯逻辑单元的深度测试（parse_width` / `auto_width` / `banded_rows`）。

use docx_rs::WidthType;
use easydoc_writer::util::parse_width;
use easydoc_writer::{AutoWidthStrategy, BandedRowsStrategy};

// ===========================================================================
// parse_width
// ===========================================================================

#[test]
fn parse_width_cm() {
    let w = parse_width("2cm").expect("2cm");
    assert_eq!(w.value, 1134); // 2 * 567 twips
    assert_eq!(w.width_type, WidthType::Dxa);
}

#[test]
fn parse_width_px() {
    let w = parse_width("80px").expect("80px");
    // 1 px = 15 twips at 96 DPI
    assert_eq!(w.value, 1200);
}

#[test]
fn parse_width_percent() {
    let w = parse_width("30%").expect("30%");
    // 1% = 50 pct units
    assert_eq!(w.value, 1500);
}

#[test]
fn parse_width_percent_fraction() {
    let w = parse_width("12.5%").expect("12.5%");
    assert_eq!(w.value, 625); // 12.5 * 50
}

#[test]
fn parse_width_auto() {
    let w = parse_width("auto").expect("auto");
    assert_eq!(w.value, 0);
    assert_eq!(w.width_type, docx_rs::WidthType::Auto);
}

#[test]
fn parse_width_auto_uppercase() {
    assert!(parse_width("AUTO").is_some());
    assert!(parse_width("Auto").is_some());
}

#[test]
fn parse_width_invalid_inputs() {
    assert!(parse_width("").is_none());
    assert!(parse_width("  ").is_none());
    assert!(parse_width("abc").is_none());
    assert!(parse_width("%").is_none()); // 无数值的百分比
    assert!(parse_width("12cmx").is_none()); // 未知单位后缀
}

#[test]
fn parse_width_bare_number_is_twips() {
    // 裸数字按 twips 处理
    let w = parse_width("1000").expect("bare number");
    assert_eq!(w.value, 1000);
    assert_eq!(w.width_type, docx_rs::WidthType::Dxa);
}

#[test]
fn parse_width_trims_whitespace() {
    let w = parse_width("  2cm  ").expect("trimmed");
    assert_eq!(w.value, 1134);
}

#[test]
fn parse_width_large_value() {
    let w = parse_width("100cm").expect("100cm");
    assert_eq!(w.value, 56_700);
}

// ===========================================================================
// AutoWidthStrategy
// ===========================================================================

#[test]
fn auto_width_defaults() {
    let s = AutoWidthStrategy::new();
    assert_eq!(s.min_width, 240);
    assert_eq!(s.max_width, 9600);
}

#[test]
fn auto_width_small_content_uses_min() {
    let s = AutoWidthStrategy::new();
    assert_eq!(s.calculate_width(0), 240);
    assert_eq!(s.calculate_width(1), 240);
}

#[test]
fn auto_width_linear_growth() {
    let s = AutoWidthStrategy::new();
    // 每字符 240 twips，clamp 到 [240, 9600]
    assert_eq!(s.calculate_width(2), 480);
    assert_eq!(s.calculate_width(5), 1200);
    assert_eq!(s.calculate_width(10), 2400);
    assert_eq!(s.calculate_width(40), 9600);
}

#[test]
fn auto_width_large_content_clamps_max() {
    let s = AutoWidthStrategy::new();
    assert_eq!(s.calculate_width(41), 9600);
    assert_eq!(s.calculate_width(1000), 9600);
    assert_eq!(s.calculate_width(usize::MAX), 9600);
}

#[test]
fn auto_width_custom_bounds() {
    let s = AutoWidthStrategy {
        min_width: 100,
        max_width: 500,
    };
    assert_eq!(s.calculate_width(0), 100);
    // 3 字符 → 720 twips，clamp 到 max 500
    assert_eq!(s.calculate_width(3), 500);
    assert_eq!(s.calculate_width(100), 500);
}

#[test]
fn auto_width_debug() {
    let s = AutoWidthStrategy::new();
    let dbg = format!("{s:?}");
    assert!(dbg.contains("min_width"));
    assert!(dbg.contains("max_width"));
}

// ===========================================================================
// BandedRowsStrategy
// ===========================================================================

#[test]
fn banded_rows_basic_alternation() {
    let s = BandedRowsStrategy::new();
    // 偶数行索引返回条纹色，奇数行返回 None（或反之，取决于实现）
    let row0 = s.color_for_row(0);
    let row1 = s.color_for_row(1);
    let row2 = s.color_for_row(2);
    assert_eq!(row0, row2, "even rows should share the same band color");
    assert_ne!(row0, row1, "adjacent rows should alternate");
}

#[test]
fn banded_rows_all_rows_alternate() {
    let s = BandedRowsStrategy::new();
    let mut prev: Option<Option<easydoc_core::style::Color>> = None;
    for i in 0..10 {
        let current = s.color_for_row(i);
        if let Some(p) = prev {
            assert_ne!(p, current, "row {i} should differ from previous");
        }
        prev = Some(current);
    }
}

#[test]
fn banded_rows_debug() {
    let s = BandedRowsStrategy::new();
    let dbg = format!("{s:?}");
    assert!(dbg.contains("BandedRows"));
}
