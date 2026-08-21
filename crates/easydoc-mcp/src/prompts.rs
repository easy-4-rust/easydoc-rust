//! MCP 提示模板（prompts）支持：预定义的 prompt 模板供 LLM 调用。
//!
//! MCP 的 prompts 能力允许服务器向客户端暴露可参数化的提示模板。
//! LLM 可以通过 `prompts/list` 发现可用模板，再通过 `prompts/get`
//! 渲染具体的消息序列。
//!
//! 本模块内置四个文档处理相关的 prompt：
//!
//! | 名称 | 描述 |
//! |------|------|
//! | `summarize_document` | 生成文档的简洁摘要 |
//! | `analyze_table_data` | 分析文档中的表格数据 |
//! | `extract_key_points` | 从文档提取关键要点 |
//! | `compare_documents` | 对比两份文档的差异 |

use serde::{Deserialize, Serialize};

/// Prompt 参数定义。
///
/// 描述 prompt 模板接受的一个参数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptArgument {
    /// 参数名称。
    pub name: String,
    /// 参数描述。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否为必填参数。
    #[serde(default)]
    pub required: bool,
}

/// Prompt 模板定义。
///
/// 在 `prompts/list` 中返回，描述一个可用的提示模板。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Prompt {
    /// 模板名称（唯一标识）。
    pub name: String,
    /// 模板描述。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 模板参数列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<PromptArgument>,
}

/// 渲染后的提示消息。
///
/// 由 `prompts/get` 返回，代表对话中的一条消息。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptMessage {
    /// 消息角色：`"user"` 或 `"assistant"`。
    pub role: String,
    /// 消息内容。
    pub content: PromptContent,
}

/// 消息内容类型。
///
/// 目前仅支持文本类型，未来可扩展图片等。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum PromptContent {
    /// 文本内容。
    #[serde(rename = "text")]
    Text {
        /// 文本内容。
        text: String,
    },
}

/// Prompt 渲染器 trait。
///
/// 实现此 trait 以向 MCP 客户端暴露 prompt 模板。
pub trait PromptRenderer: Send + Sync {
    /// 返回所有可用的 prompt 模板。
    fn list(&self) -> Vec<Prompt>;

