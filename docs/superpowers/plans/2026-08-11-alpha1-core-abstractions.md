# v0.1.0-alpha.1 核心抽象与读写能力 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: rust-workspace, rust-module-layout

**Goal:** 完成 9-crate workspace 架构、SAX 流式读取、DOCX 写入、语义模型、双向 MD 转换、MCP 服务器、serde 桥接、安全加固，发布首个 alpha。

**Architecture:** `docs/easydoc-rust-Architecture.md`

**Tech Stack:** Rust 1.88+, docx-rs, office_oxide, quick-xml, zip, serde, proc-macro2/syn/quote

## Global Constraints

- MSRV 1.88.0
- `unsafe_code = "forbid"` 全 workspace
- pedantic clippy 全开 + `missing_docs = "warn"`
- rustfmt 100% 合规
- 一个类型一个 .rs 文件（lib.rs/mod.rs 仅 mod 声明 + pub use）
- 生产代码零 wildcard import

---

### Task 1: 9-crate workspace 骨架

> Files:
> - Create: `Cargo.toml`（workspace root）
> - Create: `crates/easydoc/Cargo.toml`
> - Create: `crates/easydoc-core/Cargo.toml`
> - Create: `crates/easydoc-derive/Cargo.toml`
> - Create: `crates/easydoc-ooxml/Cargo.toml`
> - Create: `crates/easydoc-reader/Cargo.toml`
> - Create: `crates/easydoc-writer/Cargo.toml`
> - Create: `crates/easydoc-template/Cargo.toml`
> - Create: `crates/easydoc-markdown/Cargo.toml`
> - Create: `crates/easydoc-mcp/Cargo.toml`

**Steps:**
- [x] 创建 workspace root Cargo.toml，resolver = "3"
- [x] 定义 workspace.package（version, edition, rust-version, license, repository）
- [x] 定义 workspace.dependencies（docx-rs, office_oxide, quick-xml, zip, serde, tempfile, thiserror 等）
- [x] 定义 workspace.lints（unsafe_code = "forbid", clippy pedantic）
- [x] 创建 9 个 crate 的 Cargo.toml，引用 workspace.dependencies
- [x] 每个 crate 创建 src/lib.rs，仅包含 mod 声明

---

### Task 2: easydoc-core 核心 trait 体系

> Files:
> - Create: `crates/easydoc-core/src/traits.rs`
> - Create: `crates/easydoc-core/src/types.rs`
> - Create: `crates/easydoc-core/src/error.rs`
> - Create: `crates/easydoc-core/src/lib.rs`
> - Create: `crates/easydoc-core/src/converter/mod_file.rs`
> - Create: `crates/easydoc-core/src/converter/registry.rs`
> - Create: `crates/easydoc-core/src/metadata/document.rs`
> - Create: `crates/easydoc-core/src/metadata/column.rs`

**Steps:**
- [x] 定义 `DocxRow` trait（schema / from_row / to_row / from_row_with_converters / to_row_with_converters）
- [x] 定义 `DocConverter<T>` trait（support_type / to_doc_value / from_doc_value）
- [x] 定义 `DocReadListener<T>` trait（invoke / invoke_table / on_complete / on_error / has_next）
- [x] 定义 `DocWriteHandler` trait（before/after document/paragraph/table/cell，含 order()）
- [x] 定义 `DocumentReader` trait（read_model / read_events）
- [x] 定义 `EventSink` trait（on_event）
- [x] 定义 `DocError` 错误体系（Io / Zip / Format / Template / Conversion / Unsupported / Document）
- [x] 定义 `DocValue` 枚举（String / Number / Bool / Bytes）
- [x] 定义 `TableColumn` / `RowData` / `CellData` 类型
- [x] 实现 `ConverterRegistry`（type-erased 模式）
- [x] 实现 `DocMeta`（title / author / subject / keywords）

---

### Task 3: easydoc-core 语义文档模型

> Files:
> - Create: `crates/easydoc-core/src/document/document_content.rs`
> - Create: `crates/easydoc-core/src/document/document_block.rs`
> - Create: `crates/easydoc-core/src/document/document_text_run.rs`
> - Create: `crates/easydoc-core/src/document/document_table.rs`
> - Create: `crates/easydoc-core/src/document/document_table_row.rs`
> - Create: `crates/easydoc-core/src/document/document_table_cell.rs`
> - Create: `crates/easydoc-core/src/document/document_list.rs`
> - Create: `crates/easydoc-core/src/document/document_list_item.rs`
> - Create: `crates/easydoc-core/src/document/document_image.rs`
> - Create: `crates/easydoc-core/src/document/mod.rs`

