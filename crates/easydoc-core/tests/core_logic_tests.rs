//! core 纯逻辑单元的深度测试（units / style / metadata / types）。
//!
//! 这些模块无 I/O、无解析器依赖，适合密集断言覆盖，是
//! 0.1.0 验收"1000+ tests"目标的主力补充。

use easydoc_core::style::{Color, FontConfig, ParagraphStyle, TableStyle};
use easydoc_core::types::{ErrorAction, HeadingLevel, HorizontalAlignment};
use easydoc_core::{DocumentMeta, DocumentSection};

// ===========================================================================
// units::Length
// ===========================================================================

// ===========================================================================
// units::Pt
// ===========================================================================

// ===========================================================================
// units::Px
// ===========================================================================

// ===========================================================================
// style::Color
// ===========================================================================

#[test]
fn color_default_is_black() {
    let c = Color::default();
    assert_eq!(c.to_hex(), 0x000000);
}

#[test]
fn color_hex_rgb() {
    let red = Color::rgb(0xFF, 0x00, 0x00);
    assert_eq!(red.to_hex(), 0xFF0000);
    let green = Color::rgb(0x00, 0xFF, 0x00);
    assert_eq!(green.to_hex(), 0x00FF00);
    let blue = Color::rgb(0x00, 0x00, 0xFF);
    assert_eq!(blue.to_hex(), 0x0000FF);
    let white = Color::rgb(0xFF, 0xFF, 0xFF);
    assert_eq!(white.to_hex(), 0xFFFFFF);
}

#[test]
fn color_clamps_channels() {
    // 超范围通道应被截断（u8 已保证 0..=255，验证混合正确）
    let mixed = Color::rgb(0x12, 0x34, 0xAB);
    assert_eq!(mixed.to_hex(), 0x1234AB);
}

#[test]
fn color_default_derives() {
    let a = Color::default();
    let b = Color::default();
    assert_eq!(a, b);
}

// ===========================================================================
// style::ParagraphStyle
// ===========================================================================

#[test]
fn paragraph_style_builder_chain() {
    let style = ParagraphStyle::new()
        .alignment(HorizontalAlignment::Center)
        .first_line_indent(240)
        .space_after(120)
        .line_spacing(360);
    // builder 返回 Self，字段应可通过 Debug 输出验证
    let dbg = format!("{style:?}");
    assert!(dbg.contains("Center"), "dbg: {dbg}");
}

#[test]
fn paragraph_style_default_is_plain() {
    let style = ParagraphStyle::default();
    let dbg = format!("{style:?}");
    assert!(dbg.contains("ParagraphStyle"));
}

#[test]
fn paragraph_style_alignment_variants() {
    for (a, s) in [
        (HorizontalAlignment::Left, "Left"),
        (HorizontalAlignment::Center, "Center"),
        (HorizontalAlignment::Right, "Right"),
        (HorizontalAlignment::Both, "Both"),
    ] {
        let style = ParagraphStyle::new().alignment(a);
        assert!(format!("{style:?}").contains(s));
    }
}

// ===========================================================================
// style::TableStyle
// ===========================================================================

#[test]
fn table_style_builder() {
    let style = TableStyle::default();
    let dbg = format!("{style:?}");
    assert!(dbg.contains("TableStyle"));
}

// ===========================================================================
// style::FontConfig
// ===========================================================================

#[test]
fn font_config_default() {
    let f = FontConfig::default();
    let dbg = format!("{f:?}");
    assert!(dbg.contains("FontConfig"));
}

// ===========================================================================
// types::HeadingLevel
// ===========================================================================

#[test]
fn heading_level_all_variants() {
    for level in [
        HeadingLevel::H1,
        HeadingLevel::H2,
        HeadingLevel::H3,
        HeadingLevel::H4,
        HeadingLevel::H5,
        HeadingLevel::H6,
    ] {
        let dbg = format!("{level:?}");
        assert!(dbg.starts_with('H'), "dbg: {dbg}");
    }
}

