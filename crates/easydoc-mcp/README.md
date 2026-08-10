# easydoc-mcp

MCP (Model Context Protocol) server exposing [EasyDoc](https://github.com/easy-4-rust/easydoc-rust) capabilities to LLM agents.

## Overview

`easydoc-mcp` implements a stdio-based MCP server that allows LLM agents to read, convert, and create DOCX/DOC documents through the standard MCP tool interface. It uses JSON-RPC 2.0 over newline-delimited stdin/stdout — the standard transport for locally spawned MCP servers.

## Tools

| Tool | Description |
|------|-------------|
| `read_docx` | Read a document in plain, annotated, outline, or stats view mode |
| `read_table` | Extract tables as JSON arrays |
| `read_docx_blocks` | Read the full semantic document model as JSON |
| `extract_images` | Extract embedded images to a directory |
| `convert_to_markdown` | Convert DOCX/DOC to Markdown text |
| `create_docx_from_data` | Create a DOCX from structured data (heading, table, or list) |

## Usage

### As a subprocess

```bash
# The MCP server reads from stdin and writes to stdout.
# Most LLM agent runtimes spawn it as a child process.
easydoc-mcp
```

### Programmatic

```rust
use easydoc_mcp::server;

// Process a single JSON-RPC message.
let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
let response = server::handle_raw(request).unwrap();
println!("{response:?}");
```

## Protocol

The server implements the following MCP methods:

- `initialize` — returns server info and capabilities
- `tools/list` — returns the 6 tool definitions with JSON Schema input specs
- `tools/call` — dispatches to the requested tool and returns results
- `ping` — health check (returns `{}`)
- `notifications/initialized` — client acknowledgement (no response)

## License

Apache-2.0
