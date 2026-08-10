//! `EasyDoc` MCP 服务器：通过 Model Context Protocol 暴露 `EasyDoc` 能力。
//!
//! 本 crate 实现了一个 MCP（Model Context Protocol）服务器，允许
//! LLM 代理通过标准 MCP 接口读取、转换和创建 DOCX/DOC 文档。
//!
//! # 架构
//!
//! ```text
//! LLM Agent
//!     |
//!     | JSON-RPC 2.0 over stdio
//!     v
//! ┌──────────────┐
//! │  transport/   │  stdio transport（换行分隔 JSON）
//! │  stdio.rs     │
//! └──────┬───────┘
//!        v
//! ┌──────────────┐
//! │  server.rs    │  MCP 协议分发（initialize, tools, resources, prompts）
//! └──────┬───────┘
//!        v
//! ┌──────────────────────────────────────────┐
//! │  tools.rs / resources.rs / prompts.rs    │  具体能力实现
//! └──────┬───────────────────────────────────┘
//!        v
//! ┌──────────────┐
//! │  easydoc      │  `EasyDoc` 静态方法（read, write, convert）
//! └──────────────┘
//! ```
//!
//! # 能力
//!
//! ## 工具（tools）
//!
//! | 工具名 | 描述 |
//! |--------|------|
//! | `read_docx` | 以多种视图模式读取文档 |
//! | `read_table` | 提取表格为 JSON |
//! | `read_docx_blocks` | 读取语义文档模型为 JSON |
//! | `extract_images` | 提取嵌入图片到目录 |
//! | `convert_to_markdown` | DOCX/DOC 转 Markdown |
//! | `create_docx_from_data` | 从结构化数据创建 DOCX |
//!
//! ## 资源（resources）
//!
//! 通过 `resources/list` 暴露目录中的文档文件，通过 `resources/read` 读取
//! 文档内容（自动转为 Markdown 文本）。
//!
//! ## 提示模板（prompts）
//!
//! 内置四个 prompt 模板：`summarize_document`、`analyze_table_data`、
//! `extract_key_points`、`compare_documents`。
//!
//! # 用法
//!
//! ```no_run
//! // 在 stdin/stdout 上运行 MCP 服务器（阻塞直到 EOF）。
//! easydoc_mcp::run_stdio_server().expect("server failed");
//! ```
//!
//! 编程式使用，调用 `handle_raw` 处理单条消息：
//!
//! ```
//! use easydoc_mcp::server;
//!
//! let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
//! let response = server::handle_raw(request).unwrap();
//! assert!(response.unwrap().contains("easydoc-mcp"));
//! ```

#![deny(unsafe_code)]

pub mod prompts;
pub mod protocol;
pub mod resources;
pub mod server;
pub mod tools;
pub mod transport;

pub use prompts::{
    BuiltinPrompts, Prompt, PromptArgument, PromptContent, PromptMessage, PromptRenderer,
};
pub use resources::{DirectoryResourceProvider, Resource, ResourceContent, ResourceProvider};

/// 在 stdin/stdout 上运行 MCP 服务器，阻塞直到 EOF。
///
/// 这是将 `easydoc-mcp` 作为 LLM 代理运行时子进程运行的主要入口。
///
/// # 错误
///
/// 在 stdio 通信发生致命 I/O 错误时返回。
pub fn run_stdio_server() -> anyhow::Result<()> {
    transport::stdio::run_stdio_server()
}
