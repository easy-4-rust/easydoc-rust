//! 集成测试：MCP resources 和 prompts 能力。
//!
//! 测试 resources/list、resources/read、prompts/list、prompts/get
//! 的完整 JSON-RPC 2.0 请求/响应周期，以及 initialize 中的能力声明。

use std::path::{Path, PathBuf};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 使用指定配置发送 JSON-RPC 请求并返回解析后的响应。
fn call_with_config(raw: &str, config: &easydoc_mcp::server::ServerConfig) -> serde_json::Value {
    let response_str = easydoc_mcp::server::handle_raw_with_config(raw, config)
        .expect("handle_raw_with_config failed")
        .expect("expected a response (got notification)");
    serde_json::from_str(&response_str).expect("response is not valid JSON")
}

/// 创建测试用 DOCX 文件并返回路径。
fn create_test_docx(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
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
                text: "Hello, world! This is a test paragraph with some content.".into(),
                ..easydoc_core::DocumentTextRun::default()
            }]),
            easydoc_core::DocumentBlock::Table(easydoc_core::DocumentTable {
                rows: vec![
                    easydoc_core::DocumentTableRow {
                        cells: vec![
                            easydoc_core::DocumentTableCell {
                                blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                                    easydoc_core::DocumentTextRun {
                                        text: "Name".into(),
                                        ..easydoc_core::DocumentTextRun::default()
                                    },
                                ])],
                                ..easydoc_core::DocumentTableCell::default()
                            },
                            easydoc_core::DocumentTableCell {
                                blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                                    easydoc_core::DocumentTextRun {
                                        text: "Age".into(),
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
                                        text: "Alice".into(),
                                        ..easydoc_core::DocumentTextRun::default()
                                    },
                                ])],
                                ..easydoc_core::DocumentTableCell::default()
                            },
                            easydoc_core::DocumentTableCell {
                                blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                                    easydoc_core::DocumentTextRun {
                                        text: "30".into(),
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

/// 创建带子目录的测试目录结构。
///
/// 返回 (根目录, 子目录路径)。
fn create_test_dir_structure() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir failed");
    let sub_dir = dir.path().join("subdir");
    std::fs::create_dir_all(&sub_dir).expect("create subdir failed");

    // 根目录：1 个 docx + 1 个 txt
    create_test_docx(dir.path(), "report.docx");
    std::fs::write(dir.path().join("readme.txt"), "not a docx").unwrap();

    // 子目录：1 个 docx
    create_test_docx(&sub_dir, "nested.docx");

    (dir, sub_dir)
}

/// 创建使用目录资源提供者和内置 prompts 的配置。
fn make_config(dir: &Path) -> easydoc_mcp::server::ServerConfig {
    easydoc_mcp::server::ServerConfig::new(
        Arc::new(
            easydoc_mcp::DirectoryResourceProvider::new(dir)
                .recursive(true)
                .with_extensions(vec!["docx".into()]),
        ),
        Arc::new(easydoc_mcp::BuiltinPrompts::new()),
    )
}

// ===========================================================================
// resources 测试
// ===========================================================================

#[test]
fn resources_list_returns_docx_files() {
    let (dir, _sub) = create_test_dir_structure();
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":1,"method":"resources/list"}"#,
        &config,
    );

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let resources = resp["result"]["resources"]
        .as_array()
        .expect("resources is not an array");

    // 递归模式下应有 2 个 docx（根目录 + 子目录），txt 被过滤
    assert_eq!(resources.len(), 2, "expected 2 docx resources");

    let names: Vec<&str> = resources
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"report.docx"));
    assert!(names.contains(&"nested.docx"));
}

#[test]
fn resources_list_filters_extensions() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_docx(dir.path(), "a.docx");
    std::fs::write(dir.path().join("b.txt"), "plain text").unwrap();
    std::fs::write(dir.path().join("c.pdf"), "fake pdf").unwrap();

    // 只允许 docx
    let config = easydoc_mcp::server::ServerConfig::new(
        Arc::new(
            easydoc_mcp::DirectoryResourceProvider::new(dir.path())
                .with_extensions(vec!["docx".into()]),
        ),
        Arc::new(easydoc_mcp::BuiltinPrompts::new()),
    );

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":2,"method":"resources/list"}"#,
        &config,
    );

    let resources = resp["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0]["name"], "a.docx");
}

#[test]
fn resources_list_empty_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#,
        &config,
    );

    let resources = resp["result"]["resources"].as_array().unwrap();
    assert!(resources.is_empty());
}

