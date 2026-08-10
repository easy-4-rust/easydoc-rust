//! MCP tool definitions and dispatch logic.
//!
//! Defines the six tools exposed by `easydoc-mcp` and routes `tools/call`
//! requests to the appropriate `EasyDoc` API.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::protocol::ToolCallResult;

// ---------------------------------------------------------------------------
// Tool metadata (returned by `tools/list`)
// ---------------------------------------------------------------------------

/// Describes a single MCP tool: name, description, and JSON Schema for input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool identifier (must be unique).
    pub name: String,
    /// Human-readable description for the LLM.
    pub description: String,
    /// JSON Schema describing accepted parameters.
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// Returns the list of all tools advertised by this server.
#[must_use]
pub fn tool_definitions() -> Vec<Tool> {
    vec![
        Tool {
            name: "read_docx".into(),
            description: "Read a DOCX/DOC file and return its content in the requested view mode. \
                          Modes: plain (bare text), annotated (structural markers like [段落 1]), \
                          outline (headings only), stats (block and word counts)."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the DOCX or DOC file"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["plain", "annotated", "outline", "stats"],
                        "default": "annotated",
                        "description": "View mode for rendering the document"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "read_table".into(),
            description: "Read all tables from a DOCX/DOC file and return them as JSON arrays. \
                          Each table is an array of rows; each row is an array of cell strings. \
                          Use 'sheet' to select a specific table (0-based index); omit to get all."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the DOCX or DOC file"
                    },
                    "sheet": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "0-based table index to read; omit to return all tables"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "read_docx_blocks".into(),
            description: "Read a DOCX/DOC file and return its full semantic document model as JSON. \
                          The model includes blocks (headings, paragraphs, tables, lists, images, etc.) \
                          with their metadata. Useful for programmatic document analysis."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the DOCX or DOC file"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "extract_images".into(),
            description: "Extract all embedded images from a DOCX file and save them to the \
                          specified output directory. Returns a JSON list of extracted file paths. \
                          Only works with DOCX format (not legacy DOC)."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the DOCX file"
                    },
                    "output_dir": {
                        "type": "string",
                        "description": "Absolute path to the directory where images will be saved"
                    }
                },
                "required": ["path", "output_dir"]
            }),
        },
        Tool {
            name: "convert_to_markdown".into(),
            description: "Convert a DOCX/DOC file to Markdown text. Options control image extraction \
                          directory and whether to include YAML front matter with document metadata."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the DOCX or DOC file"
                    },
                    "options": {
                        "type": "object",
                        "properties": {
                            "image_dir": {
                                "type": "string",
                                "description": "Directory to save extracted images; omit to skip extraction"
                            },
                            "front_matter": {
                                "type": "boolean",
                                "default": false,
                                "description": "Whether to include YAML front matter"
                            }
                        }
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "create_docx_from_data".into(),
            description: "Create a new DOCX file from structured data. Supports three templates: \
                          'heading' (a single heading paragraph), 'table' (rows of cells), and \
                          'list' (bullet items). The output file is written to 'path'."
                .into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path for the output DOCX file"
                    },
                    "template": {
                        "type": "string",
                        "enum": ["heading", "table", "list"],
                        "description": "Document template type"
                    },
                    "data": {
                        "type": "object",
                        "description": "Template-specific data (see below)",
                        "properties": {
                            "text": {
                                "type": "string",
                                "description": "For 'heading': the heading text"
                            },
                            "level": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 6,
                                "default": 1,
                                "description": "For 'heading': heading level (1-6)"
                            },
                            "rows": {
                                "type": "array",
                                "items": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "description": "For 'table': array of rows, each row is an array of cell strings"
                            },
                            "items": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "For 'list': array of bullet item strings"
                            }
                        }
                    }
                },
                "required": ["path", "template", "data"]
            }),
        },
    ]
}

// ---------------------------------------------------------------------------
// Tool dispatch
// ---------------------------------------------------------------------------

/// Dispatch a `tools/call` request to the appropriate handler.
///
/// # Errors
///
/// Returns an error result (not a Rust `Err`) for invalid parameters or
/// tool execution failures.  Rust `Err` is reserved for protocol-level bugs.
#[must_use]
pub fn call_tool(name: &str, args: &serde_json::Value) -> ToolCallResult {
    let result = match name {
        "read_docx" => handle_read_docx(args),
        "read_table" => handle_read_table(args),
        "read_docx_blocks" => handle_read_docx_blocks(args),
        "extract_images" => handle_extract_images(args),
        "convert_to_markdown" => handle_convert_to_markdown(args),
        "create_docx_from_data" => handle_create_docx_from_data(args),
        _ => return ToolCallResult::error(format!("unknown tool: {name}")),
    };

    match result {
        Ok(value) => {
            let text = match serde_json::to_string_pretty(&value) {
                Ok(s) => s,
                Err(e) => return ToolCallResult::error(format!("serialisation error: {e}")),
            };
            ToolCallResult::text(text)
        }
        Err(e) => ToolCallResult::error(format!("{e:#}")),
    }
}

