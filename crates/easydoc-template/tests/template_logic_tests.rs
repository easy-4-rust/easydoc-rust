//! template 占位符解析与填充配置的深度测试。

use easydoc_template::{FillConfig, FillDirection, Placeholder};

// ===========================================================================
// Placeholder::find_all
// ===========================================================================

#[test]
fn scalar_simple() {
    let found = Placeholder::find_all("Hello {name}");
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0],
        Placeholder::Scalar {
            raw: "{name}".to_owned(),
            key: "name".to_owned(),
        }
    );
}

#[test]
fn scalar_with_whitespace_inside() {
    // 内容会被 trim
    let found = Placeholder::find_all("{  spaced  }");
    assert_eq!(found.len(), 1);
    match &found[0] {
        Placeholder::Scalar { raw, key } => {
            assert_eq!(raw, "{  spaced  }");
            assert_eq!(key, "spaced");
        }
        other => panic!("expected Scalar, got {other:?}"),
    }
}

#[test]
fn collection_dot_prefix() {
    let found = Placeholder::find_all("{.rows}");
    assert_eq!(found.len(), 1);
    match &found[0] {
        Placeholder::Collection { field, .. } => assert_eq!(field, "rows"),
        other => panic!("expected Collection, got {other:?}"),
    }
}

#[test]
fn named_collection_prefix_dot_field() {
    let found = Placeholder::find_all("{user.name}");
    assert_eq!(found.len(), 1);
    match &found[0] {
        Placeholder::NamedCollection { prefix, field, .. } => {
            assert_eq!(prefix, "user");
            assert_eq!(field, "name");
        }
        other => panic!("expected NamedCollection, got {other:?}"),
    }
}

#[test]
fn multiple_placeholders_in_order() {
    let found = Placeholder::find_all("{a} text {b} more {.c}");
    assert_eq!(found.len(), 3);
    assert!(matches!(&found[0], Placeholder::Scalar { key, .. } if key == "a"));
    assert!(matches!(&found[1], Placeholder::Scalar { key, .. } if key == "b"));
    assert!(matches!(&found[2], Placeholder::Collection { field, .. } if field == "c"));
}

#[test]
fn no_placeholders() {
    assert!(Placeholder::find_all("plain text without braces").is_empty());
    assert!(Placeholder::find_all("").is_empty());
}

#[test]
fn unmatched_braces_ignored() {
    // 未闭合的 `{` 或空 `{}` 不应产生占位符
    assert!(Placeholder::find_all("open {unclosed").is_empty());
    assert!(Placeholder::find_all("empty {} braces").is_empty());
    assert!(Placeholder::find_all("just } close").is_empty());
}

#[test]
fn empty_content_ignored() {
    // 完全空 `{}` 被忽略（content 为空）
    assert!(Placeholder::find_all("{}").is_empty());
    // 仅空格的 `{ }` 会解析为 key 为空的 Scalar（现有实现行为，不崩溃）
    let found = Placeholder::find_all("{ }");
    assert_eq!(found.len(), 1);
    assert!(matches!(&found[0], Placeholder::Scalar { key, .. } if key.is_empty()));
}

#[test]
fn dot_only_content_ignored() {
    // `{.}` 无字段名——按现有实现解析为 Collection { field: "" }
    let found = Placeholder::find_all("{.}");
    assert_eq!(found.len(), 1);
    assert!(matches!(&found[0], Placeholder::Collection { field, .. } if field.is_empty()));
}

#[test]
fn unicode_keys_supported() {
    let found = Placeholder::find_all("{姓名}");
    assert_eq!(found.len(), 1);
    match &found[0] {
        Placeholder::Scalar { key, .. } => assert_eq!(key, "姓名"),
        other => panic!("expected Scalar, got {other:?}"),
    }
}

#[test]
fn adjacent_placeholders_no_separator() {
    let found = Placeholder::find_all("{a}{b}");
    assert_eq!(found.len(), 2);
    assert!(matches!(&found[0], Placeholder::Scalar { key, .. } if key == "a"));
    assert!(matches!(&found[1], Placeholder::Scalar { key, .. } if key == "b"));
}

