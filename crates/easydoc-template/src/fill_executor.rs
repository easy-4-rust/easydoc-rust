//! Template fill executors — placeholder replacement and collection expansion.
//!
//! DOCX files are ZIP archives containing XML. This module:
//! 1. Opens the template as a ZIP
//! 2. Modifies `word/document.xml` in-place (scalar replacement)
//! 3. For collection expansion, replicates table rows containing `{.field}`
//! 4. Writes a new ZIP preserving all other entries (styles, images, etc.)

use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use easydoc_core::{DocError, Result};
use crate::placeholder::Placeholder;

/// Helper to convert zip errors to DocError.
fn zip_err(e: zip::result::ZipError) -> DocError {
    DocError::Zip(e.to_string())
}

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
pub fn fill_scalar(
    template: &Path,
    output: &Path,
    data: &HashMap<String, String>,
) -> Result<()> {
    let template_bytes = fs::read(template)?;
    let reader = Cursor::new(template_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e: zip::result::ZipError| DocError::Zip(e.to_string()))?;

    // Build the output ZIP
    let out_file = fs::File::create(output)?;
    let mut out_zip = zip::ZipWriter::new(out_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(zip_err)?;
        let name = entry.name().to_owned();

        if entry.is_dir() {
            out_zip.add_directory(name, options)
                .map_err(zip_err)?;
            continue;
        }

        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;
        let content_str = String::from_utf8_lossy(&content).to_string();

        // Only process word/document.xml for placeholder replacement
        let modified = if name == "word/document.xml" {
            replace_scalar_placeholders(&content_str, data)
        } else {
            content_str
        };

        out_zip.start_file(name, options)
            .map_err(zip_err)?;
        out_zip.write_all(modified.as_bytes())?;
    }

    out_zip.finish().map_err(zip_err)?;
    Ok(())
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
            let value = serde_json::to_value(item)
                .map_err(|e| DocError::Conversion {
                    field: list_field.to_owned(),
                    value: format!("{item:?}"),
                    message: format!("serialization error: {e}"),
                })?;
            to_string_map(&value)
        })
        .collect::<Result<Vec<_>>>()?;

    let template_bytes = fs::read(template)?;
    let reader = Cursor::new(template_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(zip_err)?;

    let out_file = fs::File::create(output)?;
    let mut out_zip = zip::ZipWriter::new(out_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(zip_err)?;
        let name = entry.name().to_owned();

        if entry.is_dir() {
            out_zip.add_directory(name, options)
                .map_err(zip_err)?;
            continue;
        }

        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;
        let content_str = String::from_utf8_lossy(&content).to_string();

        let modified = if name == "word/document.xml" {
            expand_collection_rows(&content_str, &items, list_field)?
        } else {
            content_str
        };

        out_zip.start_file(name, options)
            .map_err(zip_err)?;
        out_zip.write_all(modified.as_bytes())?;
    }

    out_zip.finish().map_err(zip_err)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Replaces all `{key}` placeholders with values from the data map.
fn replace_scalar_placeholders(xml: &str, data: &HashMap<String, String>) -> String {
    let placeholders = Placeholder::find_all(xml);
    let mut result = xml.to_owned();

    for placeholder in &placeholders {
        if let Placeholder::Scalar { raw, key } = placeholder {
            if let Some(replacement) = data.get(key) {
                result = result.replace(raw.as_str(), replacement);
            }
        }
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
                    replica = replica.replace(&dot_key, value);
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
fn try_expand_in_table_rows(
    xml: &str,
    items: &[HashMap<String, String>],
) -> Result<String> {
    // Find any <w:tr> containing {. placeholder
    let tr_start = xml.find("<w:tr")
        .ok_or_else(|| DocError::Template {
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
            row = row.replace(&dot_key, value);

            // Also handle {prefix.field} — replace in the template row
            let placeholders = Placeholder::find_all(&row);
            for ph in &placeholders {
                match ph {
                    Placeholder::NamedCollection { raw, field, .. } if field == key => {
                        row = row.replace(raw.as_str(), value);
                    }
                    Placeholder::Collection { raw, field } if field == key => {
                        row = row.replace(raw.as_str(), value);
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
            let next = chars.next().map(|(i, _)| i).unwrap_or(1);
            pos += next;
        }
    }
}