// ---------------------------------------------------------------------------
// Individual handlers
// ---------------------------------------------------------------------------

/// Extract the required `path` argument as an absolute `PathBuf`.
fn require_path(args: &serde_json::Value) -> anyhow::Result<PathBuf> {
    let s = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: path"))?;
    Ok(PathBuf::from(s))
}

/// Handler for `read_docx`.
fn handle_read_docx(args: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let path = require_path(args)?;
    let mode_str = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("annotated");

    let mode = match mode_str {
        "plain" => easydoc_reader::ViewMode::Plain,
        "annotated" => easydoc_reader::ViewMode::Annotated,
        "outline" => easydoc_reader::ViewMode::Outline { max_level: 3 },
        "stats" => easydoc_reader::ViewMode::Stats,
        other => return Err(anyhow::anyhow!("unknown view mode: {other}")),
    };

    let content = easydoc::EasyDoc::view_as(&path, &mode)?;
    Ok(serde_json::json!({ "content": content }))
}

/// Handler for `read_table`.
fn handle_read_table(args: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let path = require_path(args)?;
    let content = easydoc::EasyDoc::load(&path)?;

    let tables: Vec<Vec<Vec<String>>> = content
        .blocks
        .iter()
        .filter_map(|block| {
            if let easydoc_core::DocumentBlock::Table(table) = block {
                let rows: Vec<Vec<String>> = table
                    .rows
                    .iter()
                    .map(|row| {
                        row.cells
                            .iter()
                            .map(|cell| extract_cell_text(&cell.blocks))
                            .collect()
                    })
                    .collect();
                Some(rows)
            } else {
                None
            }
        })
        .collect();

    if let Some(sheet) = args.get("sheet").and_then(serde_json::Value::as_u64) {
        let idx = sheet as usize;
        if idx >= tables.len() {
            return Err(anyhow::anyhow!(
                "sheet index {idx} out of range (document has {} table(s))",
                tables.len()
            ));
        }
        Ok(serde_json::json!({ "table": tables[idx] }))
    } else {
        Ok(serde_json::json!({ "tables": tables }))
    }
}

/// Handler for `read_docx_blocks`.
fn handle_read_docx_blocks(args: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let path = require_path(args)?;
    let content = easydoc::EasyDoc::load(&path)?;
    let json = document_to_json(&content);
    Ok(serde_json::json!({ "document": json }))
}

/// Handler for `extract_images`.
fn handle_extract_images(args: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let path = require_path(args)?;
    let output_dir = args
        .get("output_dir")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: output_dir"))?;
    let output_dir = PathBuf::from(output_dir);

    std::fs::create_dir_all(&output_dir)?;

    // Open the DOCX as a ZIP archive and extract image parts.
    let file = std::fs::File::open(&path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| anyhow::anyhow!("not a valid ZIP/DOCX: {e}"))?;

    // Parse relationships to find image entries.
    let rels_xml = {
        let mut rels_file = archive
            .by_name("word/_rels/document.xml.rels")
            .map_err(|e| anyhow::anyhow!("missing relationships file: {e}"))?;
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut rels_file, &mut buf)?;
        buf
    };

    let rels = easydoc_reader::extractor::image::Relationships::parse(&rels_xml)?;

    let mut extracted = Vec::new();
    let image_entries = collect_image_entries(&rels_xml);

    for (rel_id, _target) in &image_entries {
        if let Some(zip_path) = rels.resolve_image(rel_id)
            && let Ok(bytes) =
                easydoc_reader::extractor::image::read_zip_part(&mut archive, zip_path)
        {
            let filename = std::path::Path::new(zip_path).file_name().map_or_else(
                || format!("{rel_id}.bin"),
                |f| f.to_string_lossy().into_owned(),
            );
            let dest = output_dir.join(&filename);
            std::fs::write(&dest, &bytes)?;
            extracted.push(dest.to_string_lossy().into_owned());
        }
    }

    Ok(serde_json::json!({ "extracted": extracted, "count": extracted.len() }))
}

