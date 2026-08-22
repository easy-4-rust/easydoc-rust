//! OMML → LaTeX 集成测试：真实 Word 产出的 OMML 风格样例。
//!
//! 与单元测试的区别：这里使用 Word 公式编辑器实际会写出的**属性形式** Pr
//! （`<m:dPr m:begChr="["/>`）与带命名空间的完整片段，通过公开 API 断言。

use easydoc_math::omml_to_latex;

/// 包裹完整的 `<m:oMath>`（带 Word 实际使用的 m: 命名空间前缀声明方式）。
fn wrap(inner: &str) -> String {
    format!(
        "<m:oMath xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\">\
         {inner}</m:oMath>"
    )
}

#[test]
fn word_style_quadratic_formula() {
    // Word 导出的二次公式：分式 + 根式 + 上下标，fPr 属性形式
    let xml = wrap(
        "<m:f><m:fPr m:type=\"bar\"/>\
         <m:num><m:r><m:t>-b</m:t></m:r><m:sSup><m:e><m:r><m:t>b</m:t></m:r></m:e>\
         <m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup>\
         <m:r><m:t>-4ac</m:t></m:r></m:num>\
         <m:den><m:r><m:t>2a</m:t></m:r></m:den></m:f>",
    );
    let latex = omml_to_latex::convert(&xml).unwrap();
    assert!(latex.contains("\\frac"), "{latex}");
    assert!(latex.contains("b^{2}"), "{latex}");
    assert!(latex.contains("-4ac"), "{latex}");
    assert!(latex.contains("2a"), "{latex}");
}

#[test]
fn word_style_integral_with_limits() {
    // Word 的积分：naryPr 属性形式（chr/limLoc 直接作为属性）
    let xml = wrap(
        "<m:nary><m:naryPr m:chr=\"∫\" m:limLoc=\"subSup\" m:grow=\"1\"/>\
         <m:sub><m:r><m:t>0</m:t></m:r></m:sub>\
         <m:sup><m:r><m:t>1</m:t></m:r></m:sup>\
         <m:e><m:r><m:t>x</m:t></m:r><m:sSup><m:e><m:r><m:t>x</m:t></m:r></m:e>\
         <m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup></m:e></m:nary>",
    );
    let latex = omml_to_latex::convert(&xml).unwrap();
    assert_eq!(latex, r"\int_{0}^{1}xx^{2}", "{latex}");
}

#[test]
fn word_style_matrix_with_brackets() {
    // Word 矩阵：dPr 属性形式包裹 m:m
    let xml = wrap(
        "<m:d><m:dPr m:begChr=\"[\" m:endChr=\"]\"/>\
         <m:e><m:m><m:mr><m:e><m:r><m:t>1</m:t></m:r></m:e>\
         <m:e><m:r><m:t>2</m:t></m:r></m:e></m:mr>\
         <m:mr><m:e><m:r><m:t>3</m:t></m:r></m:e>\
         <m:e><m:r><m:t>4</m:t></m:r></m:e></m:mr></m:m></m:e></m:d>",
    );
    let latex = omml_to_latex::convert(&xml).unwrap();
    assert!(
        latex.contains("\\left[") && latex.contains("\\right]"),
        "{latex}"
    );
    assert!(latex.contains("matrix"), "{latex}");
    assert!(latex.contains('1') && latex.contains('4'), "{latex}");
}

#[test]
fn word_style_accents_and_bars() {
    // Word 重音/上线：accPr/barPr 属性形式
    let xml = wrap(
        "<m:acc><m:accPr m:chr=\"̂\"/><m:e><m:r><m:t>x</m:t></m:r></m:e></m:acc>\
         <m:bar><m:barPr m:pos=\"top\"/><m:e><m:r><m:t>AB</m:t></m:r></m:e></m:bar>",
    );
    let latex = omml_to_latex::convert(&xml).unwrap();
    assert!(latex.contains("\\hat{x}"), "{latex}");
    assert!(latex.contains("\\overline{AB}"), "{latex}");
}

#[test]
fn word_style_lim_low() {
    // Word 的 m:limLow（极限布局）
    let xml = wrap(
        "<m:limLow><m:e><m:r><m:rPr><m:nor/></m:rPr><m:t>lim</m:t></m:r></m:e>\
         <m:lim><m:r><m:t>x</m:t></m:r><m:r><m:t>→</m:t></m:r><m:r><m:t>0</m:t></m:r></m:lim>\
         </m:limLow>",
    );
    let latex = omml_to_latex::convert(&xml).unwrap();
    assert_eq!(latex, r"\lim_{x\to 0}");
}

#[test]
fn word_style_display_paragraph() {
    // oMathPara 包装的块级公式（convert 应跳过包装直取 oMath）
    let xml = format!(
        "<m:oMathPara>{}</m:oMathPara>",
        wrap("<m:r><m:t>x+y</m:t></m:r>")
    );
    let latex = omml_to_latex::convert(&xml).unwrap();
    assert_eq!(latex, "x+y");
}