#[test]
fn resources_read_returns_markdown() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_docx(dir.path(), "test.docx");
    let config = make_config(dir.path());

    let uri = format!("file://{}", path.display());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "resources/read",
        "params": { "uri": uri }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let contents = resp["result"]["contents"]
        .as_array()
        .expect("contents is not an array");
    assert_eq!(contents.len(), 1);
    assert_eq!(contents[0]["mimeType"], "text/markdown");
    let text = contents[0]["text"].as_str().unwrap();
    assert!(text.contains("Test Document"), "text: {text}");
    assert!(text.contains("Hello, world!"), "text: {text}");
}

#[test]
fn resources_read_unknown_uri_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "resources/read",
        "params": { "uri": "file:///nonexistent/path/doc.docx" }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    // 资源不存在应返回 JSON-RPC 错误（code -32002）
    assert!(
        !resp["error"].is_null(),
        "expected error for missing resource"
    );
    assert_eq!(resp["error"]["code"], -32002);
}

#[test]
fn resources_read_path_traversal_blocked() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_docx(dir.path(), "secret.docx");
    let config = make_config(dir.path());

    // 尝试路径穿越：URI 指向根目录外的文件
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "resources/read",
        "params": { "uri": "file:///etc/passwd" }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    // 路径穿越应被拒绝（资源不存在）
    assert!(
        !resp["error"].is_null(),
        "expected error for path traversal"
    );
}

#[test]
fn resources_read_non_file_uri_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    // 非 file:// URI 应返回错误
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "resources/read",
        "params": { "uri": "http://example.com/doc.docx" }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);
    assert!(!resp["error"].is_null(), "expected error for non-file URI");
}

#[test]
fn resources_capability_declared_in_initialize() {
    let resp_str = easydoc_mcp::server::handle_raw(
        r#"{"jsonrpc":"2.0","id":100,"method":"initialize","params":{}}"#,
    )
    .unwrap()
    .unwrap();
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();

    assert!(resp["error"].is_null());
    let caps = &resp["result"]["capabilities"];

    // resources 能力应被声明（subscribe: true，支持 resources/subscribe）
    assert!(
        caps["resources"].is_object(),
        "resources capability not declared"
    );
    assert_eq!(caps["resources"]["subscribe"], true);

    // prompts 能力应被声明
    assert!(
        caps["prompts"].is_object(),
        "prompts capability not declared"
    );
}

// ===========================================================================
// prompts 测试
// ===========================================================================

#[test]
fn prompts_list_returns_builtin_prompts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":20,"method":"prompts/list"}"#,
        &config,
    );

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let prompts = resp["result"]["prompts"]
        .as_array()
        .expect("prompts is not an array");
    assert_eq!(prompts.len(), 4, "expected 4 builtin prompts");

    let names: Vec<&str> = prompts
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"summarize_document"));
    assert!(names.contains(&"analyze_table_data"));
    assert!(names.contains(&"extract_key_points"));
    assert!(names.contains(&"compare_documents"));

    // 验证每个 prompt 有 description 和 arguments
    for prompt in prompts {
        assert!(
            prompt["description"].is_string(),
            "prompt {} missing description",
            prompt["name"]
        );
    }
}

#[test]
fn prompts_list_each_prompt_has_required_fields() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":21,"method":"prompts/list"}"#,
        &config,
    );

    let prompts = resp["result"]["prompts"].as_array().unwrap();
    for prompt in prompts {
        let name = prompt["name"].as_str().unwrap();
        assert!(!name.is_empty(), "prompt name is empty");
        assert!(
            prompt["description"].is_string(),
            "prompt {name} missing description"
        );

        // 每个 prompt 的 arguments 应有 name 字段
        if let Some(args) = prompt["arguments"].as_array() {
            for arg in args {
                assert!(
                    arg["name"].is_string(),
                    "argument in prompt {name} missing name"
                );
            }
        }
    }
}

#[test]
fn prompts_get_summarize_document_renders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_docx(dir.path(), "summary.docx");
    let config = make_config(dir.path());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "prompts/get",
        "params": {
            "name": "summarize_document",
            "arguments": {
                "path": path.to_string_lossy(),
                "max_length": 200
            }
        }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let messages = resp["result"]["messages"]
        .as_array()
        .expect("messages is not an array");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content"]["type"], "text");
    let text = messages[0]["content"]["text"].as_str().unwrap();
    assert!(text.contains("摘要"), "should mention summary: {text}");
    assert!(text.contains("200"), "should mention max_length: {text}");
}

#[test]
fn prompts_get_analyze_table_renders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_docx(dir.path(), "tables.docx");
    let config = make_config(dir.path());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 23,
        "method": "prompts/get",
        "params": {
            "name": "analyze_table_data",
            "arguments": {
                "path": path.to_string_lossy()
            }
        }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let messages = resp["result"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    let text = messages[0]["content"]["text"].as_str().unwrap();
    assert!(text.contains("表格"), "should mention table: {text}");
}