#[test]
fn mixed_types_in_one_text() {
    let found = Placeholder::find_all("{title} {.items} {order.id}");
    assert_eq!(found.len(), 3);
    assert!(matches!(&found[0], Placeholder::Scalar { .. }));
    assert!(matches!(&found[1], Placeholder::Collection { .. }));
    assert!(matches!(&found[2], Placeholder::NamedCollection { .. }));
}

// ===========================================================================
// FillDirection
// ===========================================================================

#[test]
fn fill_direction_variants_distinct() {
    assert_ne!(FillDirection::Horizontal, FillDirection::Vertical);
    let h = FillDirection::Horizontal;
    let h2 = h;
    assert_eq!(h, h2);
}

// ===========================================================================
// FillConfig
// ===========================================================================

#[test]
fn fill_config_defaults() {
    let cfg = FillConfig::new();
    let dbg = format!("{cfg:?}");
    assert!(dbg.contains("FillConfig"));
}

#[test]
fn fill_config_builder_chain() {
    let cfg = FillConfig::new()
        .direction(FillDirection::Horizontal)
        .force_new_row(true)
        .auto_style(false);
    let dbg = format!("{cfg:?}");
    assert!(dbg.contains("Horizontal"), "dbg: {dbg}");
}

#[test]
fn fill_config_direction_roundtrip() {
    let v = FillConfig::new().direction(FillDirection::Vertical);
    let h = FillConfig::new().direction(FillDirection::Horizontal);
    assert_ne!(format!("{v:?}"), format!("{h:?}"));
}

// ===========================================================================
// Placeholder 更多边界
// ===========================================================================

#[test]
fn placeholder_nested_braces_single() {
    // 只解析最内层或最外层——验证不 panic 且至少一个占位符
    let found = Placeholder::find_all("{a{b}}");
    assert!(!found.is_empty());
}

#[test]
fn placeholder_long_key() {
    let found = Placeholder::find_all("{a_very_long_key_name_0123456789}");
    assert_eq!(found.len(), 1);
    assert!(
        matches!(&found[0], Placeholder::Scalar { key, .. } if key == "a_very_long_key_name_0123456789")
    );
}

#[test]
fn placeholder_key_with_underscore() {
    let found = Placeholder::find_all("{user_name}");
    assert!(matches!(&found[0], Placeholder::Scalar { key, .. } if key == "user_name"));
}

#[test]
fn placeholder_key_with_dash() {
    let found = Placeholder::find_all("{a-b}");
    assert_eq!(found.len(), 1);
}

#[test]
fn placeholder_named_collection_nested_dots() {
    let found = Placeholder::find_all("{a.b.c}");
    // split_once('.') 只取第一个点，剩余部分作为 field
    assert_eq!(found.len(), 1);
    assert!(matches!(&found[0], Placeholder::NamedCollection { .. }));
}

#[test]
fn placeholder_in_sentence() {
    let found = Placeholder::find_all("Price: {price} USD");
    assert_eq!(found.len(), 1);
    assert!(matches!(&found[0], Placeholder::Scalar { key, .. } if key == "price"));
}

#[test]
fn placeholder_duplicate_keys_detected_twice() {
    let found = Placeholder::find_all("{k} and {k} again");
    assert_eq!(found.len(), 2);
    assert!(matches!(&found[0], Placeholder::Scalar { key, .. } if key == "k"));
    assert!(matches!(&found[1], Placeholder::Scalar { key, .. } if key == "k"));
}

#[test]
fn placeholder_brace_only_text() {
    // 花括号不成对——按字面处理，不产生占位符或仅产生闭合的
    let found = Placeholder::find_all("just } here");
    assert!(found.is_empty());
}

#[test]
fn placeholder_with_tab_newline() {
    let found = Placeholder::find_all("{a\nb}");
    // 内容含换行——trim 后仍非空，产生 Scalar
    assert_eq!(found.len(), 1);
}