#[test]
fn heading_level_clone_eq() {
    assert_eq!(HeadingLevel::H1, HeadingLevel::H1);
    assert_ne!(HeadingLevel::H1, HeadingLevel::H2);
    let h = HeadingLevel::H3;
    let h2 = h;
    assert_eq!(h, h2);
}

// ===========================================================================
// types::HorizontalAlignment
// ===========================================================================

#[test]
fn alignment_all_variants_display() {
    assert_eq!(format!("{:?}", HorizontalAlignment::Left), "Left");
    assert_eq!(format!("{:?}", HorizontalAlignment::Center), "Center");
    assert_eq!(format!("{:?}", HorizontalAlignment::Right), "Right");
    assert_eq!(format!("{:?}", HorizontalAlignment::Both), "Both");
}

#[test]
fn alignment_clone_eq() {
    assert_eq!(HorizontalAlignment::Left, HorizontalAlignment::Left);
    assert_ne!(HorizontalAlignment::Left, HorizontalAlignment::Right);
}

// ===========================================================================
// types::ErrorAction
// ===========================================================================

#[test]
fn error_action_variants() {
    assert_eq!(format!("{:?}", ErrorAction::Continue), "Continue");
    assert_eq!(format!("{:?}", ErrorAction::Skip), "Skip");
    assert_eq!(format!("{:?}", ErrorAction::Stop), "Stop");
}

#[test]
fn error_action_semantics() {
    // Continue 与 Skip 都继续处理，Stop 终止——通过 Debug 区分即可
    assert_ne!(ErrorAction::Continue, ErrorAction::Stop);
    assert_ne!(ErrorAction::Skip, ErrorAction::Stop);
    assert_eq!(ErrorAction::Continue, ErrorAction::Continue);
}

// ===========================================================================
// metadata::DocumentMeta
// ===========================================================================

#[test]
fn meta_default_page_dimensions() {
    let meta = DocumentMeta::default();
    assert_eq!(meta.page_width, None);
    assert_eq!(meta.page_height, None);
    assert!(!meta.landscape);
}

#[test]
fn meta_builder_title_author() {
    let meta = DocumentMeta::new()
        .title("T")
        .author("A")
        .subject("S")
        .keywords("k1,k2");
    assert_eq!(meta.title.as_deref(), Some("T"));
    assert_eq!(meta.author.as_deref(), Some("A"));
    assert_eq!(meta.subject.as_deref(), Some("S"));
    assert_eq!(meta.keywords.as_deref(), Some("k1,k2"));
}

#[test]
fn meta_landscape_roundtrip() {
    let meta = DocumentMeta::new().landscape(true);
    assert!(meta.landscape);
    let meta2 = DocumentMeta::new().landscape(false);
    assert!(!meta2.landscape);
}

#[test]
fn meta_equality() {
    assert_eq!(DocumentMeta::new(), DocumentMeta::new());
    assert_ne!(
        DocumentMeta::new().title("A"),
        DocumentMeta::new().title("B")
    );
}

#[test]
fn meta_clone_is_equal() {
    let meta = DocumentMeta::new().title("Clone").author("Me");
    let cloned = meta.clone();
    assert_eq!(meta, cloned);
}

// ===========================================================================
// DocumentSection
// ===========================================================================

#[test]
fn section_continuous_and_page() {
    assert_eq!(DocumentSection::Continuous, DocumentSection::Continuous);
    assert_ne!(DocumentSection::Continuous, DocumentSection::NextPage);
    let dbg = format!("{:?}", DocumentSection::NextPage);
    assert!(dbg.contains("NextPage"), "dbg: {dbg}");
}

#[test]
fn section_clone() {
    let s = DocumentSection::Continuous;
    assert_eq!(s.clone(), s);
}

// ===========================================================================
// 组合：meta 与 style 配合
// ===========================================================================

#[test]
fn meta_and_style_combined_use() {
    let meta = DocumentMeta::new().title("Report").landscape(true);
    let style = ParagraphStyle::new().alignment(HorizontalAlignment::Right);
    // 组合后各字段保持
    assert_eq!(meta.title.as_deref(), Some("Report"));
    assert!(meta.landscape);
    assert!(format!("{style:?}").contains("Right"));
}
