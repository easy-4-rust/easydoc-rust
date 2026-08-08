//! Template fill executors — placeholder replacement and collection expansion.
//!
//! DOCX files are ZIP archives containing XML. This module:
//! 1. Opens the template as a ZIP
//! 2. Modifies `word/document.xml` in-place (scalar replacement)
//! 3. For collection expansion, replicates table rows containing `{.field}`
//! 4. Writes a new ZIP preserving all other entries (styles, images, etc.)

use std::collections::HashMap;
use std::path::Path;

use crate::placeholder::Placeholder;
use easydoc_core::{DocError, Result};
use easydoc_ooxml::PackageRewriter;

/// Builder for template fill operations.
pub struct TemplateFillBuilder {
    template: std::path::PathBuf,
    output: std::path::PathBuf,
    data: HashMap<String, String>,
}

impl TemplateFillBuilder {
    /// Creates a new template fill builder.
    #[must_use]
    pub fn new(
        template: impl Into<std::path::PathBuf>,
        output: impl Into<std::path::PathBuf>,
    ) -> Self {
        Self {
            template: template.into(),
            output: output.into(),
            data: HashMap::new(),
        }
    }

    /// Registers a key-value pair for placeholder replacement.
    #[must_use]
    pub fn register(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }

    /// Executes the fill.
    ///
    /// # Errors
    ///
    /// Returns I/O or processing errors.
    pub fn do_fill(self) -> Result<()> {
        fill_scalar(&self.template, &self.output, &self.data)
    }
}

/// Fill scalar `{key}` placeholders in a DOCX template, preserving the ZIP structure.
///
/// Opens the template as ZIP, replaces placeholders in `word/document.xml`,
/// and writes a new DOCX with all other entries preserved.
///
/// # Errors
///
/// Returns I/O or ZIP processing errors.
pub fn fill_scalar(template: &Path, output: &Path, data: &HashMap<String, String>) -> Result<()> {
    PackageRewriter::default().rewrite(template, output, |name, content| {
        if name != "word/document.xml" {
            return Ok(None);
        }
        let xml = std::str::from_utf8(content)
            .map_err(|error| DocError::Format(format!("document.xml is not UTF-8: {error}")))?;
        Ok(Some(replace_scalar_placeholders(xml, data).into_bytes()))
    })
}

