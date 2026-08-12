# MCP 服务器与 LLM Agent 集成设计

- **日期**：2026-08-11
- **作者**：ZCode Agent（协同设计）
- **状态**：已实现基础框架，本文档为补全设计
- **依赖**：easydoc-mcp（server/tools/resources/prompts/transport）、easydoc-core、easydoc-reader、easydoc-markdown、easydoc-writer

## 1. 目标与范围

通过 Model Context Protocol（MCP）将 easydoc-rust 的全部文档操作能力暴露给 LLM Agent，使 AI 代理能够读取、分析、转换和创建 DOCX/DOC 文档。

**核心需求**：

1. 实现 MCP 2024-11-05 协议，支持 `initialize`、`tools/list`、`tools/call`、`resources/list`、`resources/read`、`prompts/list`、`prompts/get`。
2. 6 个内置工具：`read_docx`、`read_table`、`read_docx_blocks`、`extract_images`、`convert_to_markdown`、`create_docx_from_data`。
3. 资源发现：扫描目录中的 docx/doc 文件，通过 `file://` URI 暴露。
4. 4 个内置 prompt 模板：`summarize_document`、`analyze_table_data`、`extract_key_points`、`compare_documents`。
5. stdio 传输层：JSON-RPC 2.0 over stdin/stdout（换行分隔）。
6. 路径安全：资源读取时防路径穿越（canonicalize + starts_with 校验）。

**非目标**：

- 不提供 HTTP/SSE 传输层（当前仅 stdio）。
- 不支持 MCP 的 sampling 能力（LLM 调用 back）。
- 不提供资源订阅（subscribe）通知。
- 不支持自定义工具注册（工具列表硬编码）。

## 2. 总体架构

```
┌────────────────────────────────────────────────────────────────┐
│                       LLM Agent (Claude / GPT / ...)          │
│                                                                │
│  JSON-RPC 2.0 requests over stdin                             │
└──────────────────────────┬─────────────────────────────────────┘
                           │ stdin (换行分隔 JSON)
                           ▼
┌────────────────────────────────────────────────────────────────┐
│                    easydoc-mcp                                 │
│                                                                │
│  ┌──────────────┐                                              │
│  │ transport/   │  stdio.rs — 读 stdin / 写 stdout             │
│  │  stdio.rs    │  换行分隔 JSON-RPC 2.0                       │
│  └──────┬───────┘                                              │
│         ▼                                                      │
│  ┌──────────────┐                                              │
│  │ server.rs    │  协议分发：                                   │
│  │              │  initialize → ServerCapabilities              │
│  │              │  tools/list  → tool_definitions()             │
│  │              │  tools/call  → tools::call_tool()             │
│  │              │  resources/list → provider.list()             │
│  │              │  resources/read → provider.read()             │
│  │              │  prompts/list  → renderer.list()              │
│  │              │  prompts/get   → renderer.render()            │
│  └──────┬───────┘                                              │
│         │                                                      │
│  ┌──────┴──────────────────────────────────────────────┐       │
│  │                                                      │       │
│  ▼              ▼                    ▼                  ▼       │
│ tools.rs    resources.rs        prompts.rs          protocol.rs │
│                                                                │
│  6 个工具    DirectoryResource   BuiltinPrompts     JSON-RPC   │
│  handler     Provider           (4 个模板)          类型定义    │
└────────┬───────────────────────────────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────────────────────────────┐
│  easydoc (facade)  +  easydoc-reader  +  easydoc-markdown      │
│  +  easydoc-writer  +  easydoc-core                            │
└────────────────────────────────────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 模块结构

```
easydoc-mcp/src/
├── lib.rs              crate 入口 + run_stdio_server()
├── server.rs           MCP 协议分发 + ServerConfig
├── protocol.rs         JSON-RPC 2.0 类型定义
├── tools.rs            6 个工具定义 + dispatch
├── resources.rs        ResourceProvider trait + DirectoryResourceProvider
├── prompts.rs          PromptRenderer trait + BuiltinPrompts
└── transport/
    ├── mod.rs
    └── stdio.rs        stdin/stdout 传输层
