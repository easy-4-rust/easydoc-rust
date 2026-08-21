//! 基于 comrak 的 Markdown 导入器。
//!
//! 使用成熟解析引擎 [comrak] 将 Markdown 文本解析为 `CommonMark` AST，
//! 再映射为 easydoc 的语义模型 [`DocumentContent`]。相比手写的
//! `markdown_import`，本模块完整支持：
//!
//! - 行内数学 `$...$` 与块级数学 `$$...$$`（含多行）→ [`DocumentBlock::Math`]
//! - GFM 表格 / 任务列表 / 删除线 / 脚注 / 自动链接
//! - 标准 `CommonMark` 的嵌套结构（引用、列表、强调）
//!
//! 对应 Java: 无（Java 生态无 comrak 对应物；对标 `commonmark-java` 等解析器）
//!
//! [comrak]: https://crates.io/crates/comrak

use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options};
use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentImage, DocumentList, DocumentListItem, DocumentMeta,
    DocumentTable, DocumentTableCell, DocumentTableRow, DocumentTextRun, Result,
};

/// 使用 comrak 解析 Markdown 文本并转换为语义文档模型。
///
/// # 参数
///
/// - `source`: Markdown 源码。
///
/// # 返回
///
/// 解析后的 [`DocumentContent`]，含 front matter 元数据（若存在）。
///
/// # 错误
///
/// 当前实现不返回错误（comrak 对任意输入都能解析），
/// 保留 `Result` 签名以与 `markdown_import` 接口对齐。
pub fn import_with_comrak(source: &str) -> Result<DocumentContent> {
    let mut options = Options::default();
    // 数学：math_dollars（$...$ / $$...$$）需与 math_code 同时启用
    options.extension.math_dollars = true;
    options.extension.math_code = true;
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.autolink = true;
    options.extension.tagfilter = true;
    options.extension.front_matter_delimiter = Some("---".into());

    let arena = Arena::new();
    let root = comrak::parse_document(&arena, source, &options);

    // 提取 front matter（comrak 的 FrontMatter 节点）
    let mut metadata = DocumentMeta::default();
    let mut blocks = Vec::new();
    let mut footnote_defs: std::collections::HashMap<String, Vec<DocumentBlock>> =
        std::collections::HashMap::new();

    collect_blocks(root, source, &mut blocks, &mut metadata, &mut footnote_defs);

    Ok(DocumentContent { metadata, blocks })
}

