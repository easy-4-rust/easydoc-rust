//! 属性测试（proptest）：两个转换器对任意输入不 panic，语法生成公式可往返。
//!
//! 对应 roadmap 0.1.0 fuzz 目标：模糊测试新转换器，重点检测 panic/abort。

use proptest::prelude::*;

use easydoc_math::latex_to_omml;
use easydoc_math::omml_to_latex;

/// 语法生成的 LaTeX 表达式（递归，深度受限）。
fn latex_expr() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        Just("x".to_string()),
        Just("y".to_string()),
        Just("1".to_string()),
        Just("2".to_string()),
        Just(r"\alpha".to_string()),
        Just(r"\beta".to_string()),
    ];
    leaf.prop_recursive(3, 32, 3, |inner| {
        prop_oneof![
            inner.clone().prop_map(|s| format!(r"\frac{{{s}}}{{{s}}}")),
            inner.clone().prop_map(|s| format!(r"\sqrt{{{s}}}")),
            inner.clone().prop_map(|s| format!(r"{s}^{{{s}}}")),
            inner.clone().prop_map(|s| format!(r"{s}_{{{s}}}")),
            inner.clone().prop_map(|s| format!(r"\left({s}\right)")),
            inner.clone().prop_map(|s| format!(r"\hat{{{s}}}")),
        ]
    })
}

proptest! {
    /// 任意 UTF-8 字符串喂给 OMML→LaTeX，不得 panic（XML 解析错误返回 Err）。
    #[test]
    fn omml_to_latex_no_panic_on_garbage(s in any::<String>()) {
        let _ = omml_to_latex::convert(&s);
    }

    /// 任意 UTF-8 字符串喂给 LaTeX→OMML，不得 panic（未知命令返回 Err）。
    #[test]
    fn latex_to_omml_no_panic_on_garbage(s in any::<String>()) {
        let _ = latex_to_omml::convert(&s);
    }

    /// 语法生成的 LaTeX 必须转换成功，且 OMML→LaTeX 往返非空（内容不丢）。
    #[test]
    fn latex_roundtrip_succeeds_and_nonempty(latex in latex_expr()) {
        let omml = latex_to_omml::convert(&latex);
        prop_assert!(omml.is_ok(), "{latex} 应转换成功");
        let omml = omml.unwrap();
        let back = omml_to_latex::convert(&omml);
        prop_assert!(back.is_ok(), "{latex} 的 OMML 应能转回 LaTeX");
        prop_assert!(!back.unwrap().is_empty(), "{latex} 往返不应为空");
    }
}