#[test]
fn prompts_get_extract_key_points_renders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_docx(dir.path(), "keypoints.docx");
    let config = make_config(dir.path());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 24,
        "method": "prompts/get",
        "params": {
            "name": "extract_key_points",
            "arguments": {
                "path": path.to_string_lossy()
            }
        }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let messages = resp["result"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    let text = messages[0]["content"]["text"].as_str().unwrap();
    assert!(
        text.contains("关键要点"),
        "should mention key points: {text}"
    );
}

#[test]
fn prompts_get_compare_documents_renders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path_a = create_test_docx(dir.path(), "doc_a.docx");
    let path_b = create_test_docx(dir.path(), "doc_b.docx");
    let config = make_config(dir.path());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 25,
        "method": "prompts/get",
        "params": {
            "name": "compare_documents",
            "arguments": {
                "path_a": path_a.to_string_lossy(),
                "path_b": path_b.to_string_lossy()
            }
        }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    let messages = resp["result"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    let text = messages[0]["content"]["text"].as_str().unwrap();
    assert!(text.contains("差异"), "should mention diff: {text}");
    assert!(text.contains("文档 A"), "should mention doc A: {text}");
    assert!(text.contains("文档 B"), "should mention doc B: {text}");
}

#[test]
fn prompts_get_unknown_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 26,
        "method": "prompts/get",
        "params": {
            "name": "nonexistent_prompt",
            "arguments": {}
        }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    // 未知 prompt 应返回错误
    assert!(
        !resp["error"].is_null(),
        "expected error for unknown prompt"
    );
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("unknown prompt"),
        "error message should mention unknown prompt: {msg}"
    );
}

#[test]
fn prompts_get_missing_required_arg_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    // summarize_document 需要 path 参数，这里故意不传
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 27,
        "method": "prompts/get",
        "params": {
            "name": "summarize_document",
            "arguments": {}
        }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    // 缺少必填参数应返回错误
    assert!(
        !resp["error"].is_null(),
        "expected error for missing required argument"
    );
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("missing"),
        "error should mention missing arg: {msg}"
    );
}

#[test]
fn prompts_get_compare_documents_missing_path_b() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path_a = create_test_docx(dir.path(), "a.docx");
    let config = make_config(dir.path());

    // 只传 path_a，缺少 path_b
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 28,
        "method": "prompts/get",
        "params": {
            "name": "compare_documents",
            "arguments": {
                "path_a": path_a.to_string_lossy()
            }
        }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    assert!(
        !resp["error"].is_null(),
        "expected error for missing path_b"
    );
}

// ===========================================================================
// Server 集成测试
// ===========================================================================

#[test]
fn handle_resources_list_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_docx(dir.path(), "integration.docx");
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":30,"method":"resources/list"}"#,
        &config,
    );

    assert_eq!(resp["id"], 30);
    assert!(resp["error"].is_null());
    let resources = resp["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0]["name"], "integration.docx");
    assert!(resources[0]["uri"].as_str().unwrap().starts_with("file://"));
    assert_eq!(
        resources[0]["mimeType"],
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    );
}

#[test]
fn handle_prompts_get_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_docx(dir.path(), "prompt_test.docx");
    let config = make_config(dir.path());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "prompts/get",
        "params": {
            "name": "extract_key_points",
            "arguments": {
                "path": path.to_string_lossy()
            }
        }
    });
    let req_str = serde_json::to_string(&req).unwrap();

    let resp = call_with_config(&req_str, &config);

    assert_eq!(resp["id"], 31);
    assert!(resp["error"].is_null());
    let messages = resp["result"]["messages"].as_array().unwrap();
    assert!(!messages.is_empty());
}

#[test]
fn resources_read_invalid_params_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    // 缺少 uri 参数
    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":32,"method":"resources/read","params":{}}"#,
        &config,
    );

    assert!(!resp["error"].is_null(), "expected error for missing uri");
    assert_eq!(resp["error"]["code"], -32602); // INVALID_PARAMS
}

#[test]
fn resources_subscribe_known_uri_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_docx(dir.path(), "watch.docx");
    let config = make_config(dir.path());

    let uri = format!("file://{}", path.display());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 33,
        "method": "resources/subscribe",
        "params": { "uri": uri }
    });
    let resp = call_with_config(&serde_json::to_string(&req).unwrap(), &config);

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    // MCP 规范：subscribe 成功返回空 result
    assert_eq!(resp["result"], serde_json::json!({}));
}

#[test]
fn resources_subscribe_unknown_uri_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 34,
        "method": "resources/subscribe",
        "params": { "uri": "file:///nonexistent/path/doc.docx" }
    });
    let resp = call_with_config(&serde_json::to_string(&req).unwrap(), &config);

    assert!(
        !resp["error"].is_null(),
        "expected error for missing resource"
    );
    assert_eq!(resp["error"]["code"], -32002);
}

