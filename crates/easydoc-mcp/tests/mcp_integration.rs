//! Integration tests for the `EasyDoc` MCP server.
//!
//! Tests the full JSON-RPC 2.0 request/response cycle without spawning a
//! subprocess — we call `server::handle_raw` directly.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Send a raw JSON-RPC message and return the parsed response.
fn call(raw: &str) -> serde_json::Value {
    let response_str = easydoc_mcp::server::handle_raw(raw)
        .expect("handle_raw failed")
        .expect("expected a response (got notification)");
    serde_json::from_str(&response_str).expect("response is not valid JSON")
}

/// 把路径转为可安全嵌入 JSON 字符串的表示（Windows 反斜杠转义）。
fn json_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

/// Create a minimal DOCX file for testing and return its path.
///
/// Uses `DocumentContent` + `EasyDoc::write_content` to build the file
/// without depending on the `Table` builder which requires `DocxRow`.
fn create_test_docx(dir: &Path) -> PathBuf {
    let path = dir.join("test.docx");
    let content = easydoc_core::DocumentContent {
        blocks: vec![
            easydoc_core::DocumentBlock::Heading {
                level: 1,
                runs: vec![easydoc_core::DocumentTextRun {
                    text: "Test Document".into(),
                    ..easydoc_core::DocumentTextRun::default()
                }],
            },
            easydoc_core::DocumentBlock::Paragraph(vec![easydoc_core::DocumentTextRun {
                text: "Hello, world!".into(),
                ..easydoc_core::DocumentTextRun::default()
            }]),
            easydoc_core::DocumentBlock::Table(easydoc_core::DocumentTable {
                rows: vec![
                    easydoc_core::DocumentTableRow {
                        cells: vec![
                            easydoc_core::DocumentTableCell {
                                blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                                    easydoc_core::DocumentTextRun {
                                        text: "A".into(),
                                        ..easydoc_core::DocumentTextRun::default()
                                    },
                                ])],
                                ..easydoc_core::DocumentTableCell::default()
                            },
                            easydoc_core::DocumentTableCell {
                                blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                                    easydoc_core::DocumentTextRun {
                                        text: "B".into(),
                                        ..easydoc_core::DocumentTextRun::default()
                                    },
                                ])],
                                ..easydoc_core::DocumentTableCell::default()
                            },
                        ],
                        is_header: true,
                    },
                    easydoc_core::DocumentTableRow {
                        cells: vec![
                            easydoc_core::DocumentTableCell {
                                blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                                    easydoc_core::DocumentTextRun {
                                        text: "C".into(),
                                        ..easydoc_core::DocumentTextRun::default()
                                    },
                                ])],
                                ..easydoc_core::DocumentTableCell::default()
                            },
                            easydoc_core::DocumentTableCell {
                                blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                                    easydoc_core::DocumentTextRun {
                                        text: "D".into(),
                                        ..easydoc_core::DocumentTextRun::default()
                                    },
                                ])],
                                ..easydoc_core::DocumentTableCell::default()
                            },
                        ],
                        is_header: false,
                    },
                ],
            }),
        ],
        ..easydoc_core::DocumentContent::default()
    };
    easydoc::EasyDoc::write_content(&content, &path).expect("write_content failed");
    path
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------

#[test]
fn initialize_returns_server_info() {
    let resp = call(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);

    assert_eq!(resp["id"], 1);
    assert!(resp["error"].is_null());
    let result = &resp["result"];
    assert_eq!(result["serverInfo"]["name"], "easydoc-mcp");
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
}

#[test]
fn initialize_with_protocol_version_param() {
    let resp = call(
        r#"{"jsonrpc":"2.0","id":42,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#,
    );
    assert_eq!(resp["id"], 42);
    assert!(resp["error"].is_null());
}

// ---------------------------------------------------------------------------
// tools/list
// ---------------------------------------------------------------------------

#[test]
fn tools_list_returns_six_tools() {
    let resp = call(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);

    assert_eq!(resp["id"], 2);
    assert!(resp["error"].is_null());
    let tools = resp["result"]["tools"]
        .as_array()
        .expect("tools is not an array");
    assert_eq!(tools.len(), 6);

    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"read_docx"));
    assert!(names.contains(&"read_table"));
    assert!(names.contains(&"read_docx_blocks"));
    assert!(names.contains(&"extract_images"));
    assert!(names.contains(&"convert_to_markdown"));
    assert!(names.contains(&"create_docx_from_data"));
}