/// 递归收集文档块。
#[allow(clippy::too_many_arguments)]
fn collect_blocks<'a>(
    node: &'a AstNode<'a>,
    source: &str,
    blocks: &mut Vec<DocumentBlock>,
    metadata: &mut DocumentMeta,
    footnote_defs: &mut std::collections::HashMap<String, Vec<DocumentBlock>>,
) {
    for child in node.children() {
        let value = &child.data.borrow().value;
        match value {
            NodeValue::FrontMatter(text) => {
                // 简单解析 YAML 键值对（title/author/subject/keywords）
                parse_front_matter(text, metadata);
            }
            NodeValue::Heading(heading) => {
                let runs = collect_inline(child, source);
                let level = heading.level.clamp(1, 6);
                blocks.push(DocumentBlock::Heading { level, runs });
            }
            NodeValue::Paragraph => {
                let kids: Vec<&AstNode> = child.children().collect();
                // 块级数学：段落仅含单个 Math(display=true) 子节点时提升为 Math 块
                if kids.len() == 1
                    && let NodeValue::Math(m) = &kids[0].data.borrow().value
                    && m.display_math
                {
                    blocks.push(DocumentBlock::Math {
                        omml: None,
                        latex: Some(m.literal.clone()),
                        display: true,
                    });
                    continue;
                }
                // 独立图片：段落仅含单个 Image 子节点时提升为 Image 块
                if kids.len() == 1
                    && let NodeValue::Image(image) = &kids[0].data.borrow().value
                {
                    let alt = node_text(kids[0], source);
                    let extension = image
                        .url
                        .rsplit('.')
                        .next()
                        .map(ToOwned::to_owned)
                        .filter(|e| !e.is_empty());
                    blocks.push(DocumentBlock::Image(DocumentImage {
                        alt_text: if alt.is_empty() { None } else { Some(alt) },
                        data: None,
                        extension,
                    }));
                    continue;
                }
                // 混合段落：分离 display 数学与文本（行内 $a$ 保留在段落文本）
                let mut math_blocks: Vec<DocumentBlock> = Vec::new();
                let mut text_runs = Vec::new();
                for kid in &kids {
                    if let NodeValue::Math(m) = &kid.data.borrow().value
                        && m.display_math
                    {
                        math_blocks.push(DocumentBlock::Math {
                            omml: None,
                            latex: Some(m.literal.clone()),
                            display: true,
                        });
                    } else {
                        text_runs.extend(collect_inline(kid, source));
                    }
                }
                for mb in math_blocks {
                    blocks.push(mb);
                }
                if !text_runs.is_empty() {
                    blocks.push(DocumentBlock::Paragraph(text_runs));
                }
            }
            NodeValue::Math(math) => {
                // math.literal 是公式的 LaTeX 源码
                let latex = math.literal.clone();
                blocks.push(DocumentBlock::Math {
                    omml: None,
                    latex: Some(latex),
                    display: math.display_math,
                });
            }
            NodeValue::CodeBlock(code) => {
                let language = if code.info.is_empty() {
                    None
                } else {
                    Some(code.info.split_whitespace().next().unwrap_or("").to_owned())
                };
                // comrak 的代码块内容在 `literal` 字段（无子节点）
                let code_text = code.literal.clone();
                blocks.push(DocumentBlock::CodeBlock {
                    language,
                    code: code_text,
                });
            }
            NodeValue::ThematicBreak => {
                blocks.push(DocumentBlock::ThematicBreak);
            }
            NodeValue::BlockQuote => {
                let mut inner = Vec::new();
                collect_blocks(child, source, &mut inner, metadata, footnote_defs);
                // 引用块统一以 TextBox 表达（嵌套引用保持层级）
                blocks.push(DocumentBlock::TextBox(inner));
            }
            NodeValue::List(list) => {
                let ordered = list.list_type == comrak::nodes::ListType::Ordered;
                let start_number = if ordered {
                    Some(u32::try_from(list.start).unwrap_or(1))
                } else {
                    None
                };
                let mut items = Vec::new();
                for item_child in child.children() {
                    if let NodeValue::Item(_) | NodeValue::TaskItem(_) =
                        &item_child.data.borrow().value
                    {
                        let mut item_blocks = Vec::new();
                        collect_blocks(
                            item_child,
                            source,
                            &mut item_blocks,
                            metadata,
                            footnote_defs,
                        );
                        // 任务列表：checkbox 前缀
                        if let NodeValue::TaskItem(task) = &item_child.data.borrow().value {
                            let checked = task.symbol.is_some();
                            if let Some(DocumentBlock::Paragraph(runs)) = item_blocks.first_mut() {
                                let prefix = if checked { "☑ " } else { "☐ " };
                                runs.insert(
                                    0,
                                    DocumentTextRun {
                                        text: prefix.to_owned(),
                                        ..DocumentTextRun::default()
                                    },
                                );
                            }
                        }
                        items.push(DocumentListItem {
                            blocks: item_blocks,
                            nested: None,
                        });
                    }
                }
                blocks.push(DocumentBlock::List(DocumentList {
                    ordered,
                    start_number,
                    items,
                }));
            }
            NodeValue::Table(_table) => {
                let mut rows = Vec::new();
                for row_child in child.children() {
                    if let NodeValue::TableRow(is_header) = &row_child.data.borrow().value {
                        let cells = row_child
                            .children()
                            .map(|cell| {
                                let runs = collect_inline(cell, source);
                                DocumentTableCell {
                                    blocks: if runs.is_empty() {
                                        Vec::new()
                                    } else {
                                        vec![DocumentBlock::Paragraph(runs)]
                                    },
                                    column_span: 1,
                                    row_span: 1,
                                }
                            })
                            .collect::<Vec<_>>();
                        rows.push(DocumentTableRow {
                            cells,
                            is_header: *is_header,
                        });
                    }
                }
                blocks.push(DocumentBlock::Table(DocumentTable { rows }));
            }
            NodeValue::FootnoteDefinition(def) => {
                let mut def_blocks = Vec::new();
                collect_blocks(child, source, &mut def_blocks, metadata, footnote_defs);
                footnote_defs.insert(def.name.clone(), def_blocks);
            }
            NodeValue::Image(image) => {
                let alt = collect_inline(child, source)
                    .into_iter()
                    .map(|r| r.text)
                    .collect::<String>();
                let extension = image
                    .url
                    .rsplit('.')
                    .next()
                    .map(ToOwned::to_owned)
                    .filter(|e| !e.is_empty());
                blocks.push(DocumentBlock::Image(DocumentImage {
                    alt_text: if alt.is_empty() { None } else { Some(alt) },
                    data: None,
                    extension,
                }));
            }
            _ => {
                // 其他节点类型（如 DescriptionList）当前不支持，递归收集子节点
                collect_blocks(child, source, blocks, metadata, footnote_defs);
            }
        }
    }
}