/// Handler for `convert_to_markdown`.
fn handle_convert_to_markdown(args: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let path = require_path(args)?;

    let mut builder = easydoc_markdown::MarkdownBuilder::new(&path);

    if let Some(opts) = args.get("options") {
        if let Some(dir) = opts.get("image_dir").and_then(|v| v.as_str()) {
            builder = builder.image_directory(dir);
        }
        if let Some(fm) = opts
            .get("front_matter")
            .and_then(serde_json::Value::as_bool)
        {
            builder = builder.include_front_matter(fm);
        }
    }

    let result = builder.do_convert()?;
    Ok(serde_json::json!({
        "markdown": result.markdown,
        "warnings": result.warnings.len(),
        "assets": result.assets.len(),
    }))
}

/// Handler for `create_docx_from_data`.
///
/// Uses `DocBuilder` for heading template and `DocumentContent` + `EasyDoc::write_content`
/// for table and list templates, since the `Table` builder requires `DocxRow`.
fn handle_create_docx_from_data(args: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let path = require_path(args)?;
    let template = args
        .get("template")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: template"))?;
    let data = args
        .get("data")
        .ok_or_else(|| anyhow::anyhow!("missing required parameter: data"))?;

    match template {
        "heading" => {
            let text = data
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("data.text is required for heading template"))?;
            let level = data
                .get("level")
                .and_then(serde_json::Value::as_u64)
                .map_or(1, |l| l as u8);
            let heading_level = match level {
                1 => easydoc_core::HeadingLevel::H1,
                2 => easydoc_core::HeadingLevel::H2,
                3 => easydoc_core::HeadingLevel::H3,
                4 => easydoc_core::HeadingLevel::H4,
                5 => easydoc_core::HeadingLevel::H5,
                6 => easydoc_core::HeadingLevel::H6,
                _ => return Err(anyhow::anyhow!("heading level must be 1-6")),
            };
            easydoc::EasyDoc::document(&path)
                .add_heading(text, heading_level)
                .build()?
                .save()?;
        }
        "table" => {
            let rows = data
                .get("rows")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow::anyhow!("data.rows is required for table template"))?;

            let table_rows: Vec<easydoc_core::DocumentTableRow> = rows
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    let cells = row
                        .as_array()
                        .ok_or_else(|| anyhow::anyhow!("row {i} must be an array"))?;
                    let table_cells: Vec<easydoc_core::DocumentTableCell> = cells
                        .iter()
                        .map(|cell| {
                            let text = cell.as_str().unwrap_or("");
                            easydoc_core::DocumentTableCell {
                                blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                                    easydoc_core::DocumentTextRun {
                                        text: text.into(),
                                        ..easydoc_core::DocumentTextRun::default()
                                    },
                                ])],
                                ..easydoc_core::DocumentTableCell::default()
                            }
                        })
                        .collect();
                    Ok(easydoc_core::DocumentTableRow {
                        cells: table_cells,
                        is_header: i == 0,
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;

            let content = easydoc_core::DocumentContent {
                blocks: vec![easydoc_core::DocumentBlock::Table(
                    easydoc_core::DocumentTable { rows: table_rows },
                )],
                ..easydoc_core::DocumentContent::default()
            };
            easydoc::EasyDoc::write_content(&content, &path)?;
        }
        "list" => {
            let items = data
                .get("items")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow::anyhow!("data.items is required for list template"))?;

            let list_items: Vec<easydoc_core::DocumentListItem> = items
                .iter()
                .map(|item| {
                    let text = item.as_str().unwrap_or("");
                    easydoc_core::DocumentListItem {
                        blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                            easydoc_core::DocumentTextRun {
                                text: text.into(),
                                ..easydoc_core::DocumentTextRun::default()
                            },
                        ])],
                        ..easydoc_core::DocumentListItem::default()
                    }
                })
                .collect();

            let content = easydoc_core::DocumentContent {
                blocks: vec![easydoc_core::DocumentBlock::List(
                    easydoc_core::DocumentList {
                        ordered: false,
                        items: list_items,
                        ..easydoc_core::DocumentList::default()
                    },
                )],
                ..easydoc_core::DocumentContent::default()
            };
            easydoc::EasyDoc::write_content(&content, &path)?;
        }
        other => return Err(anyhow::anyhow!("unknown template: {other}")),
    }

    Ok(serde_json::json!({ "path": path.to_string_lossy(), "template": template }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract plain text from a list of `DocumentBlock`s inside a table cell.
fn extract_cell_text(blocks: &[easydoc_core::DocumentBlock]) -> String {
    let mut parts = Vec::new();
    for block in blocks {
        match block {
            easydoc_core::DocumentBlock::Paragraph(runs)
            | easydoc_core::DocumentBlock::Heading { runs, .. } => {
                let text: String = runs.iter().map(|r| r.text.as_str()).collect();
                parts.push(text);
            }
            easydoc_core::DocumentBlock::Table(_) => {
                parts.push("[nested table]".into());
            }
            _ => {}
        }
    }
    parts.join(" ")
}

/// Serialize a `DocumentContent` into a JSON-friendly value for the
/// `read_docx_blocks` tool.  We convert blocks recursively.
fn document_to_json(doc: &easydoc_core::DocumentContent) -> serde_json::Value {
    serde_json::json!({
        "metadata": {
            "title": doc.metadata.title,
            "author": doc.metadata.author,
        },
        "blocks": doc.blocks.iter().map(block_to_json).collect::<Vec<_>>(),
    })
}

/// Convert a single `DocumentBlock` to JSON.
fn block_to_json(block: &easydoc_core::DocumentBlock) -> serde_json::Value {
    match block {
        easydoc_core::DocumentBlock::Heading { level, runs } => serde_json::json!({
            "type": "heading",
            "level": level,
            "text": runs_text(runs),
        }),
        easydoc_core::DocumentBlock::Paragraph(runs) => serde_json::json!({
            "type": "paragraph",
            "text": runs_text(runs),
        }),
        easydoc_core::DocumentBlock::Table(table) => serde_json::json!({
            "type": "table",
            "rows": table.rows.len(),
            "columns": table.rows.first().map_or(0, |r| r.cells.len()),
        }),
        easydoc_core::DocumentBlock::List(list) => serde_json::json!({
            "type": "list",
            "items": list.items.len(),
        }),
        easydoc_core::DocumentBlock::Image(img) => serde_json::json!({
            "type": "image",
            "alt_text": img.alt_text,
            "extension": img.extension,
            "size_bytes": img.data.as_ref().map_or(0, Vec::len),
        }),
        easydoc_core::DocumentBlock::ThematicBreak => {
            serde_json::json!({ "type": "thematic_break" })
        }
        easydoc_core::DocumentBlock::PageBreak => serde_json::json!({ "type": "page_break" }),
        easydoc_core::DocumentBlock::ColumnBreak => serde_json::json!({ "type": "column_break" }),
        easydoc_core::DocumentBlock::CodeBlock { language, code } => serde_json::json!({
            "type": "code_block",
            "language": language,
            "lines": code.lines().count(),
        }),
        easydoc_core::DocumentBlock::TextBox(blocks) => serde_json::json!({
            "type": "text_box",
            "blocks": blocks.len(),
        }),
        easydoc_core::DocumentBlock::Footnote { id, blocks } => serde_json::json!({
            "type": "footnote",
            "id": id,
            "blocks": blocks.len(),
        }),
        easydoc_core::DocumentBlock::Endnote { id, blocks } => serde_json::json!({
            "type": "endnote",
            "id": id,
            "blocks": blocks.len(),
        }),
        easydoc_core::DocumentBlock::Section {
            blocks,
            section_type,
        } => serde_json::json!({
            "type": "section",
            "section_type": section_type,
            "blocks": blocks.len(),
        }),
        easydoc_core::DocumentBlock::Math { latex, display, .. } => serde_json::json!({
            "type": "math",
            "latex": latex,
            "display": display,
        }),
        // Future variants from #[non_exhaustive]
        other => serde_json::json!({
            "type": "unknown",
            "debug": format!("{other:?}"),
        }),
    }
}

/// Concatenate text runs into a single string.
fn runs_text(runs: &[easydoc_core::DocumentTextRun]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
}

/// Parse relationship XML to extract image relationship IDs and targets.
fn collect_image_entries(xml: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Empty(ref tag)) => {
                if tag.name().as_ref() == b"Relationship" {
                    let mut id = None;
                    let mut target = None;
                    let mut is_image = false;
                    for attr in tag.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"Id" => {
                                id = attr
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .ok()
                                    .map(std::borrow::Cow::into_owned);
                            }
                            b"Target" => {
                                target = attr
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .ok()
                                    .map(std::borrow::Cow::into_owned);
                            }
                            b"Type" => {
                                if let Ok(val) =
                                    attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    && val.ends_with("/image")
                                {
                                    is_image = true;
                                }
                            }
                            _ => {}
                        }
                    }
                    if is_image && let (Some(i), Some(t)) = (id, target) {
                        entries.push((i, t));
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    entries
}
