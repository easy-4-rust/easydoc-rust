# Release v0.1.0-alpha.1

首个 alpha 预发布。**核心能力已就绪，API 仍可能调整。**

- **测试**：607 个全绿
- **代码质量**：clippy `-D warnings` 0 警告，rustfmt 100% 合规
- **类型安全**：100% `unsafe_code = "forbid"`（全 workspace）
- **MSRV**：Rust 1.88.0
- **License**：Apache-2.0

## 9-crate workspace

| crate | 职责 |
|-------|------|
| `easydoc` | 对外门面（`EasyDoc` 静态工厂，18 个方法） |
| `easydoc-core` | 6 大核心 trait + 数据模型 + 错误体系 |
| `easydoc-derive` | `#[derive(DocxRow)]` 过程宏（9 个属性） |
| `easydoc-reader` | SAX 流式 DOCX 读取器（O(1) 内存） |
| `easydoc-writer` | DOCX 写入器（注解真正生效到 OOXML） |
| `easydoc-markdown` | DOCX ↔ Markdown 双向转换 |
| `easydoc-template` | `{key}` + `{.field}` 模板填充 |
| `easydoc-ooxml` | OOXML ZIP 包原子重写 |
| `easydoc-mcp` | MCP 服务器（tools + resources + prompts） |

## 快速开始

```toml
[dependencies]
easydoc = "0.1.0-alpha.1"
```

### 写 DOCX

```rust
use easydoc::{EasyDoc, DocxRow};

#[derive(DocxRow)]
struct User {
    #[docx(name = "姓名", order = 0, width = "3cm")]
    name: String,
    #[docx(name = "年龄", order = 1, align = "center")]
    age: u32,
}

let users = vec![
    User { name: "Alice".into(), age: 30 },
    User { name: "Bob".into(), age: 25 },
];

EasyDoc::write_table("users.docx", &users).do_write()?;
```

### 流式读取（O(1) 内存）

```rust
use easydoc::{EasyDoc, ContentCollector, EventSink};

let mut collector = ContentCollector::new();
EasyDoc::read_events("huge.docx", &mut collector)?;
let content = collector.into_content();
```

### LLM 友好视图

```rust
use easydoc::{EasyDoc, ViewMode};

// Annotated 模式带结构标注，LLM 最友好
let text = EasyDoc::view_as("report.docx", &ViewMode::Annotated)?;
// [标题1] 季度报告
// [段落 1] 本季度业绩...
// [表格 1: 3行×4列] ...
```

### DOCX → Markdown

```rust
let markdown = EasyDoc::to_markdown("document.docx")?;
```

### MCP 服务器（给 LLM agent 用）

```bash
# 安装到 Claude Code / Cursor 等
officecli mcp  # 或直接运行 easydoc-mcp 二进制
```

## 核心特性

### SAX 流式读取（O(1) 内存）
- 段落、标题（H1-H6）、表格（含 `gridSpan`/`vMerge` 合并）
- 图片（真实二进制提取，从 word/media/*）
- 列表（有序/无序 + 多级嵌套，numbering.xml 解析）
- 超链接（rId 解析为真实 URL）
- 嵌套表格
- OMML 数学公式（`<m:oMath>` + `<m:oMathPara>`）

### 派生宏属性（全部生效到 OOXML）
`#[docx(name, order, width, format, align, wrap, converter, ignore)]`

### Markdown 双向转换
- DOCX → MD（含 OMML → LaTeX 公式 `$$...$$`/`$...$`）
- MD → DOCX（手工状态机，无外部依赖）

### serde 派生（feature-gated）
```toml
easydoc-core = { version = "0.1.0-alpha.1", features = ["serde"] }
```

### MCP 服务器
- 6 个 tools（read_docx / read_table / read_docx_blocks / extract_images / convert_to_markdown / create_docx_from_data）
- resources（DirectoryResourceProvider，路径穿越防护）
- 4 个 prompts（summarize_document / analyze_table_data / extract_key_points / compare_documents）

### 安全加固
- SSRF 防护（拒绝 localhost / 私有 IP / 云元数据 169.254.169.254）
- ZIP bomb 防护（100MB / 100x / 10000 条目上限）
- Zip Slip 防护（拒绝 `..` 路径）

## CI/CD
- GitHub Actions：3 job × 6 矩阵（ubuntu/macos/windows × stable/1.88.0）
- cargo-deny 依赖审计（每周一）

## 已知局限
- MD → DOCX 不支持：HTML 标签、引用块、任务列表、脚注、删除线、front matter
- serde `Vec<u8>` 序列化为 JSON 数组而非 base64
- 写吞吐 ~600 rows/s @1K rows（XML 序列化是瓶颈）
- MCP `resources/subscribe` 通知未实现

## Rust 项目规范合规
- ✅ `lib.rs` / `mod.rs` 0 类型定义
- ✅ 一个类型一个 .rs 文件
- ✅ 生产代码零 wildcard import
- ✅ rustfmt 100% 合规
- ✅ clippy `-D warnings` 0 警告
- ✅ 607 个测试全绿

详见 [CHANGELOG.md](./CHANGELOG.md)。

---

**反馈渠道**：[GitHub Issues](https://github.com/easy-4-rust/easydoc-rust/issues)
