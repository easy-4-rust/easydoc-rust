<a id="readme-top"></a>

<div align="center">

# easydoc-mcp

**MCP (Model Context Protocol) server exposing EasyDoc DOC/DOCX capabilities to LLM agents.**

[![Crates.io](https://img.shields.io/crates/v/easydoc-mcp)](https://crates.io/crates/easydoc-mcp)
[![docs.rs](https://img.shields.io/docsrs/easydoc-mcp)](https://docs.rs/easydoc-mcp)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#3-rust-baseline--platform-support)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[Positioning](#1-project-positioning--status) · [Tools](#2-tools) ·
[Resources](#3-resources) · [Prompts](#4-prompts) · [Install](#5-installation) ·
[Agent Config](#6-agent-integration) · [Protocol](#7-protocol) ·
[Security](#8-security) · [License](#10-license)

</div>

---

> **Current version**: `0.1.0-alpha.1`
> **MSRV**: Rust `1.88`
> **Edition**: `2024`
> **Maturity**: Alpha
> **Last verified**: 2026-08-11

## 1. Project Positioning & Status

### 1.1 What It Is

**`easydoc-mcp` is an MCP server that exposes `easydoc-rust` document operations to LLM agents.** It implements the Model Context Protocol over stdio (JSON-RPC 2.0, newline-delimited), allowing agents like Claude Code, Cursor, and other MCP-compatible runtimes to read, convert, and create DOCX/DOC documents.

| Dimension | Value |
|---|---|
| Crate | `easydoc-mcp` |
| Current version | `0.1.0-alpha.1` |
| MSRV / Edition | `1.88` / `2024` |
| Transport | stdio (JSON-RPC 2.0) |
| `unsafe` policy | `deny` (crate-level) |
| License | `Apache-2.0` |

### 1.2 What It Is Not

- Not a standalone CLI tool -- it runs as a subprocess spawned by an LLM agent runtime.
- Not an HTTP server -- it communicates exclusively over stdin/stdout.
- Not a general-purpose document converter -- it exposes only the operations defined by the MCP tool/resource/prompt contracts.

### 1.3 Capabilities Overview

| MCP Capability | Count | Details |
|---|---|---|
| Tools | 6 | Read, extract, convert, create documents |
| Resources | Dynamic | Directory scanning with path traversal protection |
| Prompts | 4 | Summarize, analyze, extract, compare |

## 2. Tools

The server exposes 6 tools via `tools/list`. Each tool accepts JSON input and returns JSON output.

### 2.1 Tool Definitions

| Tool | Description | Required Parameters | Optional Parameters |
|---|---|---|---|
| `read_docx` | Read a document in one of 4 view modes | `path` | `mode` (plain/annotated/outline/stats, default: annotated) |
| `read_table` | Extract tables as JSON arrays | `path` | `sheet` (0-based table index; omit for all) |
| `read_docx_blocks` | Read full semantic document model as JSON | `path` | -- |
| `extract_images` | Extract embedded images to a directory | `path`, `output_dir` | -- |
| `convert_to_markdown` | Convert DOCX/DOC to Markdown text | `path` | `options.image_dir`, `options.front_matter` |
| `create_docx_from_data` | Create a DOCX from structured data | `path`, `template`, `data` | -- |

### 2.2 Tool Input Schemas

#### `read_docx`

```json
{
  "path": "/absolute/path/to/document.docx",
  "mode": "annotated"
}
```

View modes:
- `plain` -- bare text, paragraphs separated by newlines
- `annotated` -- structural markers like `[Heading1]`, `[Paragraph 1]`, `[Table 1: 3x4]`
- `outline` -- headings only, Markdown-style `#` / `##`
- `stats` -- block and word counts

#### `read_table`

```json
{
  "path": "/absolute/path/to/document.docx",
  "sheet": 0
}
```

Returns `{"table": [[...]]}` for a specific table, or `{"tables": [[[...]], [[...]]]}` for all.

#### `read_docx_blocks`

```json
{
  "path": "/absolute/path/to/document.docx"
}
```

Returns the full `DocumentContent` model serialized as JSON (blocks, metadata, types).

#### `extract_images`

```json
{
  "path": "/absolute/path/to/document.docx",
  "output_dir": "/absolute/path/to/output"
}
```

Returns `{"extracted": [...], "count": N}`. Only works with DOCX (not legacy DOC).

#### `convert_to_markdown`

```json
{
  "path": "/absolute/path/to/document.docx",
  "options": {
    "image_dir": "assets",
    "front_matter": true
  }
}
```

Returns `{"markdown": "...", "warnings": N, "assets": N}`.

#### `create_docx_from_data`

Three template types are supported:

**Heading template:**
```json
{
  "path": "/output.docx",
  "template": "heading",
  "data": { "text": "Chapter 1", "level": 1 }
}
```

**Table template:**
```json
{
  "path": "/output.docx",
  "template": "table",
  "data": {
    "rows": [
      ["Name", "Age", "Email"],
      ["Alice", "30", "alice@example.com"]
    ]
  }
}
```

**List template:**
```json
{
  "path": "/output.docx",
  "template": "list",
  "data": {
    "items": ["First item", "Second item", "Third item"]
  }
}
```

## 3. Resources

The server implements MCP `resources/list` and `resources/read` to expose document files from a configured directory.

### 3.1 Directory Scanning

`DirectoryResourceProvider` scans a root directory for documents matching configured extensions (default: all files). Files are exposed as `file://` URIs.

```rust,ignore
use easydoc_mcp::DirectoryResourceProvider;

let provider = DirectoryResourceProvider::new("/path/to/docs")
    .recursive(true)
    .with_extensions(vec!["docx".into(), "doc".into()]);
```

### 3.2 Content Reading

When a resource is read via `resources/read`, the server converts the document to annotated text using `EasyDoc::view_as()` and returns it as `text/markdown`.

### 3.3 Path Traversal Protection

`DirectoryResourceProvider` performs path traversal protection on all `file://` URI reads. The resolved path must be within the configured root directory (checked via `canonicalize()`). Paths outside the root return `null`.

## 4. Prompts

The server exposes 4 built-in prompt templates via `prompts/list` and `prompts/get`.

| Prompt | Description | Required Args | Optional Args |
|---|---|---|---|
| `summarize_document` | Generate a concise document summary | `path` | `max_length` (default: 500) |
| `analyze_table_data` | Analyze table data in a document | `path` | `table_index` (0-based) |
| `extract_key_points` | Extract key points from a document | `path` | -- |
| `compare_documents` | Compare two documents for differences | `path_a`, `path_b` | -- |

Each prompt reads the document using `EasyDoc::view_as()` (Annotated mode) or `EasyDoc::load()` (for table analysis), then renders a structured message for the LLM.

## 5. Installation

### 5.1 From crates.io

```bash
cargo install easydoc-mcp
```

### 5.2 From workspace

```bash
cargo install --path crates/easydoc-mcp
```

### 5.3 As a dependency

```toml
[dependencies]
easydoc-mcp = "0.1.0-alpha.1"
```

## 6. Agent Integration

### 6.1 Claude Code

Add to `.claude/settings.json`:

```json
{
  "mcpServers": {
    "easydoc": {
      "command": "easydoc-mcp",
      "args": []
    }
  }
}
```

### 6.2 Cursor

Add to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "easydoc": {
      "command": "easydoc-mcp",
      "args": []
    }
  }
}
```

### 6.3 Generic MCP Client

The server speaks JSON-RPC 2.0 over newline-delimited stdin/stdout. Any MCP-compatible client can spawn it as a subprocess:

```bash
# Start the server
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | easydoc-mcp
```

### 6.4 Programmatic Usage

```rust
use easydoc_mcp::server;

// Process a single JSON-RPC message
let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
let response = server::handle_raw(request).unwrap();
println!("{response:?}");
```

## 7. Protocol

The server implements the following MCP methods:

| Method | Description |
|---|---|
| `initialize` | Returns server info and capabilities (tools, resources, prompts) |
| `tools/list` | Returns the 6 tool definitions with JSON Schema input specs |
| `tools/call` | Dispatches to the requested tool handler |
| `resources/list` | Returns available document resources from the configured directory |
| `resources/read` | Reads a resource by URI (converts to annotated text) |
| `prompts/list` | Returns the 4 prompt template definitions |
| `prompts/get` | Renders a prompt with the provided arguments |
| `ping` | Health check (returns `{}`) |
| `notifications/initialized` | Client acknowledgement (no response sent) |

## 8. Security

| Concern | Mitigation |
|---|---|
| Path traversal | `DirectoryResourceProvider` uses `canonicalize()` + root prefix check |
| File access scope | Only `file://` URIs within the configured root directory |
| Input validation | All tool parameters validated before execution |
| `unsafe` code | `deny` at crate level |
| Resource limits | ZIP bomb protection via `easydoc-ooxml` limits (10K entries, 256MB/entry, 1GB total) |

## 9. Build & Test

```bash
cargo check -p easydoc-mcp
cargo test -p easydoc-mcp
cargo clippy -p easydoc-mcp -- -D warnings
cargo doc -p easydoc-mcp --no-deps
```

## 10. License

Apache-2.0 -- see [LICENSE](../../LICENSE) for details.

---

<div align="center">

[Back to top](#readme-top) · [docs.rs](https://docs.rs/easydoc-mcp) · [crates.io](https://crates.io/crates/easydoc-mcp) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
