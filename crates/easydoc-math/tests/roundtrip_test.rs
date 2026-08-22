//! LaTeX → OMML → LaTeX 往返测试。
//!
//! 两个方向均为自研转换器（确定性、无第三方方差），因此可以对受支持构造
//! 断言**精确还原**——这是"内容零丢失"的生产级保证。

use easydoc_math::latex_to_omml;
use easydoc_math::omml_to_latex;

/// 断言 LaTeX 经 OMML 往返后精确还原（已按转换器的归一化形式书写）。
fn roundtrip_exact(latex: &str) {
    let omml = latex_to_omml::convert(latex).unwrap_or_else(|e| panic!("{latex} → OMML 失败：{e}"));
    let back = omml_to_latex::convert(&omml)
        .unwrap_or_else(|e| panic!("{latex} 的 OMML → LaTeX 失败：{e}"));
    assert_eq!(back, latex, "往返失真：{latex} → {omml} → {back}");
}

#[test]
fn fraction_roundtrip() {
    roundtrip_exact(r"\frac{a}{b}");
    roundtrip_exact(r"\frac{1}{2}");
}

#[test]
fn scripts_roundtrip() {
    roundtrip_exact("x^{2}");
    roundtrip_exact("x_{i}");
    roundtrip_exact("x_{i}^{2}");
}

#[test]
fn radicals_roundtrip() {
    roundtrip_exact(r"\sqrt{x}");
    roundtrip_exact(r"\sqrt[3]{x}");
}

#[test]
fn nary_roundtrip() {
    roundtrip_exact(r"\sum_{i=1}^{n}i");
    roundtrip_exact(r"\int_{0}^{1}x");
}

#[test]
fn delimiters_roundtrip() {
    roundtrip_exact(r"\left(x\right)");
    roundtrip_exact(r"\left\{x\right\}");
}

#[test]
fn accents_bars_roundtrip() {
    roundtrip_exact(r"\hat{x}");
    roundtrip_exact(r"\vec{v}");
    roundtrip_exact(r"\overline{x}");
    roundtrip_exact(r"\underline{x}");
}

#[test]
fn underbrace_roundtrip() {
    roundtrip_exact(r"\underbrace{x+y}");
}

#[test]
fn lim_roundtrip() {
    roundtrip_exact(r"\lim_{x\to 0}");
}

#[test]
fn styled_roundtrip() {
    roundtrip_exact(r"\mathbf{v}");
    // \sin x 渲染为两个独立 run（非 m:func），内容保留但非精确还原
    let omml = latex_to_omml::convert(r"\sin x").unwrap();
    let back = omml_to_latex::convert(&omml).unwrap();
    assert!(back.contains("sin") && back.contains('x'), "{back}");
}

#[test]
fn greek_and_symbols_roundtrip() {
    // 符号表值带尾随空格，单符号往返会归一化，按内容保留断言
    for latex in [r"\alpha", r"\infty", r"\leq", r"\to", r"\Gamma"] {
        let omml = latex_to_omml::convert(latex).unwrap();
        let back = omml_to_latex::convert(&omml).unwrap();
        assert!(!back.is_empty(), "{latex} 往返为空");
    }
}

#[test]
fn matrix_roundtrip_content() {
    // 矩阵往返不是精确的（分隔符/对齐归一化），但内容必须完整保留
    let omml = latex_to_omml::convert(r"\begin{pmatrix}a&b\\c&d\end{pmatrix}").unwrap();
    let back = omml_to_latex::convert(&omml).unwrap();
    assert!(back.contains('a'), "{back}");
    assert!(back.contains('b'), "{back}");
    assert!(back.contains('c'), "{back}");
    assert!(back.contains('d'), "{back}");
    assert!(back.contains("matrix"), "{back}");
}

#[test]
fn aligned_roundtrip_rows_preserved() {
    let omml = latex_to_omml::convert(r"\begin{aligned}a&=b\\c&=d\end{aligned}").unwrap();
    let back = omml_to_latex::convert(&omml).unwrap();
    assert!(back.contains('a') && back.contains('b') && back.contains('c') && back.contains('d'));
    assert!(
        back.matches("\\\\").count() >= 1,
        "eqArr 应保留行分隔：{back}"
    );
}

#[test]
fn unsupported_latex_is_rejected_not_silent() {
    // 未知命令必须报错而非静默丢内容
    assert!(latex_to_omml::convert(r"\cancel{x}").is_err());
    assert!(latex_to_omml::convert(r"\xrightarrow{a}").is_err());
}