#[test]
fn tools_list_each_tool_has_schema() {
    let resp = call(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#);
    let tools = resp["result"]["tools"].as_array().unwrap();

    for tool in tools {
        assert!(!tool["name"].as_str().unwrap().is_empty());
        assert!(!tool["description"].as_str().unwrap().is_empty());
        assert!(
            tool["inputSchema"].is_object(),
            "tool {} missing inputSchema",
            tool["name"]
        );
    }
}

// ---------------------------------------------------------------------------
// ping
// ---------------------------------------------------------------------------

#[test]
fn ping_returns_empty_object() {
    let resp = call(r#"{"jsonrpc":"2.0","id":10,"method":"ping"}"#);
    assert_eq!(resp["id"], 10);
    assert!(resp["error"].is_null());
    assert_eq!(resp["result"], serde_json::json!({}));
}

// ---------------------------------------------------------------------------
// tools/call — read_docx
// ---------------------------------------------------------------------------

#[test]
fn read_docx_annotated_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_docx(dir.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{{"name":"read_docx","arguments":{{"path":"{}","mode":"annotated"}}}}}}"#,
        json_path(&path)
    );
    let resp = call(&req);

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let content = &resp["result"]["content"];
    let arr = content.as_array().expect("content is not an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["type"], "text");
    let text = arr[0]["text"].as_str().unwrap();
    assert!(text.contains("Test Document"));
    assert!(text.contains("Hello, world!"));
}

#[test]
fn read_docx_plain_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_docx(dir.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{{"name":"read_docx","arguments":{{"path":"{}","mode":"plain"}}}}}}"#,
        json_path(&path)
    );
    let resp = call(&req);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Test Document"));
}

#[test]
fn read_docx_stats_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_docx(dir.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"read_docx","arguments":{{"path":"{}","mode":"stats"}}}}}}"#,
        json_path(&path)
    );
    let resp = call(&req);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("\u{6bb5}\u{843d}\u{6570}") || text.contains("\u{8868}\u{683c}\u{6570}"));
}

#[test]
fn read_docx_outline_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_docx(dir.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{{"name":"read_docx","arguments":{{"path":"{}","mode":"outline"}}}}}}"#,
        json_path(&path)
    );
    let resp = call(&req);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("# Test Document"));
}

#[test]
fn read_docx_default_mode_is_annotated() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_docx(dir.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{{"name":"read_docx","arguments":{{"path":"{}"}}}}}}"#,
        json_path(&path)
    );
    let resp = call(&req);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    // Annotated mode should include structural markers.
    assert!(text.contains("Test Document"));
}

// ---------------------------------------------------------------------------
// tools/call — read_table
// ---------------------------------------------------------------------------

#[test]
fn read_table_returns_all_tables() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_docx(dir.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{{"name":"read_table","arguments":{{"path":"{}"}}}}}}"#,
        json_path(&path)
    );
    let resp = call(&req);
    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    let tables = parsed["tables"].as_array().unwrap();
    assert!(!tables.is_empty());
}

#[test]
fn read_table_specific_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_docx(dir.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{{"name":"read_table","arguments":{{"path":"{}","sheet":0}}}}}}"#,
        json_path(&path)
    );
    let resp = call(&req);
    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    let table = parsed["table"].as_array().unwrap();
    assert_eq!(table.len(), 2); // 2 rows
}

// ---------------------------------------------------------------------------
// tools/call — read_docx_blocks
// ---------------------------------------------------------------------------

#[test]
fn read_docx_blocks_returns_semantic_model() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_docx(dir.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{{"name":"read_docx_blocks","arguments":{{"path":"{}"}}}}}}"#,
        json_path(&path)
    );
    let resp = call(&req);
    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    let blocks = parsed["document"]["blocks"].as_array().unwrap();
    assert!(!blocks.is_empty());
    // First block should be a heading.
    assert_eq!(blocks[0]["type"], "heading");
    assert_eq!(blocks[0]["text"], "Test Document");
}

