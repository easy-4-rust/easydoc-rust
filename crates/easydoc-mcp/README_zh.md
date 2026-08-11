<a id="readme-top"></a>

<div align="center">

# easydoc-mcp

**MCP（Model Context Protocol）服务器，将 EasyDoc DOC/DOCX 能力暴露给 LLM 代理。**

[![Crates.io](https://img.shields.io/crates/v/easydoc-mcp)](https://crates.io/crates/easydoc-mcp)
[![docs.rs](https://img.shields.io/docsrs/easydoc-mcp)](https://docs.rs/easydoc-mcp)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#3-rust-基线与平台支持)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[定位](#1-项目定位与状态) · [工具](#2-工具) ·
[资源](#3-资源) · [提示模板](#4-提示模板) · [安装](#5-安装) ·
[Agent 配置](#6-agent-集成) · [协议](#7-协议) ·
[安全](#8-安全) · [许可证](#10-许可证)

</div>

---

> **当前版本**：`0.1.0-alpha.1`
> **MSRV**：Rust `1.88`
> **Edition**：`2024`
> **成熟度**：Alpha
> **最后核验**：2026-08-11

## 1. 项目定位与状态

### 1.1 是什么

**`easydoc-mcp` 是一个 MCP 服务器，将 `easydoc-rust` 的文档操作能力暴露给 LLM 代理。** 它通过 stdio（JSON-RPC 2.0，换行分隔）实现 Model Context Protocol，允许 Claude Code、Cursor 和其他兼容 MCP 的运行时代理读取、转换和创建 DOCX/DOC 文档。

| 维度 | 内容 |
|---|---|
| crate | `easydoc-mcp` |
| 当前版本 | `0.1.0-alpha.1` |
| MSRV / Edition | `1.88` / `2024` |
| 传输层 | stdio（JSON-RPC 2.0） |
| `unsafe` 策略 | `deny`（crate 级别） |
| 许可证 | `Apache-2.0` |

### 1.2 不是什么

- 不是独立 CLI 工具 -- 它作为 LLM 代理运行时的子进程运行。
- 不是 HTTP 服务器 -- 仅通过 stdin/stdout 通信。
- 不是通用文档转换器 -- 仅暴露 MCP 工具/资源/提示模板契约定义的操作。

### 1.3 能力概览

| MCP 能力 | 数量 | 说明 |
|---|---|---|
| 工具（tools） | 6 | 读取、提取、转换、创建文档 |
| 资源（resources） | 动态 | 目录扫描 + 路径穿越防护 |
| 提示模板（prompts） | 4 | 摘要、分析、提取、对比 |

## 2. 工具

服务器通过 `tools/list` 暴露 6 个工具。每个工具接受 JSON 输入并返回 JSON 输出。

### 2.1 工具定义

| 工具 | 说明 | 必填参数 | 可选参数 |
|---|---|---|---|
| `read_docx` | 以 4 种视图模式之一读取文档 | `path` | `mode`（plain/annotated/outline/stats，默认：annotated） |
| `read_table` | 将表格提取为 JSON 数组 | `path` | `sheet`（从零开始的表格索引；省略返回全部） |
| `read_docx_blocks` | 读取完整语义文档模型为 JSON | `path` | -- |
| `extract_images` | 将嵌入图片提取到目录 | `path`、`output_dir` | -- |
| `convert_to_markdown` | 将 DOCX/DOC 转换为 Markdown 文本 | `path` | `options.image_dir`、`options.front_matter` |
| `create_docx_from_data` | 从结构化数据创建 DOCX | `path`、`template`、`data` | -- |

### 2.2 工具输入 Schema

#### `read_docx`

```json
{
  "path": "/absolute/path/to/document.docx",
  "mode": "annotated"
}
```

视图模式：
- `plain` -- 纯文本，段落以换行分隔
- `annotated` -- 带结构标注，如 `[Heading1]`、`[Paragraph 1]`、`[Table 1: 3x4]`
- `outline` -- 仅标题，Markdown 风格的 `#` / `##`
- `stats` -- 块数和字数统计

#### `read_table`

```json
{
  "path": "/absolute/path/to/document.docx",
  "sheet": 0
}
```

返回 `{"table": [[...]]}`（指定表格）或 `{"tables": [[[...]], [[...]]]}`（全部表格）。

#### `read_docx_blocks`

```json
{
  "path": "/absolute/path/to/document.docx"
}
```

返回完整 `DocumentContent` 模型的 JSON 序列化（blocks、metadata、types）。

#### `extract_images`

```json
{
  "path": "/absolute/path/to/document.docx",
  "output_dir": "/absolute/path/to/output"
}
```

返回 `{"extracted": [...], "count": N}`。仅支持 DOCX（不支持旧版 DOC）。

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

返回 `{"markdown": "...", "warnings": N, "assets": N}`。

#### `create_docx_from_data`

支持三种模板类型：

**标题模板：**
```json
{
  "path": "/output.docx",
  "template": "heading",
  "data": { "text": "第一章", "level": 1 }
}
```

**表格模板：**
```json
{
  "path": "/output.docx",
  "template": "table",
  "data": {
    "rows": [
      ["姓名", "年龄", "邮箱"],
      ["张三", "30", "zhangsan@example.com"]
    ]
  }
}
```

**列表模板：**
```json
{
  "path": "/output.docx",
  "template": "list",
  "data": {
    "items": ["第一项", "第二项", "第三项"]
  }
}
```

## 3. 资源

服务器通过 `resources/list` 和 `resources/read` 暴露配置目录中的文档文件。

### 3.1 目录扫描

`DirectoryResourceProvider` 从根目录扫描匹配配置扩展名的文件（默认：所有文件）。文件以 `file://` URI 形式暴露。

```rust,ignore
use easydoc_mcp::DirectoryResourceProvider;

let provider = DirectoryResourceProvider::new("/path/to/docs")
    .recursive(true)
    .with_extensions(vec!["docx".into(), "doc".into()]);
```

### 3.2 内容读取

当通过 `resources/read` 读取资源时，服务器使用 `EasyDoc::view_as()` 将文档转换为标注文本并以 `text/markdown` 格式返回。

### 3.3 路径穿越防护

`DirectoryResourceProvider` 对所有 `file://` URI 读取执行路径穿越防护。解析后的路径必须在配置的根目录内（通过 `canonicalize()` 检查）。根目录外的路径返回 `null`。

## 4. 提示模板

服务器通过 `prompts/list` 和 `prompts/get` 暴露 4 个内置提示模板。

| 提示模板 | 说明 | 必填参数 | 可选参数 |
|---|---|---|---|
| `summarize_document` | 生成文档简洁摘要 | `path` | `max_length`（默认：500） |
| `analyze_table_data` | 分析文档中的表格数据 | `path` | `table_index`（从零开始） |
| `extract_key_points` | 从文档提取关键要点 | `path` | -- |
| `compare_documents` | 对比两份文档的差异 | `path_a`、`path_b` | -- |

每个提示模板使用 `EasyDoc::view_as()`（Annotated 模式）或 `EasyDoc::load()`（表格分析）读取文档，然后为 LLM 渲染结构化消息。

## 5. 安装

### 5.1 从 crates.io

```bash
cargo install easydoc-mcp
```

### 5.2 从 workspace

```bash
cargo install --path crates/easydoc-mcp
```

### 5.3 作为依赖

```toml
[dependencies]
easydoc-mcp = "0.1.0-alpha.1"
```

## 6. Agent 集成

### 6.1 Claude Code

添加到 `.claude/settings.json`：

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

添加到 `.cursor/mcp.json`：

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

### 6.3 通用 MCP 客户端

服务器通过换行分隔的 stdin/stdout 使用 JSON-RPC 2.0 通信。任何兼容 MCP 的客户端都可以将其作为子进程启动：

```bash
# 启动服务器
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | easydoc-mcp
```

### 6.4 编程式使用

```rust
use easydoc_mcp::server;

// 处理单条 JSON-RPC 消息
let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
let response = server::handle_raw(request).unwrap();
println!("{response:?}");
```

## 7. 协议

服务器实现以下 MCP 方法：

| 方法 | 说明 |
|---|---|
| `initialize` | 返回服务器信息和能力（tools、resources、prompts） |
| `tools/list` | 返回 6 个工具定义（含 JSON Schema 输入规范） |
| `tools/call` | 分发到请求的工具处理器 |
| `resources/list` | 返回配置目录中的可用文档资源 |
| `resources/read` | 按 URI 读取资源（转换为标注文本） |
| `prompts/list` | 返回 4 个提示模板定义 |
| `prompts/get` | 使用提供的参数渲染提示模板 |
| `ping` | 健康检查（返回 `{}`） |
| `notifications/initialized` | 客户端确认（不发送响应） |

## 8. 安全

| 关注点 | 缓解措施 |
|---|---|
| 路径穿越 | `DirectoryResourceProvider` 使用 `canonicalize()` + 根目录前缀检查 |
| 文件访问范围 | 仅限配置根目录内的 `file://` URI |
| 输入验证 | 执行前验证所有工具参数 |
| `unsafe` 代码 | crate 级别 `deny` |
| 资源限制 | 通过 `easydoc-ooxml` 限制 ZIP bomb 防护（10K 条目、256MB/条目、1GB 总计） |

## 9. 构建与测试

```bash
cargo check -p easydoc-mcp
cargo test -p easydoc-mcp
cargo clippy -p easydoc-mcp -- -D warnings
cargo doc -p easydoc-mcp --no-deps
```

## 10. 许可证

Apache-2.0 -- 详见 [LICENSE](../../LICENSE)。

---

<div align="center">

[返回顶部](#readme-top) · [docs.rs](https://docs.rs/easydoc-mcp) · [crates.io](https://crates.io/crates/easydoc-mcp) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
