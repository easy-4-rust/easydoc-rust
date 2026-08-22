//! 数学公式 OMML 注入。
//!
//! docx-rs 不支持 Office Math（OMML）；本模块在 document.xml 后处理阶段
//! 把渲染时生成的占位标记替换为原生 `<m:oMath>` 元素。
//! LaTeX → OMML 转换使用自研 [`easydoc_math::latex_to_omml`]（严格错误通道：
//! 无法无损转换时返回 `Err`，此处回退保留 `$latex$` 原文，内容零丢失）。
//!
//! 对应 Java: 无直接对应（Apache POI 的 `XWPFOMath` 系列）

use easydoc_math::latex_to_omml;

/// OMML 命名空间，需声明在 document.xml 根元素上，否则 Word 不识别公式。
const OMML_NS: &str = "xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"";

/// 把 document.xml 中的 Math 占位标记替换为原生 `<m:oMath>` 元素。
///
/// # 参数
///
/// - `document_xml`: docx-rs 生成的 document.xml 内容。
/// - `math`: [`super::content_renderer::take_rendered_math`] 返回的
///   `(标记, latex, display)` 列表。
///
/// # 返回
///
/// 替换完成后的 document.xml。每个占位段落（`<w:p>` 内含标记 run）被
/// 替换为包含 `<m:oMath>`（行内）或 `<m:oMathPara>`（块级、居中）的段落；
/// 根元素补充 `xmlns:m` 声明。无法转换的公式保留 LaTeX 文本
/// （`$$...$$` / `$...$` 形式），保证内容不丢失。
///
/// # 错误
///
/// 无（转换失败时回退为 LaTeX 文本，由 [`latex_to_omml::convert`] 的
/// 严格错误通道保证不静默丢内容）。
#[must_use]
pub fn postprocess_math_xml(document_xml: &str, math: &[(String, String, bool)]) -> String {
    let mut xml = ensure_omml_namespace(document_xml);
    for (marker, latex, display) in math {
        let replacement = match latex_to_omml::convert(latex) {
            Ok(omml) if !omml.trim().is_empty() && !is_empty_omath(&omml) => {
                build_omath_paragraph(&omml, *display)
            }
            // 转换失败（不支持的命令/不配对等）或产物为空：回退为 LaTeX 文本段落。
            _ => build_latex_fallback_paragraph(latex, *display),
        };
        // marker 在 docx-rs 生成的 `<w:t>` 文本内；需找到其所属的整个
        // `<w:p>` 段落并整体替换（否则 OMML 会嵌进 `<w:t>` 造成非法 XML）。
        if let Some((para_start, para_end)) = find_containing_paragraph(&xml, marker) {
            xml.replace_range(para_start..para_end, &replacement);
        } else {
            xml = xml.replace(marker, &replacement);
        }
    }
    xml
}

/// 确保 document.xml 根元素声明 `xmlns:m`，缺则补入。
///
/// docx-rs 生成的根元素为 `<w:document …>`；在第一个 `<w:document` 起始标签
/// 的 `>` 之前插入命名空间属性（已存在则跳过）。
fn ensure_omml_namespace(document_xml: &str) -> String {
    if document_xml.contains("xmlns:m=") {
        return document_xml.to_owned();
    }
    let Some(start) = document_xml.find("<w:document") else {
        return document_xml.to_owned();
    };
    let Some(tag_end) = document_xml[start..].find('>') else {
        return document_xml.to_owned();
    };
    let abs_end = start + tag_end;
    let mut xml = String::with_capacity(document_xml.len() + OMML_NS.len() + 1);
    xml.push_str(&document_xml[..abs_end]);
    xml.push(' ');
    xml.push_str(OMML_NS);
    xml.push_str(&document_xml[abs_end..]);
    xml
}

/// 判断产物是否为空的 `<m:oMath></m:oMath>`。
fn is_empty_omath(omml: &str) -> bool {
    let inner = omml
        .trim()
        .strip_prefix("<m:oMath>")
        .and_then(|s| s.strip_suffix("</m:oMath>"))
        .unwrap_or(omml.trim());
    inner.trim().is_empty()
}

/// 构建包含 `<m:oMath>` 的段落 XML。
///
/// 块级公式（display）包 `<m:oMathPara>` 并居中，行内公式直接包 `<m:oMath>`。
fn build_omath_paragraph(omml: &str, display: bool) -> String {
    if display {
        format!(
            "<w:p><m:oMathPara><m:oMathParaPr><m:jc m:val=\"center\"/></m:oMathParaPr>\
             {omml}</m:oMathPara></w:p>"
        )
    } else {
        format!("<w:p>{omml}</w:p>")
    }
}