```

### 3.2 各组件职责

| 组件 | 职责 |
|---|---|
| `server.rs` | 消息路由：解析 JSON-RPC → 分发到对应 handler → 返回响应 |
| `protocol.rs` | 类型：`JsonRpcRequest`、`JsonRpcResponse`、`InitializeResult`、`ServerCapabilities`、`ToolCallParams`、`ToolCallResult` |
| `tools.rs` | 工具定义（`tool_definitions()`）+ 实现（`call_tool()`） |
| `resources.rs` | `ResourceProvider` trait + `DirectoryResourceProvider`（目录扫描 + 路径安全） |
| `prompts.rs` | `PromptRenderer` trait + `BuiltinPrompts`（4 个内置模板） |
| `transport/stdio.rs` | stdin 逐行读取 → `server::handle_raw()` → stdout 写入 |

### 3.3 工具清单

| 工具名 | 功能 | 输入参数 | 输出 |
|---|---|---|---|
| `read_docx` | 多模式读取文档 | `path`, `mode` (plain/annotated/outline/stats) | 文本内容 |
| `read_table` | 提取表格为 JSON | `path`, `sheet` (可选) | 表格数据 |
| `read_docx_blocks` | 语义模型 JSON | `path` | DocumentContent JSON |
| `extract_images` | 提取嵌入图片 | `path`, `output_dir` | 提取的文件路径列表 |
| `convert_to_markdown` | 转 Markdown | `path`, `options` (image_dir, front_matter) | Markdown 文本 |
| `create_docx_from_data` | 创建 DOCX | `path`, `template` (heading/table/list), `data` | 创建结果 |

### 3.4 Prompt 模板

| 模板名 | 功能 | 参数 |
|---|---|---|
| `summarize_document` | 文档摘要 | `path`, `max_length` (可选) |
| `analyze_table_data` | 表格分析 | `path`, `table_index` (可选) |
| `extract_key_points` | 关键要点提取 | `path` |
| `compare_documents` | 文档对比 | `path_a`, `path_b` |

## 4. 关键数据流

### 4.1 工具调用流程

```
LLM Agent
    │
    │ JSON-RPC: {"method":"tools/call","params":{"name":"read_docx","arguments":{"path":"/tmp/report.docx","mode":"annotated"}}}
    │
    ▼
transport/stdio.rs::run_stdio_server()
    │ 读取 stdin 一行
    ▼
server::handle_raw(raw)
    │ 解析 JSON → JsonRpcRequest
    ▼
server::handle_request_with_config(request, config)
    │ 匹配 method = "tools/call"
    ▼
server::handle_tools_call(id, params)
    │ 解析 ToolCallParams { name, arguments }
    ▼
tools::call_tool("read_docx", args)
    │
    ▼
tools::handle_read_docx(args)
    │ EasyDoc::view_as(path, &ViewMode::Annotated)
    │
    ▼
ToolCallResult::text(json_string)
    │
    ▼
JsonRpcResponse::success(id, result)
    │
    ▼
stdout 写入 JSON 字符串 + 换行
```

### 4.2 资源读取流程

```
LLM Agent
    │
    │ JSON-RPC: {"method":"resources/read","params":{"uri":"file:///tmp/report.docx"}}
    │
    ▼
server::handle_resources_read(id, params, config)
    │
    ▼
DirectoryResourceProvider::read(uri)
    │
    ├── safe_resolve_path(root, uri)
    │   │
    │   ├── strip "file://" prefix
    │   ├── path.canonicalize()
    │   ├── root.canonicalize()
    │   └── canonical_path.starts_with(canonical_root)  ← 路径穿越防护
    │
    ├── EasyDoc::view_as(path, &ViewMode::Annotated)
    │
    ▼
ResourceContent { uri, mime_type: "text/markdown", text: content }
```

### 4.3 Prompt 渲染流程

```
LLM Agent
    │
    │ JSON-RPC: {"method":"prompts/get","params":{"name":"summarize_document","arguments":{"path":"/tmp/report.docx","max_length":200}}}
    │
    ▼