**Steps:**
- [x] 定义 `DocumentContent`（metadata + blocks）
- [x] 定义 `DocumentBlock` 枚举（13 个变体：Heading / Paragraph / Table / List / Image / Math / CodeBlock / PageBreak / ColumnBreak / ThematicBreak / TextBox / Footnote / Endnote / Section）
- [x] 定义 `DocumentTextRun`（text / bold / italic / strikethrough / hyperlink）
- [x] 定义 `DocumentTable` / `DocumentTableRow` / `DocumentTableCell`
- [x] 定义 `DocumentList` / `DocumentListItem`（ordered / start_number / items / nesting）
- [x] 定义 `DocumentImage`（data / alt_text / content_type）

---

### Task 4: easydoc-core 样式系统

> Files:
> - Create: `crates/easydoc-core/src/style/font.rs`
> - Create: `crates/easydoc-core/src/style/paragraph.rs`
> - Create: `crates/easydoc-core/src/style/table.rs`
> - Create: `crates/easydoc-core/src/style/color.rs`
> - Create: `crates/easydoc-core/src/style/mod.rs`
> - Create: `crates/easydoc-core/src/units.rs`

**Steps:**
- [x] 定义 `FontConfig`（name / size / bold / italic / underline / color）
- [x] 定义 `ParagraphStyle`（alignment / first_line_indent / space_after / line_spacing）
- [x] 定义 `TableStyle`（banded_rows / auto_width / borders / header_background）
- [x] 定义 `Color`（BLACK / WHITE / RED / HEADER_BLUE / rgb / from_hex / to_hex）
- [x] 定义 `HorizontalAlignment`（Left / Center / Right / Both）
- [x] 定义 `FontSlot`（eastAsia / ascii / hAnsi / cs）支持中文字体

---

### Task 5: easydoc-core serde 序列化桥接

> Files:
> - Modify: `crates/easydoc-core/src/lib.rs`
> - Create: `crates/easydoc-core/src/serde_bridge.rs`

**Steps:**
- [x] 为 `DocumentContent` / `DocumentBlock` / `DocumentTextRun` 等 10 个核心类型手动 impl Serialize / Deserialize
- [x] 使用 tagged enum 模式（serde tag = "type"）
- [x] feature-gated：`serde` feature 控制编译
- [x] 提供 `to_json` / `from_json` / `to_json_value` / `from_json_value` 辅助函数

---

### Task 6: easydoc-derive 过程宏

> Files:
> - Create: `crates/easydoc-derive/src/lib.rs`
> - Create: `crates/easydoc-derive/src/implementation.rs`

**Steps:**
- [x] 实现 `#[derive(DocxRow)]` 过程宏
- [x] 支持 9 个属性：name / index / order / width / format / align / wrap / converter / ignore
- [x] struct-level 属性：banded_rows / table_width / auto_width
- [x] 生成 `schema()` / `from_row()` / `to_row()` / `from_row_with_converters()` / `to_row_with_converters()`
- [x] width 属性解析为 OOXML 类型（dxa / pct / auto）
- [x] converter 属性通过 `ConverterRegistry` 运行时分发

---

### Task 7: easydoc-ooxml 原子文件操作

> Files:
> - Create: `crates/easydoc-ooxml/src/atomic_file.rs`
> - Create: `crates/easydoc-ooxml/src/package_limits.rs`
> - Create: `crates/easydoc-ooxml/src/package_rewriter.rs`
> - Create: `crates/easydoc-ooxml/src/lib.rs`

**Steps:**
- [x] 实现原子文件写入（temp file + persist）
- [x] 实现 `PackageLimits`（ZIP 条目数 ≤10000、单文件 ≤50MB、总大小 ≤100MB、压缩比 ≤100x、文件名长度 ≤256）
- [x] 实现 `PackageRewriter`（安全 ZIP 重写，未修改条目 byte-for-byte 保留）
- [x] 实现 Zip Slip 防护（拒绝 `..` 和绝对路径）

---

### Task 8: easydoc-reader SAX 流式读取

> Files:
> - Create: `crates/easydoc-reader/src/lib.rs`
> - Create: `crates/easydoc-reader/src/read_document.rs`
> - Create: `crates/easydoc-reader/src/read_text.rs`
> - Create: `crates/easydoc-reader/src/read_tables.rs`
> - Create: `crates/easydoc-reader/src/security.rs`
> - Create: `crates/easydoc-reader/src/builder/read_builder.rs`
> - Create: `crates/easydoc-reader/src/builder/mod.rs`
> - Create: `crates/easydoc-reader/src/view/` (plain.rs, annotated.rs, outline.rs, stats.rs, view_mode.rs, render.rs, mod.rs)
> - Create: `crates/easydoc-reader/src/extractor/` (directory)
)
> - Create: `crates/easydoc-reader/src/listener/` (directory)

