//! MCP 资源（resources）支持：暴露文档资源给 LLM 客户端。
//!
//! MCP 的 resources 能力允许服务器向客户端暴露可按 URI 读取的命名资源。
//! 本模块定义资源元数据、内容格式、提供者 trait，以及一个基于目录扫描的
//! 默认实现 `DirectoryResourceProvider`。
//!
//! # 安全
//!
//! `DirectoryResourceProvider` 在读取 `file://` URI 时执行路径穿越防护，
//! 确保解析后的路径落在配置的根目录内。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// MCP 资源元数据。
///
/// 描述一个可通过 URI 读取的资源，在 `resources/list` 中返回。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    /// 资源唯一标识符（URI 格式，如 `file:///path/to/doc.docx`）。
    pub uri: String,
    /// 人类可读的资源名称。
    pub name: String,
    /// 资源描述。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME 类型。
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// 资源内容项（`resources/read` 返回）。
///
/// 一个资源可以返回多段内容（如文本 + 关联图片）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceContent {
    /// 资源 URI。
    pub uri: String,
    /// 内容的 MIME 类型。
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// 文本内容（当资源可表示为文本时）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64 编码的二进制内容（当资源为二进制时）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// 资源提供者 trait。
///
/// 实现此 trait 以向 MCP 客户端暴露文档资源。提供者负责
/// 资源发现（列出）和内容读取。
pub trait ResourceProvider: Send + Sync {
    /// 列出所有可用资源。
    fn list(&self) -> Vec<Resource>;

    /// 读取指定 URI 的资源内容。
    ///
    /// # 返回
    ///
    /// - `Ok(Some(contents))` — 资源存在且已读取。
    /// - `Ok(None)` — 资源不存在。
    /// - `Err(...)` — 读取过程中发生错误。
    fn read(&self, uri: &str) -> anyhow::Result<Option<Vec<ResourceContent>>>;
}

/// 基于目录扫描的资源提供者。
///
/// 从指定根目录递归扫描文件，将匹配扩展名的文件暴露为 MCP 资源。
/// 读取时自动调用 `EasyDoc` 将文档转换为 Markdown 文本。
pub struct DirectoryResourceProvider {
    /// 扫描根目录。
    root: PathBuf,
    /// 是否递归子目录。
    recursive: bool,
    /// 文件扩展名白名单（不含前导点，如 `["docx", "doc"]`）。
    extensions: Vec<String>,
}

impl DirectoryResourceProvider {
    /// 创建新的目录资源提供者。
    ///
    /// 默认不递归子目录，不限制扩展名。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            recursive: false,
            extensions: Vec::new(),
        }
    }

    /// 设置是否递归子目录。
    #[must_use]
    pub fn recursive(mut self, enabled: bool) -> Self {
        self.recursive = enabled;
        self
    }

    /// 设置文件扩展名白名单。
    ///
    /// 扩展名不含前导点，如 `["docx", "doc"]`。
    #[must_use]
    pub fn with_extensions(mut self, exts: Vec<String>) -> Self {
        self.extensions = exts;
        self
    }
}

impl ResourceProvider for DirectoryResourceProvider {
    fn list(&self) -> Vec<Resource> {
        let mut resources = Vec::new();
        collect_files(&self.root, self.recursive, &self.extensions, &mut resources);
        resources
    }

    fn read(&self, uri: &str) -> anyhow::Result<Option<Vec<ResourceContent>>> {
        let Some(path) = safe_resolve_path(&self.root, uri) else {
            return Ok(None);
        };

        if !path.exists() {
            return Ok(None);
        }

        // 调用 EasyDoc 将文档转为 Markdown 文本。
        let markdown = easydoc::EasyDoc::view_as(&path, &easydoc_reader::ViewMode::Annotated)
            .map_err(|e| anyhow::anyhow!("failed to read resource '{uri}': {e:#}"))?;

        Ok(Some(vec![ResourceContent {
            uri: uri.to_string(),
            mime_type: Some("text/markdown".into()),
            text: Some(markdown),
            blob: None,
        }]))
    }
}

// ---------------------------------------------------------------------------
// 内部辅助函数
// ---------------------------------------------------------------------------

/// 递归收集目录下的文件，构建资源列表。
fn collect_files(dir: &Path, recursive: bool, extensions: &[String], out: &mut Vec<Resource>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            if recursive {
                collect_files(&path, recursive, extensions, out);
            }
            continue;
        }

        // 过滤扩展名
        if !extensions.is_empty() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                continue;
            }
        }

        let uri = format!("file://{}", path.display());
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let mime_type = guess_mime_type(&path);

        out.push(Resource {
            uri,
            name,
            description: None,
            mime_type,
        });
    }
}

/// 根据文件扩展名推断 MIME 类型。
fn guess_mime_type(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "docx" => {
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document".into())
        }
        "doc" => Some("application/msword".into()),
        "md" | "markdown" => Some("text/markdown".into()),
        "txt" => Some("text/plain".into()),
        "pdf" => Some("application/pdf".into()),
        "html" | "htm" => Some("text/html".into()),
        _ => Some("application/octet-stream".into()),
    }
}

/// 安全解析 `file://` URI 并确保路径在根目录内（防路径穿越）。
///
/// # 返回
///
/// - `Some(canonical_path)` — 路径有效且在根目录内。
/// - `None` — URI 格式无效、路径不存在或路径穿越。
fn safe_resolve_path(root: &Path, uri: &str) -> Option<PathBuf> {
    let path_str = uri.strip_prefix("file://")?;
    let path = PathBuf::from(path_str);

    // 如果路径不存在，canonicalize 会失败——此时返回 None。
    let canonical_root = root.canonicalize().ok()?;
    let canonical_path = path.canonicalize().ok()?;

    // 校验路径在根目录下。
    if canonical_path.starts_with(&canonical_root) {
        Some(canonical_path)
    } else {
        None
    }
}
