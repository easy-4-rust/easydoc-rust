//! MCP 根目录可配置性测试。
//!
//! `DirectoryResourceProvider` 的扫描根目录可通过
//! `default_config_with_root` / `ServerConfig::new` 配置（roadmap 0.1.0 MCP 项）。

use easydoc_mcp::server::{ServerConfig, default_config_with_root};

/// 根目录可配置：`default_config_with_root(tempdir)` 只列出该目录下的文档。
#[test]
fn root_directory_configurable() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("report.docx"), b"PK\x03\x04fake").expect("write file");
    std::fs::write(dir.path().join("notes.txt"), b"ignored").expect("write file");

    let config = default_config_with_root(dir.path());
    let resources = config.provider.list();
    let names: Vec<&str> = resources.iter().map(|r| r.uri.as_str()).collect();
    // 只应扫描到 report.docx（扩展名白名单），且 URI 指向配置的根目录
    assert_eq!(resources.len(), 1, "resources: {names:?}");
    assert!(names[0].contains("report.docx"), "{names:?}");
    assert!(names[0].starts_with("file://"), "{names:?}");
    assert!(
        names[0].contains(&dir.path().to_string_lossy().into_owned()),
        "{names:?}"
    );
}

/// 自定义 provider 可通过 `ServerConfig::new` 完全替换。
#[test]
fn custom_provider_via_server_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let provider = std::sync::Arc::new(
        easydoc_mcp::resources::DirectoryResourceProvider::new(dir.path())
            .recursive(false)
            .with_extensions(vec!["docx".into()]),
    );
    let config = ServerConfig::new(
        provider,
        std::sync::Arc::new(easydoc_mcp::prompts::BuiltinPrompts::new()),
    );
    let resources = config.provider.list();
    assert!(resources.is_empty(), "空目录应无资源");
}