**Steps:**
- [x] 实现 SAX 流式读取器（O(1) 内存）
- [x] 覆盖内容类型：段落、标题 H1-H6、表格（含 gridSpan/vMerge 合并单元格）、图片（二进制提取）、列表（有序/无序 + 多级嵌套）、超链接（rId 解析为真实 URL）、嵌套表格、OMML 数学公式
- [x] 实现 `numbering.xml` 解析（列表 ordered / start_number 正确）
- [x] 实现 relationships 解析（hyperlink rId → URL）
- [x] 实现 `read_text()` 快速文本提取
- [x] 实现 `read_tables::<T>()` 类型化表格提取
- [x] 实现 `read_events()` SAX 事件流
- [x] 实现 `ReadBuilder`（流式读取构建器）
- [x] 实现 4 种 ViewMode（Plain / Annotated / Outline / Stats）
- [x] 实现 SSRF 防护（拒绝 localhost / RFC1918 / link-local / carrier-grade NAT）
- [x] 实现 DOC 格式检测（magic bytes + 扩展名）

---

### Task 9: easydoc-writer DOCX 写入

> Files:
> - Create: `crates/easydoc-writer/src/lib.rs`
> - Create: `crates/easydoc-writer/src/builder/doc_builder.rs`
> - Create: `crates/easydoc-writer/src/builder/table_builder.rs`
> - Create: `crates/easydoc-writer/src/builder/mod.rs`
> - Create: `crates/easydoc-writer/src/executor/write_executor.rs`
> - Create: `crates/easydoc-writer/src/executor/table_executor.rs`
> - Create: `crates/easydoc-writer/src/executor/mod.rs`
> - Create: `crates/easydoc-writer/src/content_renderer.rs`
> - Create: `crates/easydoc-writer/src/handler/` (directory)
> - Create: `crates/easydoc-writer/src/paragraph.rs`
> - Create: `crates/easydoc-writer/src/run.rs`
> - Create: `crates/easydoc-writer/src/table.rs`
> - Create: `crates/easydoc-writer/src/doc_image.rs`
> - Create: `crates/easydoc-writer/src/doc_editor.rs`
> - Create: `crates/easydoc-writer/src/style/` (directory)
> - Create: `crates/easydoc-writer/src/util/` (directory)

**Steps:**
- [x] 实现 `DocBuilder`（完整文档构建器：heading / paragraph / table / image / page_break）
- [x] 实现 `TableWriteBuilder`（快捷表格写入：title / need_header / header_style / banded_rows）
- [x] 实现 `WriteExecutor` / `TableWriteExecutor`（OOXML 生成）
- [x] 实现 `content_renderer`（语义模型 → DOCX 渲染）
- [x] 实现 `DocWriteHandler` 回调集成（render_with_handler）
- [x] 实现 `DocEditor`（replace_text / save / save_as）
- [x] 实现 H1-H6 heading 写入（Word heading styles + outline level）
- [x] 实现 A4 页面、页边距、typed units、中文字体 slots
- [x] 实现模板 XML 特殊字符转义（`&` / `<` / `>` / `"` / `'`）
- [x] 实现跨 `<w:t>` 节点标量占位符替换

---

### Task 10: easydoc-template 模板填充

> Files:
> - Create: `crates/easydoc-template/src/lib.rs`
> - Create: `crates/easydoc-template/src/fill_template.rs`
> - Create: `crates/easydoc-template/src/fill_template_list.rs`
> - Create: `crates/easydoc-template/src/fill_executor.rs`
> - Create: `crates/easydoc-template/src/fill_config.rs`
> - Create: `crates/easydoc-template/src/placeholder.rs`

**Steps:**
- [x] 实现 `{key}` 标量占位符替换
- [x] 实现 `{.field}` 集合展开（表行复制 + 替换）
- [x] 实现 `FillConfig`（direction / force_new_row / auto_style）
- [x] 实现 `FillDirection`（Vertical / Horizontal）
- [x] 实现命名集合 `{prefix.field}` 语法

---

### Task 11: easydoc-markdown 双向转换

> Files:
> - Create: `crates/easydoc-markdown/src/lib.rs`
> - Create: `crates/easydoc-markdown/src/markdown_builder.rs`
> - Create: `crates/easydoc-markdown/src/markdown_renderer.rs`
> - Create: `crates/easydoc-markdown/src/markdown_import.rs`
> - Create: `crates/easydoc-markdown/src/markdown_options.rs`
> - Create: `crates/easydoc-markdown/src/markdown_result.rs`
> - Create: `crates/easydoc-markdown/src/extracted_asset.rs`
> - Create: `crates/easydoc-markdown/src/conversion_warning.rs`
> - Create: `crates/easydoc-markdown/src/math/mod.rs`
> - Create: `crates/easydoc-markdown/src/math/omml_to_latex.rs`
> - Create: `crates/easydoc-markdown/src/math/latex_dict.rs`