BuiltinPrompts::render("summarize_document", arguments)
    │
    ▼
render_summarize_document(args)
    │
    ├── read_document(path)  → Annotated 文本
    ├── truncate_to_chars(text, max_length)
    │
    ▼
vec![PromptMessage { role: "user", content: Text("请对以下文档内容生成不超过 200 字的摘要：...") }]
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | 仅支持 stdio 传输 | 最简单、最通用（所有 LLM Agent 都支持子进程） | 无法支持 Web 部署 |
| 2 | 工具列表硬编码 | 当前 6 个工具覆盖核心场景 | 新增工具需修改代码 |
| 3 | `ResourceProvider` trait 抽象 | 允许自定义资源来源（非目录扫描） | 增加一层间接 |
| 4 | `PromptRenderer` trait 抽象 | 允许自定义 prompt 模板 | 当前仅 `BuiltinPrompts` 实现 |
| 5 | `ServerConfig` 可配置 | 支持编程式使用（测试、嵌入其他服务） | 增加 API 表面 |
| 6 | 资源读取输出 Annotated 模式 | 对 LLM 最友好（含结构标记） | 输出较长，消耗 token |
| 7 | `create_docx_from_data` 用模板枚举 | 简化 LLM 调用（无需理解完整 API） | 功能受限（仅 heading/table/list） |

### 5.1 安全考虑

1. **路径穿越防护**：`safe_resolve_path()` 使用 `canonicalize()` + `starts_with()` 校验。
2. **无网络暴露**：仅 stdio，不监听端口。
3. **无认证**：依赖操作系统进程隔离。
4. **工具输出限制**：大文档的 `read_docx_blocks` 可能返回大量 JSON，需要考虑 token 限制。

### 5.2 待扩展能力

1. **HTTP/SSE 传输**：支持远程 MCP 客户端连接。
2. **自定义工具注册**：`ServerConfig` 支持动态注册工具。
3. **资源订阅**：文件变更时通知客户端。
4. **sampling 能力**：MCP 服务器回调 LLM（当前不支持）。
5. **write_docx 工具**：完整的文档写入工具（当前仅 `create_docx_from_data` 模板）。
6. **fill_template 工具**：模板填充工具。
7. **SAX 流式读取工具**：大文档的流式处理。

## 6. 测试与验收

### 6.1 现有测试

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_initialize` | 返回正确的 server info 和 capabilities | `server_test.rs` |
| `test_tools_list` | 返回 6 个工具定义 | `server_test.rs` |
| `test_read_docx` | 读取 DOCX 返回文本 | `server_test.rs` |
| `test_read_table` | 提取表格为 JSON | `server_test.rs` |
| `test_convert_to_markdown` | 转换返回 Markdown | `server_test.rs` |
| `test_create_heading` | 创建标题文档 | `server_test.rs` |
| `test_resources_list` | 列出目录中的文件 | `server_test.rs` |
| `test_resources_read` | 读取资源返回 Annotated 文本 | `server_test.rs` |
| `test_prompts_list` | 返回 4 个 prompt 模板 | `server_test.rs` |
| `test_prompts_get` | 渲染 prompt 返回消息 | `server_test.rs` |
| `test_path_traversal_blocked` | 路径穿越被拒绝 | `server_test.rs` |

### 6.2 待补充测试

- **大文档 token 限制**：`read_docx_blocks` 在超大文档下的输出截断。
- **并发请求**：多条 stdin 请求的处理顺序。
- **无效 JSON 处理**：畸形 JSON-RPC 消息的错误响应。
- **空目录资源扫描**：无 docx 文件时返回空列表。
- **prompt 参数缺失**：必填参数缺失时的错误信息。

## 7. 引用

- 架构文档：`docs/easydoc-rust-Architecture.zh_CN.md` 第 15 节「演进路线」Phase 5
- Roadmap：`docs/roadmap.md` Phase 5（easydoc-mcp）
- MCP 协议规范：https://modelcontextprotocol.io/specification/2024-11-05
- 源码：`crates/easydoc-mcp/src/`