    /// 渲染指定名称的 prompt。
    ///
    /// # 参数
    ///
    /// - `name` — prompt 模板名称。
    /// - `arguments` — JSON 对象，键为参数名。
    ///
    /// # 返回
    ///
    /// 渲染后的消息序列，可直接用于 LLM 对话。
    fn render(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<Vec<PromptMessage>>;
}

/// 内置 prompts 集合。
///
/// 提供四个文档处理相关的预定义 prompt 模板。
pub struct BuiltinPrompts;

impl BuiltinPrompts {
    /// 创建内置 prompts 集合。
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for BuiltinPrompts {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptRenderer for BuiltinPrompts {
    fn list(&self) -> Vec<Prompt> {
        vec![
            Prompt {
                name: "summarize_document".into(),
                description: Some("生成文档的简洁摘要".into()),
                arguments: vec![
                    PromptArgument {
                        name: "path".into(),
                        description: Some("DOCX 文件路径".into()),
                        required: true,
                    },
                    PromptArgument {
                        name: "max_length".into(),
                        description: Some("最大字数".into()),
                        required: false,
                    },
                ],
            },
            Prompt {
                name: "analyze_table_data".into(),
                description: Some("分析文档中的表格数据".into()),
                arguments: vec![
                    PromptArgument {
                        name: "path".into(),
                        description: Some("DOCX 文件路径".into()),
                        required: true,
                    },
                    PromptArgument {
                        name: "table_index".into(),
                        description: Some("表格索引（0-based）".into()),
                        required: false,
                    },
                ],
            },
            Prompt {
                name: "extract_key_points".into(),
                description: Some("从文档提取关键要点".into()),
                arguments: vec![PromptArgument {
                    name: "path".into(),
                    description: Some("DOCX 文件路径".into()),
                    required: true,
                }],
            },
            Prompt {
                name: "compare_documents".into(),
                description: Some("对比两份文档的差异".into()),
                arguments: vec![
                    PromptArgument {
                        name: "path_a".into(),
                        description: Some("第一份文档路径".into()),
                        required: true,
                    },
                    PromptArgument {
                        name: "path_b".into(),
                        description: Some("第二份文档路径".into()),
                        required: true,
                    },
                ],
            },
        ]
    }

    fn render(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<Vec<PromptMessage>> {
        match name {
            "summarize_document" => render_summarize_document(&arguments),
            "analyze_table_data" => render_analyze_table_data(&arguments),
            "extract_key_points" => render_extract_key_points(&arguments),
            "compare_documents" => render_compare_documents(&arguments),
            _ => Err(anyhow::anyhow!("unknown prompt: {name}")),
        }
    }
}

// ---------------------------------------------------------------------------
// 各 prompt 的渲染实现
// ---------------------------------------------------------------------------

/// 从参数中提取必填的 `path` 字段。
fn require_path_arg(args: &serde_json::Value) -> anyhow::Result<&str> {
    args.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required argument: path"))
}

/// 将文本截断到指定字符数（按 Unicode 字符边界）。
fn truncate_to_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{truncated}...")
    }
}

/// 以 Annotated 模式读取文档内容。
fn read_document(path: &str) -> anyhow::Result<String> {
    easydoc::EasyDoc::view_as(path, &easydoc_reader::ViewMode::Annotated)
        .map_err(|e| anyhow::anyhow!("failed to read document '{path}': {e:#}"))
}

/// 渲染 `summarize_document` prompt。
fn render_summarize_document(args: &serde_json::Value) -> anyhow::Result<Vec<PromptMessage>> {
    let path = require_path_arg(args)?;
    let max_length = args
        .get("max_length")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(500);

    let content = read_document(path)?;
    let truncated = truncate_to_chars(&content, max_length as usize);

    Ok(vec![PromptMessage {
        role: "user".into(),
        content: PromptContent::Text {
            text: format!(
                "请对以下文档内容生成不超过 {max_length} 字的摘要：\n\n---\n\n{truncated}\n\n---",
            ),
        },
    }])
}

/// 渲染 `analyze_table_data` prompt。
fn render_analyze_table_data(args: &serde_json::Value) -> anyhow::Result<Vec<PromptMessage>> {
    let path = require_path_arg(args)?;
    let table_index = args.get("table_index").and_then(serde_json::Value::as_u64);

    // 读取表格数据。
    let doc = easydoc::EasyDoc::load(path)?;
    let tables: Vec<&easydoc_core::DocumentTable> = doc
        .blocks
        .iter()
        .filter_map(|block| {
            if let easydoc_core::DocumentBlock::Table(table) = block {
                Some(table)
            } else {
                None
            }
        })
        .collect();

    if tables.is_empty() {
        return Ok(vec![PromptMessage {
            role: "user".into(),
            content: PromptContent::Text {
                text: "文档中没有找到表格。".into(),
            },
        }]);
    }

    let table_text = if let Some(idx) = table_index {
        let i = idx as usize;
        if i >= tables.len() {
            return Err(anyhow::anyhow!(
                "table_index {i} out of range (document has {} table(s))",
                tables.len()
            ));
        }
        table_to_text(tables[i])
    } else {
        tables
            .iter()
            .enumerate()
            .map(|(i, t)| format!("=== 表格 {} ===\n{}", i + 1, table_to_text(t)))
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    Ok(vec![PromptMessage {
        role: "user".into(),
        content: PromptContent::Text {
            text: format!("请分析以下文档中的表格数据：\n\n{table_text}"),
        },
    }])
}

/// 将表格转为可读文本。
fn table_to_text(table: &easydoc_core::DocumentTable) -> String {
    table
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| {
                    cell.blocks
                        .iter()
                        .filter_map(|block| match block {
                            easydoc_core::DocumentBlock::Paragraph(runs)
                            | easydoc_core::DocumentBlock::Heading { runs, .. } => {
                                Some(runs.iter().map(|r| r.text.as_str()).collect::<String>())
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 渲染 `extract_key_points` prompt。
fn render_extract_key_points(args: &serde_json::Value) -> anyhow::Result<Vec<PromptMessage>> {
    let path = require_path_arg(args)?;
    let content = read_document(path)?;

    Ok(vec![PromptMessage {
        role: "user".into(),
        content: PromptContent::Text {
            text: format!(
                "请从以下文档中提取关键要点，以要点列表形式呈现：\n\n---\n\n{content}\n\n---"
            ),
        },
    }])
}

/// 渲染 `compare_documents` prompt。
fn render_compare_documents(args: &serde_json::Value) -> anyhow::Result<Vec<PromptMessage>> {
    let path_a = args
        .get("path_a")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required argument: path_a"))?;
    let path_b = args
        .get("path_b")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required argument: path_b"))?;

    let content_a = read_document(path_a)?;
    let content_b = read_document(path_b)?;

    Ok(vec![PromptMessage {
        role: "user".into(),
        content: PromptContent::Text {
            text: format!(
                "请对比以下两份文档的差异，包括内容变化、结构变化和关键信息的增删：\n\n\
                 === 文档 A ===\n{content_a}\n\n\
                 === 文档 B ===\n{content_b}"
            ),
        },
    }])
}
