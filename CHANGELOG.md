# Changelog

本项目的所有重要变更记录于此文件。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。
alpha 阶段（0.x-alpha.y）允许 API 不兼容变更。

## [0.1.0-alpha.1] — 2026-08-10

首个 alpha 预发布。核心能力已就绪，但 API 仍可能调整。
607 个测试全绿，clippy/rustfmt 全 workspace 零警告。

### 新增

#### 9-crate workspace

- `easydoc` — 对外门面 crate，聚合所有能力（`EasyDoc` 静态工厂，18 个方法）
- `easydoc-core` — 6 大核心 trait（`DocxRow` / `DocConverter` / `DocReadListener` / `DocWriteHandler` / `DocumentReader` / `EventSink`）+ 数据模型 + 错误体系
- `easydoc-derive` — `#[derive(DocxRow)]` 过程宏，9 个属性
- `easydoc-reader` — SAX 流式 DOCX 读取器（O(1) 内存）
- `easydoc-writer` — DOCX 写入器（基于 docx-rs）+ 注解真正生效到 OOXML
- `easydoc-markdown` — DOCX ↔ Markdown **双向**转换
- `easydoc-template` — `{key}` 标量 + `{.field}` 列表模板填充
- `easydoc-ooxml` — OOXML ZIP 包原子重写（防损坏）
- `easydoc-mcp` — MCP 服务器（tools:6 + resources + prompts:4）

#### 读写能力

- **SAX 流式读取**（O(1) 内存）：段落、标题（H1-H6）、表格（含 `gridSpan`/`vMerge` 合并单元格）、图片（真实二进制提取）、列表（有序/无序 + 多级嵌套）、超链接（rId 解析为真实 URL）、嵌套表格、OMML 数学公式（`<m:oMath>` inline + `<m:oMathPara>` display）
- **DOCX 写入**：段落、标题、表格、图片、页面分隔，全部支持 `DocxRow` derive 宏
- **numbering.xml 解析**：列表 `ordered` / `start_number` 现在正确
- **relationships 解析**：hyperlink rId → 真实 URL

#### 派生宏（9 个属性全部生效到 OOXML）

`#[docx(name, order, width, format, align, wrap, converter, ignore)]`
- `width="2cm"/"80px"/"50%"/"auto"` → `<w:tcW>`
- `format="#,##0.00"/"yyyy-mm-dd"` → `<w:numFmt>`
- `align="right"/"center"/"left"/"both"` → `<w:jc>`
- `wrap=false` → `<w:noWrap/>`
- `converter="MyConverter"` → 通过 `ConverterRegistry` 实际转换（type-erased 模式）

#### 扩展 trait 体系（对标 easyexcel-rust）

- `DocxRow` — struct ↔ DOCX 表格行双向映射
- `DocConverter<T>` — Rust 类型 ↔ `DocValue` 双向转换（含 `ErasedConverter` 类型擦除模式）
- `DocReadListener<T>` — 流式读取回调（`invoke`/`invoke_table`/`on_complete`/`on_error`/`has_next`）
- `DocWriteHandler` — 写生命周期钩子（document/paragraph/table/cell 四级，含 `order()` 排序）
- `DocumentReader` — 统一读取入口（`read_model`/`read_events`）
- `EventSink` — 事件消费接口（配合 `ContentCollector` 收集为 `DocumentContent`）

#### 语义模型 + Read→Modify→Write 闭环

- `DocumentContent`（13 个 `DocumentBlock` 变体：Heading/Paragraph/Table/List/Image/Math/CodeBlock/PageBreak/ColumnBreak/ThematicBreak/TextBox/Footnote/Endnote/Section）
- `EasyDoc::load(path) → DocumentContent` → 程序修改 → `EasyDoc::write_content(content, path)`

#### 4 种 ViewMode（LLM 友好）

- `Plain` — 纯文本
- `Annotated` — 带结构标注 `[段落 1]`/`[表格 2: 3行×4列]`/`[图片]`（LLM 最友好）
- `Outline { max_level }` — 标题大纲 `# H1`/`## H2`
- `Stats` — 段落数/表格数/图片数/字数

#### Markdown 双向转换

- **DOCX → Markdown**（`MarkdownBuilder`）：含图片提取、front matter、OMML → LaTeX 公式（`$$...$$`/`$...$`）
- **Markdown → DOCX**（`MarkdownImportBuilder`）：手工状态机解析器，支持标题、段落、粗体/斜体/代码/超链接/图片、有序/无序列表（2 级嵌套）、表格（含 alignment colons）、代码块、水平分隔线

#### OMML → LaTeX 公式转换