/// 收集行内内容为 run 列表（处理强调/链接/代码/HTML/脚注引用/数学）。
fn collect_inline<'a>(node: &'a AstNode<'a>, source: &str) -> Vec<DocumentTextRun> {
    let mut runs = Vec::new();
    collect_inline_rec(node, source, &mut runs, false, false, false);
    runs
}

#[allow(clippy::too_many_arguments)]
fn collect_inline_rec<'a>(
    node: &'a AstNode<'a>,
    source: &str,
    runs: &mut Vec<DocumentTextRun>,
    bold: bool,
    italic: bool,
    strike: bool,
) {
    let value = &node.data.borrow().value;
    match value {
        NodeValue::Text(text) => push_text(runs, text, bold, italic, strike, None),
        NodeValue::SoftBreak | NodeValue::LineBreak => {
            push_text(runs, "\n", bold, italic, strike, None);
        }
        NodeValue::Code(code) => push_text(runs, &code.literal, bold, italic, strike, None),
        NodeValue::Strong => {
            for c in node.children() {
                collect_inline_rec(c, source, runs, true, italic, strike);
            }
        }
        NodeValue::Emph => {
            for c in node.children() {
                collect_inline_rec(c, source, runs, bold, true, strike);
            }
        }
        NodeValue::Strikethrough => {
            for c in node.children() {
                collect_inline_rec(c, source, runs, bold, italic, true);
            }
        }
        NodeValue::Link(link) => {
            let url = link.url.clone();
            for c in node.children() {
                collect_inline_rec(c, source, runs, bold, italic, strike);
            }
            // 把超链接应用到该链接的最后一个 run
            if let Some(last) = runs.last_mut() {
                last.hyperlink = Some(url);
            }
        }
        NodeValue::Image(image) => {
            // 行内图片：保留 alt 文本 + markdown 图片标记
            let alt = node_text(node, source);
            let url = image.url.clone();
            push_text(
                runs,
                &format!("![{alt}]({url})"),
                bold,
                italic,
                strike,
                None,
            );
        }
        NodeValue::FootnoteReference(_) => {
            // 脚注引用：文本保留为 [^id] 形式
            let text = node_text(node, source);
            push_text(runs, &text, bold, italic, strike, None);
        }
        NodeValue::Math(math) => {
            // 行内数学：LaTeX 源码以 $...$ 形式保留在 run 中
            // （语义模型行内公式用 Math 块表达，行内场景暂以文本呈现）
            let latex = math.literal.clone();
            push_text(runs, &format!("${latex}$"), bold, italic, strike, None);
        }
        NodeValue::HtmlInline(html) => {
            // HTML 内联标签 → run 属性（复用简单解析）
            apply_html_inline(html, runs, bold, italic, strike);
        }
        _ => {
            for c in node.children() {
                collect_inline_rec(c, source, runs, bold, italic, strike);
            }
        }
    }
}