/// Fill list placeholders (`{.field}` / `{prefix.field}`) with collection expansion.
///
/// Finds the table row containing `{.` placeholders, replicates it for each
/// data item, and replaces field references with actual values.
///
/// # Errors
///
/// Returns I/O or processing errors.
pub fn fill_list<T: serde::Serialize + std::fmt::Debug>(
    template: &Path,
    output: &Path,
    data: &[T],
    list_field: &str,
) -> Result<()> {
    // Convert each item to a JSON key-value map
    let items: Vec<HashMap<String, String>> = data
        .iter()
        .map(|item| {
            let value = serde_json::to_value(item).map_err(|e| DocError::Conversion {
                field: list_field.to_owned(),
                value: format!("{item:?}"),
                message: format!("serialization error: {e}"),
            })?;
            to_string_map(&value)
        })
        .collect::<Result<Vec<_>>>()?;

    PackageRewriter::default().rewrite(template, output, |name, content| {
        if name != "word/document.xml" {
            return Ok(None);
        }
        let xml = std::str::from_utf8(content)
            .map_err(|error| DocError::Format(format!("document.xml is not UTF-8: {error}")))?;
        Ok(Some(
            expand_collection_rows(xml, &items, list_field)?.into_bytes(),
        ))
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Replaces all `{key}` placeholders with values from the data map.
fn replace_scalar_placeholders(xml: &str, data: &HashMap<String, String>) -> String {
    let mut result = xml.to_owned();

    for (key, replacement) in data {
        let placeholder = format!("{{{key}}}");
        result = replace_across_text_nodes(&result, &placeholder, &escape_xml_text(replacement));
    }

    result
}

/// 替换可能被 Word 拆到多个 `w:t` 节点中的占位符。
fn replace_across_text_nodes(xml: &str, placeholder: &str, replacement: &str) -> String {
    let mut visible = String::new();
    let mut nodes = Vec::new();
    let mut search_from = 0;
    let mut previous_close = 0;

    while let Some(relative_start) = xml[search_from..].find("<w:t") {
        let tag_start = search_from + relative_start;
        if xml[previous_close..tag_start].contains("</w:p>") {
            visible.push('\0');
        }
        let Some(tag_end_relative) = xml[tag_start..].find('>') else {
            break;
        };
        let content_start = tag_start + tag_end_relative + 1;
        let Some(close_relative) = xml[content_start..].find("</w:t>") else {
            break;
        };
        let content_end = content_start + close_relative;
        let visible_start = visible.len();
        visible.push_str(&xml[content_start..content_end]);
        let visible_end = visible.len();
        nodes.push((content_start, content_end, visible_start, visible_end));
        previous_close = content_end + "</w:t>".len();
        search_from = previous_close;
    }

    let mut changes = Vec::new();
    for (match_start, _) in visible.match_indices(placeholder) {
        let match_end = match_start + placeholder.len();
        let mut inserted = false;
        for &(content_start, _content_end, visible_start, visible_end) in &nodes {
            let overlap_start = match_start.max(visible_start);
            let overlap_end = match_end.min(visible_end);
            if overlap_start >= overlap_end {
                continue;
            }
            let absolute_start = content_start + overlap_start - visible_start;
            let absolute_end = content_start + overlap_end - visible_start;
            let value = if inserted {
                String::new()
            } else {
                inserted = true;
                replacement.to_owned()
            };
            changes.push((absolute_start, absolute_end, value));
        }
    }

    changes.sort_unstable_by_key(|change| std::cmp::Reverse(change.0));
    let mut result = xml.to_owned();
    for (start, end, value) in changes {
        result.replace_range(start..end, &value);
    }
    result
}

/// Finds table rows containing `{.field}` or `{prefix.field}` and expands them.
///
/// Strategy:
/// 1. Find the first `<w:tr>` that contains `{.` in its text
/// 2. Extract that row as a template
/// 3. Generate N copies of the row, replacing `{.field}` with data values
/// 4. Replace the original row with all generated rows
fn expand_collection_rows(
    xml: &str,
    items: &[HashMap<String, String>],
    _list_field: &str,
) -> Result<String> {
    // First try to find the placeholder inside a paragraph (<w:p>)
    // Pattern: find <w:p>...</w:p> containing {.field}
    if let Some(result) = try_expand_in_paragraphs(xml, items) {
        return Ok(result);
    }

    // Fallback: try table row expansion (<w:tr>)
    try_expand_in_table_rows(xml, items)
}

/// Try to expand collection placeholders found in paragraphs (<w:p>).
fn try_expand_in_paragraphs(xml: &str, items: &[HashMap<String, String>]) -> Option<String> {
    let p_start = xml.find("<w:p")?;
    let mut search_pos = p_start;

    while let Some(p_pos) = xml[search_pos..].find("<w:p") {
        let abs_p_start = search_pos + p_pos;
        // Check if this is a self-closing or empty paragraph
        let after_tag = &xml[abs_p_start..];
        let tag_end = after_tag.find('>')?;

        if after_tag[..tag_end].ends_with('/') {
            search_pos = abs_p_start + tag_end + 1;
            continue;
        }

        let p_end = find_matching_close(xml, abs_p_start, "w:p").ok()?;
        let p_content = &xml[abs_p_start..p_end];

        if p_content.contains("{.") {
            let template = p_content.to_owned();
            let mut expanded = String::new();
            for item in items {
                let mut replica = template.clone();
                for (key, value) in item {
                    let dot_key = format!("{{.{key}}}");
                    replica = replica.replace(&dot_key, &escape_xml_text(value));
                }
                expanded.push_str(&replica);
            }

            let mut result = xml[..abs_p_start].to_owned();
            result.push_str(&expanded);
            result.push_str(&xml[p_end..]);
            return Some(result);
        }
        search_pos = p_end;
    }

    None
}

/// Try to expand in table rows (<w:tr>).
fn try_expand_in_table_rows(xml: &str, items: &[HashMap<String, String>]) -> Result<String> {
    // Find any <w:tr> containing {. placeholder
    let tr_start = xml.find("<w:tr").ok_or_else(|| DocError::Template {
        placeholder: "{.field}".to_owned(),
        message: "no table row (<w:tr>) found in document".to_owned(),
    })?;

    // Find the specific <w:tr> that contains a collection placeholder
    let mut search_pos = tr_start;
    let mut template_row: Option<(usize, usize)> = None;

    while let Some(tr_pos) = xml[search_pos..].find("<w:tr") {
        let abs_tr_start = search_pos + tr_pos;
        // Find matching </w:tr> by counting nesting
        let row_end = find_matching_close(xml, abs_tr_start, "w:tr")?;
        let row_content = &xml[abs_tr_start..row_end];

        // Check if this row contains {. placeholder
        if row_content.contains("{.") {
            template_row = Some((abs_tr_start, row_end));
            break;
        }
        search_pos = row_end;
    }

    let (row_start, row_end) = template_row.ok_or_else(|| DocError::Template {
        placeholder: "{.field}".to_owned(),
        message: "no table row with collection placeholder ({.field}) found".to_owned(),
    })?;

    let row_template = &xml[row_start..row_end];

    // Generate expanded rows
    let mut expanded_rows = String::new();
    for item in items {
        let mut row = row_template.to_owned();
        // Replace {.field} and {prefix.field} with actual values
        for (key, value) in item {
            // Try both {.field} and {prefix.field} patterns
            let dot_key = format!("{{.{key}}}");
            row = row.replace(&dot_key, &escape_xml_text(value));

            // Also handle {prefix.field} — replace in the template row
            let placeholders = Placeholder::find_all(&row);
            for ph in &placeholders {
                match ph {
                    Placeholder::NamedCollection { raw, field, .. } if field == key => {
                        row = row.replace(raw.as_str(), &escape_xml_text(value));
                    }
                    Placeholder::Collection { raw, field } if field == key => {
                        row = row.replace(raw.as_str(), &escape_xml_text(value));
                    }
                    _ => {}
                }
            }
        }
        expanded_rows.push_str(&row);
    }

    // Replace the template row with all expanded rows
    let mut result = xml[..row_start].to_owned();
    result.push_str(&expanded_rows);
    result.push_str(&xml[row_end..]);

    Ok(result)
}

/// Converts a `serde_json::Value` to a flat `HashMap<String, String>`.
fn to_string_map(value: &serde_json::Value) -> Result<HashMap<String, String>> {
    match value {
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                let s = match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => String::new(),
                    other => format!("{other}"),
                };
                map.insert(k.clone(), s);
            }
            Ok(map)
        }
        other => Ok(HashMap::from([(
            "value".to_owned(),
            match other {
                serde_json::Value::String(s) => s.clone(),
                _ => format!("{other}"),
            },
        )])),
    }
}

