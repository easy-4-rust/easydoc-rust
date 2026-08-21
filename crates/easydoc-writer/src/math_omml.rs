//! 数学公式 OMML 注入。
//!
//! docx-rs 不支持 Office Math（OMML）；本模块在 document.xml 后处理阶段
//! 把渲染时生成的占位标记替换为原生 `<m:oMath>` 元素。
//! LaTeX → OMML 转换使用 [`tex2word_math`]。
//!
//! 对应 Java: 无直接对应（Apache POI 的 `XWPFOMath` 系列）

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
/// 替换为包含 `<m:oMath>` 的段落。无法转换的公式保留 LaTeX 文本
/// （`$$...$$` 形式），保证内容不丢失。
///
/// # 错误
///
/// 无（tex2word-math 对未知结构返回空字符串时回退为 LaTeX 文本）。
#[must_use]
pub fn postprocess_math_xml(document_xml: &str, math: &[(String, String, bool)]) -> String {
    let mut xml = document_xml.to_owned();
    for (marker, latex, display) in math {
        let omml = tex2word_math::to_omath(latex);
        let replacement = if omml.trim_start().starts_with("<m:oMath") {
            build_omath_paragraph(&omml, *display)
        } else {
            // 转换失败：回退为 LaTeX 文本段落，内容不丢失
            let latex_text = if *display {
                format!("$${latex}$$")
            } else {
                format!("${latex}$")
            };
            format!(
                "<w:p><w:r><w:rPr><w:rFonts w:ascii=\"Courier New\"/></w:rPr><w:t xml:space=\"preserve\">{latex_text}</w:t></w:r></w:p>"
            )
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

/// 构建包含 `<m:oMath>` 的段落 XML。
fn build_omath_paragraph(omml: &str, display: bool) -> String {
    let _ = display;
    format!("<w:p>{omml}</w:p>")
}