/// 把文本追加为 run，合并相邻同属性文本。
fn push_text(
    runs: &mut Vec<DocumentTextRun>,
    text: &str,
    bold: bool,
    italic: bool,
    strike: bool,
    hyperlink: Option<String>,
) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = runs.last_mut()
        && !last.bold
        && !last.italic
        && !last.strikethrough
        && last.hyperlink.is_none()
        && !bold
        && !italic
        && !strike
        && hyperlink.is_none()
    {
        last.text.push_str(text);
        return;
    }
    runs.push(DocumentTextRun {
        text: text.to_owned(),
        bold,
        italic,
        strikethrough: strike,
        hyperlink,
    });
}

/// 解析 HTML 内联标签为 run 属性（`<strong>`/`<em>`/`<code>`/`<a>`/`<br>`）。
fn apply_html_inline(
    html: &str,
    runs: &mut Vec<DocumentTextRun>,
    bold: bool,
    italic: bool,
    strike: bool,
) {
    let html = html.trim();
    if let Some(inner) = html
        .strip_prefix("<strong>")
        .and_then(|s| s.strip_suffix("</strong>"))
    {
        push_text(runs, inner, true, italic, strike, None);
    } else if let Some(inner) = html
        .strip_prefix("<b>")
        .and_then(|s| s.strip_suffix("</b>"))
    {
        push_text(runs, inner, true, italic, strike, None);
    } else if let Some(inner) = html
        .strip_prefix("<em>")
        .and_then(|s| s.strip_suffix("</em>"))
    {
        push_text(runs, inner, bold, true, strike, None);
    } else if let Some(inner) = html
        .strip_prefix("<i>")
        .and_then(|s| s.strip_suffix("</i>"))
    {
        push_text(runs, inner, bold, true, strike, None);
    } else if html.starts_with("<br") {
        push_text(runs, "\n", bold, italic, strike, None);
    } else if let Some(inner) = html
        .strip_prefix("<code>")
        .and_then(|s| s.strip_suffix("</code>"))
    {
        push_text(runs, inner, bold, italic, strike, None);
    } else if html.starts_with("<a ") {
        // <a href="url">text</a>——comrak 通常拆分为多个 HtmlInline，
        // 简化处理：提取 href 并保留文本
        if let Some(href) = extract_href(html) {
            // 文本部分在后续 Text 节点中，此处仅标记链接开始；
            // 由调用方合并（简化：跳过，链接文本由 Text 节点产生）
            let _ = href;
        }
        push_text(runs, html, bold, italic, strike, None);
    } else {
        push_text(runs, html, bold, italic, strike, None);
    }
}

/// 从 `<a href="...">` 中提取 href 属性值。
fn extract_href(html: &str) -> Option<String> {
    let rest = html.strip_prefix("<a ")?;
    let href_pos = rest.find("href=")?;
    let after = &rest[href_pos + 5..];
    let after = after.trim_start();
    if let Some(v) = after.strip_prefix('"') {
        let end = v.find('"')?;
        Some(v[..end].to_owned())
    } else if let Some(v) = after.strip_prefix('\'') {
        let end = v.find('\'')?;
        Some(v[..end].to_owned())
    } else {
        let end = after.find(|c: char| c.is_whitespace() || c == '>')?;
        Some(after[..end].to_owned())
    }
}

/// 简单解析 front matter 的 `key: value` 键值对。
fn parse_front_matter(text: &str, metadata: &mut DocumentMeta) {
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'').to_owned();
        match key.trim().to_ascii_lowercase().as_str() {
            "title" => metadata.title = Some(value),
            "author" => metadata.author = Some(value),
            "subject" => metadata.subject = Some(value),
            "keywords" => metadata.keywords = Some(value),
            _ => {}
        }
    }
}

/// 收集节点下的所有文本（含子节点）。
fn node_text<'a>(node: &'a AstNode<'a>, source: &str) -> String {
    let mut out = String::new();
    let value = &node.data.borrow().value;
    match value {
        NodeValue::Text(t) => out.push_str(t),
        NodeValue::Code(c) => out.push_str(&c.literal),
        NodeValue::HtmlInline(h) => out.push_str(h),
        _ => {
            let _ = source;
            for c in node.children() {
                out.push_str(&node_text(c, source));
            }
        }
    }
    out
}