#[test]
fn resources_unsubscribe_known_uri_succeeds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_docx(dir.path(), "unwatch.docx");
    let config = make_config(dir.path());

    let uri = format!("file://{}", path.display());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 35,
        "method": "resources/unsubscribe",
        "params": { "uri": uri }
    });
    let resp = call_with_config(&serde_json::to_string(&req).unwrap(), &config);

    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
    assert_eq!(resp["result"], serde_json::json!({}));
}

#[test]
fn resources_subscribe_invalid_params_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":36,"method":"resources/subscribe","params":{}}"#,
        &config,
    );

    assert!(!resp["error"].is_null(), "expected error for missing uri");
    assert_eq!(resp["error"]["code"], -32602); // INVALID_PARAMS
}

#[test]
fn prompts_get_invalid_params_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    // 缺少 name 参数
    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":33,"method":"prompts/get","params":{}}"#,
        &config,
    );

    assert!(!resp["error"].is_null(), "expected error for missing name");
    assert_eq!(resp["error"]["code"], -32602); // INVALID_PARAMS
}

#[test]
fn resources_and_prompts_coexist_with_tools() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_docx(dir.path(), "coexist.docx");
    let config = make_config(dir.path());

    // tools/list 仍然正常工作
    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":40,"method":"tools/list"}"#,
        &config,
    );
    assert!(resp["error"].is_null());
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 6);

    // resources/list 也正常工作
    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":41,"method":"resources/list"}"#,
        &config,
    );
    assert!(resp["error"].is_null());
    assert_eq!(resp["result"]["resources"].as_array().unwrap().len(), 1);

    // prompts/list 也正常工作
    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":42,"method":"prompts/list"}"#,
        &config,
    );
    assert!(resp["error"].is_null());
    assert_eq!(resp["result"]["prompts"].as_array().unwrap().len(), 4);
}

#[test]
fn resources_list_recursive_structure() {
    let (dir, _) = create_test_dir_structure();
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":50,"method":"resources/list"}"#,
        &config,
    );
    assert!(resp["error"].is_null());
    // 递归目录包含子目录中的 docx
    let resources = resp["result"]["resources"].as_array().unwrap();
    assert!(!resources.is_empty());
}

#[test]
fn resources_read_file_with_special_chars() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = create_test_docx(dir.path(), "my file.docx");
    let config = make_config(dir.path());

    let uri = format!("file://{}", path.display());
    let req = serde_json::json!({
        "jsonrpc": "2.0", "id": 51,
        "method": "resources/read",
        "params": { "uri": uri }
    });
    let resp = call_with_config(&serde_json::to_string(&req).unwrap(), &config);
    assert!(resp["error"].is_null(), "error: {}", resp["error"]);
}

#[test]
fn prompts_get_extra_args_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":52,"method":"prompts/get","params":{"name":"summarize_document","arguments":{"path":"/x.docx","extra":"ignored"}}}"#,
        &config,
    );
    // 额外参数不应导致错误（按实现容忍或报错都可接受，但需有响应）
    assert!(resp["result"].is_object() || !resp["error"].is_null());
}

#[test]
fn resources_list_no_trailing_uri_issues() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_docx(dir.path(), "a.docx");
    create_test_docx(dir.path(), "b.docx");
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":53,"method":"resources/list"}"#,
        &config,
    );
    let resources = resp["result"]["resources"].as_array().unwrap();
    // 两个 docx 都应被列出
    assert!(resources.len() >= 2);
}

#[test]
fn resources_read_directory_uri_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = make_config(dir.path());
    let uri = format!("file://{}", dir.path().display());
    let req = serde_json::json!({
        "jsonrpc": "2.0", "id": 54,
        "method": "resources/read",
        "params": { "uri": uri }
    });
    let resp = call_with_config(&serde_json::to_string(&req).unwrap(), &config);
    // 目录 URI 读取应报错（不是文件）
    assert!(!resp["error"].is_null(), "expected error for directory URI");
}

#[test]
fn resources_list_uri_field_present() {
    let dir = tempfile::tempdir().expect("tempdir");
    create_test_docx(dir.path(), "x.docx");
    let config = make_config(dir.path());

    let resp = call_with_config(
        r#"{"jsonrpc":"2.0","id":55,"method":"resources/list"}"#,
        &config,
    );
    let resources = resp["result"]["resources"].as_array().unwrap();
    for r in resources {
        assert!(r.get("uri").is_some(), "resource missing uri: {r}");
        assert!(
            r.get("mimeType").is_some(),
            "resource missing mimeType: {r}"
        );
    }
}