- 完整移植 markitdown 的符号表（~175 个符号，覆盖 97%）
- 支持 17 种 OMML 结构：分数、根式、上下标、n 元运算（∑∫）、定界符、重音、矩阵、函数、极限等
- `DocumentBlock::Math { omml, latex, display }` 变体
- Markdown 渲染：`$$...$$`（block）/ `$...$`（inline）

#### serde 序列化桥接（feature-gated）

- `easydoc-core` 可选 `serde` feature：`DocumentContent` ↔ JSON/YAML/TOML
- 10 个核心类型手动 impl `Serialize`/`Deserialize`（tagged enum 模式）
- 辅助函数：`to_json`/`from_json`/`to_json_value`/`from_json_value`

#### MCP 服务器（`easydoc-mcp`）

- **6 个 tools**：`read_docx` / `read_table` / `read_docx_blocks` / `extract_images` / `convert_to_markdown` / `create_docx_from_data`
- **resources**：`DirectoryResourceProvider` 扫描目录暴露 DOCX 文件（路径穿越防护）
- **4 个 prompts**：`summarize_document` / `analyze_table_data` / `extract_key_points` / `compare_documents`
- 标准 JSON-RPC 2.0 + MCP 协议（initialize/tools/resources/prompts）
- stdio transport（newline-delimited JSON）

#### 安全加固

- **SSRF 防护**：默认拒绝 localhost/127.0.0.1/::1、所有 RFC1918 私有 IP、link-local（169.254/16，含云元数据 169.254.169.254）、carrier-grade NAT（100.64/10）、IPv6 unique-local/link-local/multicast；DNS 解析后再次检查
- **ZIP bomb 防护**：总解压大小 ≤ 100MB、单文件 ≤ 50MB、压缩比 ≤ 100x、条目数 ≤ 10000、文件名长度 ≤ 256
- **Zip Slip 防护**：拒绝路径含 `..` 和绝对路径
- **MCP resources 路径穿越防护**：canonicalize + starts_with 校验

#### 性能基准（criterion）

- 5 组基准：写吞吐（100/500/1K 行）、读文本吞吐、4 种 ViewMode 渲染、流式 vs 一次性、Markdown 转换
- 真实数字已填入 `docs/bench/RESULTS.md`

#### CI/CD 与依赖治理

- **GitHub Actions CI**：3 job × 6 矩阵 cell（ubuntu/macos/windows × stable/1.88.0 MSRV）
  - Build + Test + Clippy + Doctest + Examples build
  - `RUSTFLAGS: -D warnings`
  - `cargo fmt --check`
  - `cargo build --benches`
- **GitHub Actions Security**：rustsec/audit-check + cargo-deny（每周一 + push to main）
- **cargo-deny 配置**：License 白名单、bans、sources（仅 crates.io）
- **强制 `unsafe_code = "forbid"`**（全 workspace）
- **pedantic clippy 全开** + `missing_docs = "warn"`

#### 文档与示例

- **README.md / README_zh.md** 双语镜像，18 个 API 完整速查
- **docs/** 包含：架构文档（中英）、使用指南、路线图、benchmark 说明
- **11 个 examples**：read_basic / write_basic / table_with_struct / stream_read / view_modes / markdown_convert / load_modify_write / custom_converter / extract_images / template_fill / read_complex
- **核心 38 个 .rs 文件**：中文 doc 注释 + Java 对应标注（对标 EasyExcel 4.0.3）

### Rust 项目规范合规

- ✅ `lib.rs` / `mod.rs` 全部 0 类型定义（仅 mod 声明 + pub use）
- ✅ 一个类型一个 .rs 文件（保留合理的多类型抽象层集合：traits.rs/types.rs/protocol.rs/sax.rs）
- ✅ 生产代码零 wildcard import
- ✅ rustfmt 100% 合规
- ✅ clippy `-D warnings` 0 警告
- ✅ 607 个测试全绿

### 已知局限（后续 0.1.0 / 0.2.0 计划）

- **MD → DOCX** 不支持：HTML 标签、引用块 `>`、任务列表 `- [ ]`、脚注、删除线、front matter、数学公式 `$...$`
- **serde bridge**：`Vec<u8>` 序列化为 JSON 数组而非 base64
- **MCP**：`resources/subscribe` 通知未实现；`default_config` 扫描当前目录（生产应指定根目录）
- **列表嵌套**：不平衡 ilvl（0→2 跳过 1）创建中间空容器
- **写吞吐**：~600 rows/s @1K rows（XML 序列化是瓶颈，存在超线性增长）
- **MSRV**：1.88.0，未在 CI 矩阵外的 Rust 版本验证

## [0.1.0] — 2026-07-21（内部里程碑，未发布）

初始 6-crate workspace（core/derive/reader/writer/template/ooxml），基础读写能力。
