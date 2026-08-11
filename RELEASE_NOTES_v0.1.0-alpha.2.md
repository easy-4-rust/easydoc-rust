# Release v0.1.0-alpha.2

第二个 alpha 预发布。在 alpha.1 基础上新增 MD→DOCX 扩展、文档与质量增强。

- **测试**：635 个全绿（+28 自 alpha.1）
- **代码质量**：clippy `-D warnings` 0 警告，rustfmt 100% 合规
- **类型安全**：100% `unsafe_code = "forbid"`（全 workspace）
- **MSRV**：Rust 1.88.0
- **License**：Apache-2.0

## 新增能力

### Markdown → DOCX 扩展

- **Front matter**：解析文件开头 `---` 分隔的 YAML 元数据（`title`/`author`/`subject`/`keywords`）自动填入 `DocumentMeta`
- **引用块 `>`**：多行引用 → 斜体段落
- **任务列表 `- [ ]` / `- [x]`**：checkbox 语法 → `☐`/`☑` unicode 前缀

### Writer 增强

- **`DocumentList.start_number` 支持**：有序列表起始数字现在生效（动态 numId 分配）

### OMML 公式

- **`<m:spre>`**（pre-sub-superscript）：`{}^{top}{}_{bot}base`

### 文档（full-stack-doc 标准）

- **9 个 crate 中英文 README**（18 个文件）：Rust 母模板 + 文档格式处理 + 上游兼容（Java EasyExcel 4.0.3）+ 大型工具箱 + 多语言布局剖面
- 支持矩阵分别声明（读/写/编辑/模板填充/往返保真）

### 质量与安全

- **SECURITY.md**：漏洞报告流程 + 响应时间承诺
- **proptest fuzzing**：8 个 property-based 测试（2048 adversarial cases）
- **fidelity benchmark**：5 个 fixture byte-equal 断言
- **ROADMAP.md**：0.1.0 → 1.0.0 路线图
- **GitHub Issue/PR 模板**：bug / feature / question + quality gate checklist

## crates.io 发布（9/9）

| crate | 版本 |
|-------|------|
| `easydoc` | 0.1.0-alpha.2 |
| `easydoc-core` | 0.1.0-alpha.2 |
| `easydoc-derive` | 0.1.0-alpha.2 |
| `easydoc-reader` | 0.1.0-alpha.2 |
| `easydoc-writer` | 0.1.0-alpha.2 |
| `easydoc-template` | 0.1.0-alpha.2 |
| `easydoc-markdown` | 0.1.0-alpha.2 |
| `easydoc-ooxml` | 0.1.0-alpha.2 |
| `easydoc-mcp` | 0.1.0-alpha.2 |

## 快速开始

```toml
[dependencies]
easydoc = "0.1.0-alpha.2"
```

## 已知局限（0.1.0 计划）

- MD → DOCX 不支持：HTML 标签、脚注、删除线、数学公式 `$...$`
- 列表嵌套：不平衡 ilvl 仍创建中间容器
- 写吞吐：XML 后处理仍有优化空间
- MCP：`resources/subscribe` 通知未实现

## 完整变更

见 [CHANGELOG.md](https://github.com/easy-4-rust/easydoc-rust/blob/main/CHANGELOG.md)

---

**反馈渠道**：[GitHub Issues](https://github.com/easy-4-rust/easydoc-rust/issues) · [Discussions](https://github.com/easy-4-rust/easydoc-rust/discussions)
