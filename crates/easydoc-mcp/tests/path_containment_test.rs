//! 工具路径信任边界测试。
//!
//! `tools/call` 的所有路径参数（`path` / `output_dir`）必须解析到服务根目录
//! （`EASYDOC_MCP_ROOT`，缺省当前目录）内：绝对路径越界、`..` 逃逸、
//! 符号链接逃逸都必须被拒绝，而不是打开根目录之外的文件。

use std::path::Path;
use std::sync::Mutex;

/// 根目录覆盖是全局状态，用互斥锁保证"设根 → 发请求"窗口内
/// 没有并发测试改写；锁随 `_guard` 持有到测试结束。
static ROOT_GUARD: Mutex<()> = Mutex::new(());

fn set_server_root(dir: &Path) -> std::sync::MutexGuard<'static, ()> {
    let guard = ROOT_GUARD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    easydoc_mcp::tools::set_server_root_for_testing(dir);
    guard
}

fn call(raw: &str) -> serde_json::Value {
    let response_str = easydoc_mcp::server::handle_raw(raw)
        .expect("handle_raw failed")
        .expect("expected a response (got notification)");
    serde_json::from_str(&response_str).expect("response is not valid JSON")
}

fn tool_error_text(resp: &serde_json::Value) -> String {
    assert_eq!(
        resp["result"]["isError"], true,
        "expected tool error: {resp}"
    );
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn create_docx(path: &Path) {
    easydoc::EasyDoc::write_content(
        &easydoc_core::DocumentContent {
            metadata: easydoc_core::DocumentMeta::default(),
            blocks: vec![easydoc_core::DocumentBlock::Paragraph(vec![
                easydoc_core::DocumentTextRun {
                    text: "inside doc".into(),
                    ..easydoc_core::DocumentTextRun::default()
                },
            ])],
        },
        path,
    )
    .expect("write docx");
}

#[test]
fn absolute_path_outside_root_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_docx = outside.path().join("outside.docx");
    create_docx(&outside_docx);
    let _guard = set_server_root(root.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"read_docx","arguments":{{"path":"{}"}}}}}}"#,
        outside_docx.display()
    );
    let text = tool_error_text(&call(&req));
    assert!(
        text.contains("escapes server root"),
        "unexpected error: {text}"
    );
}

#[test]
fn dotdot_escape_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_docx = outside.path().join("sibling.docx");
    create_docx(&outside_docx);
    let _guard = set_server_root(root.path());

    // `root/../<tmpdir>/sibling.docx` 归一后落在根目录之外
    let sneaky = root.path().join("..").join(&outside_docx);
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"read_docx","arguments":{{"path":"{}"}}}}}}"#,
        sneaky.display()
    );
    let text = tool_error_text(&call(&req));
    assert!(
        text.contains("escapes server root"),
        "unexpected error: {text}"
    );
}

#[test]
#[cfg(unix)]
fn symlink_escape_is_rejected() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let outside_docx = outside.path().join("real.docx");
    create_docx(&outside_docx);
    let link = root.path().join("link.docx");
    symlink(&outside_docx, &link).expect("create symlink");
    let _guard = set_server_root(root.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"read_docx","arguments":{{"path":"{}"}}}}}}"#,
        link.display()
    );
    let text = tool_error_text(&call(&req));
    assert!(
        text.contains("escapes server root"),
        "unexpected error: {text}"
    );
}

#[test]
fn extract_images_output_dir_outside_root_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let docx = root.path().join("doc.docx");
    create_docx(&docx);
    let _guard = set_server_root(root.path());

    let req = format!(
        r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"extract_images","arguments":{{"path":"{}","output_dir":"{}"}}}}}}"#,
        docx.display(),
        outside.path().join("images").display()
    );
    let text = tool_error_text(&call(&req));
    assert!(
        text.contains("escapes server root"),
        "unexpected error: {text}"
    );
}

#[test]
fn relative_path_inside_root_still_works() {
    let root = tempfile::tempdir().unwrap();
    let docx = root.path().join("doc.docx");
    create_docx(&docx);
    let _guard = set_server_root(root.path());

    let req = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"read_docx","arguments":{"path":"doc.docx"}}}"#;
    let resp = call(req);
    assert!(
        resp["result"]["isError"] == false,
        "unexpected error: {resp}"
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("inside doc"), "unexpected text: {text}");
}