/// 转义写入 `w:t` 文本节点的动态内容，避免生成无效 XML。
fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Finds the position of the matching closing tag for an XML element,
/// handling nested elements of the same name.
fn find_matching_close(xml: &str, tag_start: usize, tag_name: &str) -> Result<usize> {
    let open_tag = format!("<{tag_name}");
    let close_tag = format!("</{tag_name}>");
    let self_close = "/>";

    let mut depth = 0;
    let mut pos = tag_start;

    loop {
        if pos >= xml.len() {
            return Err(DocError::Format(format!(
                "unclosed tag <{tag_name}> at position {tag_start}"
            )));
        }

        let remaining = &xml[pos..];

        if remaining.starts_with(&open_tag) {
            // Check if it's self-closing: <w:tr ... />
            let tag_end = remaining.find('>').unwrap_or(remaining.len());
            let tag_content = &remaining[..tag_end];
            if tag_content.ends_with(self_close) {
                // Self-closing, don't change depth
                pos += tag_end + 1;
            } else {
                depth += 1;
                pos += open_tag.len();
            }
        } else if remaining.starts_with(&close_tag) {
            depth -= 1;
            if depth == 0 {
                return Ok(pos + close_tag.len());
            }
            pos += close_tag.len();
        } else {
            // Advance one character
            let mut chars = remaining.char_indices();
            chars.next(); // skip current char
            let next = chars.next().map_or(1, |(i, _)| i);
            pos += next;
        }
    }
}