**Steps:**
- [x] DOCX → Markdown：标题、富文本、超链接、GFM 表格（pipe 转义、自动列宽）
- [x] DOCX → Markdown：合并单元格 → HTML `<table>` + 降级警告
- [x] DOCX → Markdown：有序/无序嵌套列表、代码块、脚注、尾注
- [x] DOCX → Markdown：图片提取（可配置目录和引用前缀）
- [x] DOCX → Markdown：YAML front matter、主题分隔、页面/列分隔
- [x] DOCX → Markdown：OMML → LaTeX 公式转换（175 个符号，17 种结构）
- [x] Markdown → DOCX：标题、段落、粗体/斜体/代码/超链接/图片
- [x] Markdown → DOCX：有序/无序列表（2 级嵌套）、表格（含 alignment）、代码块、水平分隔线
- [x] `MarkdownBuilder` API（image_directory / image_reference_prefix / include_front_matter）
- [x] `MarkdownResult`（markdown / assets / warnings）
- [x] 原子文件输出

---

### Task 12: easydoc-mcp MCP 服务器

> Files:
> - Create: `crates/easydoc-mcp/src/lib.rs`
> - Create: `crates/easydoc-mcp/src/server.rs`
> - Create: `crates/easydoc-mcp/src/protocol.rs`
> - Create: `crates/easydoc-mcp/src/tools.rs`
> - Create: `crates/easydoc-mcp/src/resources.rs`
> - Create: `crates/easydoc-mcp/src/prompts.rs`
> - Create: `crates/easydoc-mcp/src/transport/mod.rs`
> - Create: `crates/easydoc-mcp/src/transport/stdio.rs`

**Steps:**
- [x] 实现 MCP 协议（initialize / tools / resources / prompts）
- [x] 实现 6 个 tools：read_docx / read_table / read_docx_blocks / extract_images / convert_to_markdown / create_docx_from_data
- [x] 实现 `DirectoryResourceProvider`（扫描目录暴露 DOCX 文件，含路径穿越防护）
- [x] 实现 4 个 prompts：summarize_document / analyze_table_data / extract_key_points / compare_documents
- [x] 实现 stdio transport（newline-delimited JSON）

---

### Task 13: easydoc 门面 crate

> Files:
> - Create: `crates/easydoc/src/lib.rs`
> - Create: `crates/easydoc/src/easy_doc.rs`

**Steps:**
- [x] 实现 `EasyDoc` 静态工厂（18 个方法）
- [x] 聚合所有子 crate 能力到统一入口
- [x] 定义 `prelude` 模块（常用类型 re-export）
- [x] 实现 `detect_format()` 独立函数

---

### Task 14: 性能基准与测试

> Files:
> - Create: `crates/easydoc/benches/read_write.rs`
> - Create: `crates/easydoc/benches/fixtures/` (table.rs 等)
> - Create: `docs/bench/RESULTS.md`
> - Create: `docs/bench/METHODOLOGY.md`
> - Create: `docs/bench/README.md`

**Steps:**
- [x] 实现 5 组 Criterion 基准：写吞吐 / 读文本 / ViewMode / 流式 vs 一次性 / Markdown 转换
- [x] 填入真实 benchmark 数字到 RESULTS.md
- [x] 607 个测试全绿
- [x] clippy / rustfmt 全 workspace 零警告

---

### Task 15: CI/CD 与文档

> Files:
> - Create: `.github/workflows/ci.yml`
> - Create: `.github/workflows/security.yml`
> - Create: `deny.toml`
> - Create: `README.md`
> - Create: `README_zh.md`
> - Create: `docs/easydoc-rust-Architecture.md`
> - Create: `docs/easydoc-rust-Architecture.zh_CN.md`
> - Create: `docs/usage-guide.md`
> - Create: `docs/roadmap.md`
> - Create: `examples/` (11 个示例)

**Steps:**
- [x] GitHub Actions CI（3 job × 6 矩阵：ubuntu/macos/windows × stable/1.88.0 MSRV）
- [x] Build + Test + Clippy + Doctest + Examples build + `RUSTFLAGS: -D warnings`
- [x] GitHub Actions Security（rustsec/audit-check + cargo-deny，每周一 + push to main）
- [x] cargo-deny 配置（License 白名单、bans、sources）
- [x] 双语 README（18 个 API 完整速查）
- [x] 架构文档（中英）
- [x] 使用指南
- [x] 路线图
- [x] 11 个 examples

---

## Acceptance / Verification

```bash
cargo test --workspace                    # 607 tests pass
cargo clippy --workspace -- -D warnings   # 0 warnings
cargo fmt --check                         # 100% compliant
cargo build --benches                     # benchmarks compile
cargo build --examples                    # examples compile
```