/// 转换失败的兜底：保留 `$latex$` / `$$latex$$` 原文（等宽字体），内容不丢失。
fn build_latex_fallback_paragraph(latex: &str, display: bool) -> String {
    let latex_text = if display {
        format!("$${latex}$$")
    } else {
        format!("${latex}$")
    };
    format!(
        "<w:p><w:r><w:rPr><w:rFonts w:ascii=\"Courier New\"/></w:rPr>\
         <w:t xml:space=\"preserve\">{latex_text}</w:t></w:r></w:p>"
    )
}

/// 找到包含 `marker` 的 `<w:p>...</w:p>` 段落范围。
///
/// 向前找 `<w:p>`（后跟 `>` 或空格，排除 `<w:pPr` 等前缀标签），
/// 向后找 `</w:p>`。找不到时返回 `None`。
fn find_containing_paragraph(xml: &str, marker: &str) -> Option<(usize, usize)> {
    let marker_pos = xml.find(marker)?;
    // 向前找段落开始：最近的 `<w:p>` 后跟 `>`/空格/`/`
    let before = &xml[..marker_pos];
    let mut para_start = None;
    let mut search = 0;
    while let Some(rel) = before[search..].find("<w:p") {
        let abs = search + rel;
        let after = before[abs + 4..].chars().next();
        if matches!(after, Some('>' | ' ' | '/')) {
            para_start = Some(abs);
        }
        search = abs + 4;
    }
    let para_start = para_start?;
    // 向后找段落结束
    let after_marker = &xml[marker_pos..];
    let close_rel = after_marker.find("</w:p>")?;
    let para_end = marker_pos + close_rel + "</w:p>".len();
    Some((para_start, para_end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_math_injects_omath() {
        let xml = "<w:document><w:body><w:p><w:r><w:t>@@EASYDOC_MATH_0@@</w:t></w:r></w:p></w:body></w:document>";
        let math = vec![("@@EASYDOC_MATH_0@@".to_string(), "x^2".to_string(), false)];
        let out = postprocess_math_xml(xml, &math);
        assert!(out.contains("<m:oMath>"), "{out}");
        assert!(out.contains("<m:sSup>"), "{out}");
        assert!(out.contains("xmlns:m="), "应注入命名空间：{out}");
        assert!(!out.contains("@@EASYDOC_MATH"), "{out}");
    }

    #[test]
    fn display_math_wraps_omath_para() {
        let xml =
            "<w:document><w:body><w:p><w:r><w:t>@@M@@</w:t></w:r></w:p></w:body></w:document>";
        let math = vec![("@@M@@".to_string(), r"\frac{a}{b}".to_string(), true)];
        let out = postprocess_math_xml(xml, &math);
        assert!(out.contains("<m:oMathPara>"), "{out}");
        assert!(out.contains("<m:jc m:val=\"center\"/>"), "{out}");
        assert!(out.contains("<m:oMath>"), "{out}");
    }

    #[test]
    fn unsupported_latex_falls_back_to_source() {
        let xml =
            "<w:document><w:body><w:p><w:r><w:t>@@M@@</w:t></w:r></w:p></w:body></w:document>";
        let math = vec![("@@M@@".to_string(), r"\cancel{x}".to_string(), true)];
        let out = postprocess_math_xml(xml, &math);
        assert!(out.contains(r"$$\cancel{x}$$"), "应保留 LaTeX 原文：{out}");
        assert!(!out.contains("<m:oMath>"), "{out}");
    }

    #[test]
    fn empty_omath_falls_back() {
        let xml =
            "<w:document><w:body><w:p><w:r><w:t>@@M@@</w:t></w:r></w:p></w:body></w:document>";
        let math = vec![("@@M@@".to_string(), " ".to_string(), false)];
        let out = postprocess_math_xml(xml, &math);
        assert!(out.contains("$ $"), "{out}");
    }

    #[test]
    fn namespace_not_duplicated() {
        let xml = "<w:document xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"><w:body></w:body></w:document>";
        let out = ensure_omml_namespace(xml);
        assert_eq!(out.matches("xmlns:m=").count(), 1, "{out}");
    }

    #[test]
    fn namespace_injected_into_root_tag() {
        let xml = "<w:document xmlns:w=\"http://x\"><w:body></w:body></w:document>";
        let out = ensure_omml_namespace(xml);
        assert!(
            out.starts_with("<w:document xmlns:w=\"http://x\" xmlns:m="),
            "{out}"
        );
    }
}