// ---------------------------------------------------------------------------
// tools/call — convert_to_markdown
// ---------------------------------------------------------------------------

#[test]
fn convert_to_markdown_basic() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_docx(dir.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{{"name":"convert_to_markdown","arguments":{{"path":"{}"}}}}}}"#,
        json_path(&path)
    );
    let resp = call(&req);
    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    let md = parsed["markdown"].as_str().unwrap();
    assert!(
        md.contains("Test Document"),
        "markdown did not contain 'Test Document': {md}"
    );
    assert!(
        md.contains("Hello, world!"),
        "markdown did not contain 'Hello, world!': {md}"
    );
}

// ---------------------------------------------------------------------------
// tools/call — create_docx_from_data
// ---------------------------------------------------------------------------

#[test]
fn create_docx_heading_template() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("created.docx");

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{{"name":"create_docx_from_data","arguments":{{"path":"{}","template":"heading","data":{{"text":"Generated Title","level":2}}}}}}}}"#,
        json_path(&out_path)
    );
    let resp = call(&req);
    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    assert!(out_path.exists());

    // Read back and verify.
    let text = easydoc::EasyDoc::read_text(&out_path).unwrap();
    assert!(text.contains("Generated Title"));
}

#[test]
fn create_docx_table_template() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("table.docx");

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{{"name":"create_docx_from_data","arguments":{{"path":"{}","template":"table","data":{{"rows":[["Name","Age"],["Alice","30"],["Bob","25"]]}}}}}}}}"#,
        json_path(&out_path)
    );
    let resp = call(&req);
    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    assert!(out_path.exists());
}

#[test]
fn create_docx_list_template() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("list.docx");

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{{"name":"create_docx_from_data","arguments":{{"path":"{}","template":"list","data":{{"items":["First item","Second item","Third item"]}}}}}}}}"#,
        json_path(&out_path)
    );
    let resp = call(&req);
    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    assert!(out_path.exists());
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn unknown_method_returns_error() {
    let resp = call(r#"{"jsonrpc":"2.0","id":20,"method":"nonexistent/method"}"#);
    assert_eq!(resp["error"]["code"], -32601);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

#[test]
fn unknown_tool_returns_error() {
    let resp = call(
        r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}"#,
    );
    // The tool error is wrapped in a successful response with isError=true.
    assert!(resp["error"].is_null());
    let content = &resp["result"]["content"];
    let arr = content.as_array().unwrap();
    assert!(arr[0]["text"].as_str().unwrap().contains("unknown tool"));
    assert_eq!(resp["result"]["isError"], true);
}

#[test]
fn missing_path_param_returns_error() {
    let resp = call(
        r#"{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"read_docx","arguments":{}}}"#,
    );
    assert!(resp["error"].is_null());
    let content = &resp["result"]["content"];
    let arr = content.as_array().unwrap();
    assert!(
        arr[0]["text"]
            .as_str()
            .unwrap()
            .contains("missing required parameter")
    );
    assert_eq!(resp["result"]["isError"], true);
}

#[test]
fn invalid_json_returns_parse_error() {
    let resp = call("not json at all");
    assert_eq!(resp["error"]["code"], -32700);
}

#[test]
fn notification_produces_no_response() {
    let result = easydoc_mcp::server::handle_raw(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    )
    .unwrap();
    assert!(
        result.is_none(),
        "expected None for notification, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// round-trip: create → read
// ---------------------------------------------------------------------------

#[test]
fn round_trip_create_and_read() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("roundtrip.docx");

    // Create.
    let create_req = format!(
        r#"{{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{{"name":"create_docx_from_data","arguments":{{"path":"{}","template":"heading","data":{{"text":"Round Trip","level":1}}}}}}}}"#,
        json_path(&out_path)
    );
    let create_resp = call(&create_req);
    assert!(create_resp["error"].is_null());

    // Read back.
    let read_req = format!(
        r#"{{"jsonrpc":"2.0","id":31,"method":"tools/call","params":{{"name":"read_docx","arguments":{{"path":"{}","mode":"plain"}}}}}}"#,
        json_path(&out_path)
    );
    let read_resp = call(&read_req);
    let text = read_resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Round Trip"));
}
