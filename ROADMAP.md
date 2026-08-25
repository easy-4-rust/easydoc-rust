# easydoc-rust Roadmap

This document outlines the development plan for easydoc-rust from the current alpha through 1.0 and beyond. Timelines are quarterly estimates and may shift based on community feedback and contributor availability.

For the full list of changes in each release, see [CHANGELOG.md](CHANGELOG.md).

---

## 0.1.0-alpha.x (2026 Q2-Q3, current)

**Status: alpha, API may change without notice.**

- [x] 9-crate workspace published to crates.io
- [x] SAX streaming read with O(1) memory (paragraphs, tables, images, formulas, lists, hyperlinks, nested tables, merged cells)
- [x] Bidirectional Markdown conversion (DOCX-to-Markdown and Markdown-to-DOCX)
- [x] MCP server with 6 tools, directory resources, 4 prompts
- [x] `#[derive(DocxRow)]` proc-macro with 9 attributes fully wired to OOXML output
- [x] serde serialization bridge (feature-gated)
- [x] Security hardening: SSRF protection, ZIP bomb mitigation, Zip Slip prevention, path traversal guards
- [x] CI quality gates: fmt, clippy (`-D warnings`), cargo-deny, rustsec audit, 6-matrix build
- [x] 607 tests passing across the workspace

**Known issues carried forward to 0.1.0:**

- MD-to-DOCX missing: HTML tags, blockquotes `>`, task lists `- [ ]`, footnotes, strikethrough, front matter, math `$...$` — ✅ 全部补齐：strikethrough / 块级数学 `$$...$$` / 脚注 / HTML 内联标签（`<strong>`/`<b>`/`<em>`/`<i>`/`<code>`/`<a>`/`<br>`）+ 块级 `<hr>`/`<img>`，含往返测试
- serde `Vec<u8>` serializes as JSON array instead of base64 — ✅ 已改为 base64（向后兼容旧数字数组）
- MCP `resources/subscribe` notification not implemented — ✅ 已实现 subscribe/unsubscribe（同步 stdio 模型不推送变化通知）
- Nested list: unbalanced ilvl (e.g. 0 to 2 skipping 1) creates empty intermediate containers — ✅ 已确认不创建空容器（跳级时直接挂载最近层级，有测试覆盖）
- Write throughput: ~600 rows/s at 1K rows (XML serialization bottleneck, superlinear growth) — ✅ 已修复：apply_xml_extras 从 O(n²) 改为线性批量插入，1000 行 1.49s → 10.5ms（~95k rows/s）
- MSRV 1.88.0 not validated outside CI matrix
- `DocumentList.start_number` field not applied by writer — ✅ 已实现（动态 numbering 定义，alpha.2 已含测试）
- `<m:spre>` (pre-sub-superscript) OMML structure not supported — ✅ 已支持（omml_to_latex process_spre + 测试）
- Nested table alignment: `column_span`/`row_span` defaults may be inconsistent — ✅ 已修复：writer 输出 vMerge restart/continue，reader 计算 restart 实际跨行数，往返一致

---

## 0.1.0 (released 2026-08-25) — ✅ 首个稳定发布（API 冻结，详见 CHANGELOG）

**Goal: first stable release with frozen API and complete Markdown-to-DOCX coverage.**

### API stability

- Freeze the public API surface of all 9 crates — ✅ 已冻结（0.1.0 起，见 README API Stability）
- Commit to backward compatibility within the 0.x line (no breaking changes until 1.0) — ✅ 已在 README 声明
- `#[non_exhaustive]` on all public enums for future extensibility — ✅ 已全量覆盖（含 DocError/DocumentFormat）
- Run `cargo-semver-checks` in CI on every PR — ✅ 已有 semver.yml（push main/dev + PR）

### Markdown-to-DOCX completion

- HTML tags: `<br>`, `<hr>`, `<em>`, `<strong>`, `<code>`, `<a>`, `<img>` — ✅ 已实现（内联 `<strong>`/`<b>`/`<em>`/`<i>`/`<code>`/`<a href>`/`<br>` → run 属性；块级 `<hr>` → ThematicBreak、`<img>` → Image）
- Blockquotes (`>`) with nesting — ✅ 已实现（含 `>>` 嵌套，往返测试）
- Task lists (`- [ ]`, `- [x]`) — ✅ 已实现（任务列表渲染，含往返测试）
- Footnotes (`[^1]`) — ✅ 已实现（脚注定义/引用，含往返测试）
- Strikethrough (`~~text~~`) — ✅ 已实现（run strike 属性，含往返测试）
- Front matter (YAML) pass-through to document properties — ✅ 已实现（comrak front_matter_delimiter）
- Math formulas: `$...$` inline, `$$...$$` block — ✅ 已实现：comrak 解析（行内/多行）+ tex2word-math LaTeX→OMML 写回 DOCX 原生公式 + omml_to_latex 读回（sax 路径），MD→DOCX→MD 公式往返闭环

### Bug fixes and correctness

- `DocumentList.start_number` applied by writer
- Nested list: balanced ilvl (no empty intermediate containers)
- `<m:spre>` OMML structure support
- Nested table `column_span`/`row_span` default consistency
- serde `Vec<u8>` serialized as base64 string (with feature flag for backward compat)

### Performance

- Write throughput target: 2000+ rows/s at 1K rows (3x improvement) — ✅ 已达 ~95k rows/s（apply_xml_extras 线性化，1000 行 10.5ms）
- XML serialization optimization: streaming writer, reduced allocations — ✅ 已消除 apply_xml_extras O(n²) 字符串重建；docx-rs 流式打包保持
- Benchmark regression gate in CI (Criterion, fail on >10% regression) — ✅ 已加 bench-regression 任务（缓存基准 + scripts/bench_regression_check.py，>10% 失败）

### Quality and testing

- Target: 1000+ tests across the workspace — ✅ 已达 1000（全绿）
- Property-based tests (proptest) for round-trip: write then read produces equivalent content — ✅ 已加（`crates/easydoc/tests/roundtrip_proptest.rs`，256 cases × 3 属性）
- Fuzz testing: ZIP parsing, Markdown parser, OOXML SAX parser — ✅ 已有 fuzz_docx_xml / fuzz_docx_reader / fuzz_markdown_import 三个 target
- docs.rs metadata: categories, keywords, rustdoc-args for clean rendering — ✅ 已有 categories/keywords（easydoc crate）
- Golden test suite: known DOCX files with expected output snapshots — ✅ 已建（`crates/easydoc/tests/golden_test.rs` + `tests/golden/*.md`，UPDATE_GOLDEN=1 刷新）

### MCP

- `resources/subscribe` notification support
- Configurable root directory for `DirectoryResourceProvider` — ✅ 已支持（`default_config_with_root` / `EASYDOC_MCP_ROOT` 环境变量 / `ServerConfig::new` 自定义 provider，含测试）

---

## 0.2.0 (target: 2026 Q4)

**Goal: format expansion and differentiated capabilities.**

### XLSX / Excel support

- Integrate `easyexcel-rust` as a dependency or extract shared OOXML infrastructure
- Read: cell values, formulas, sheets, styles
- Write: data tables, basic formatting
- Shared ZIP/OOXML parsing layer between DOCX and XLSX

### PPTX / PowerPoint support

- Read: slides, text, images, shapes
- Write: basic slide creation from structured data
- Leverage the same OOXML foundation

### CSV import / export

- `EasyDoc::from_csv("data.csv") -> DocumentContent`
- `EasyDoc::to_csv("document.docx") -> csv::Writer`
- Configurable delimiter, encoding, header handling

### Advanced ViewMode

- Table view: row-oriented and column-oriented table rendering
- Markdown view: full Markdown output with front matter
- Custom ViewMode: user-defined `ViewModeFn` trait

### Caching

- LRU cache for `EasyDoc::read_text()` results (configurable size, TTL)
- Cache invalidation on file modification time change

### Security audit

- [x] XML External Entity (XXE) prevention audit — quick-xml 流式解析器不解析 DTD 外部实体，已用 3 个测试固化（external/internal/billion-laughs，见 `crates/easydoc-reader/src/security.rs`）
- Dependency vulnerability review (cargo-audit + manual)
- Threat model documentation

---

## 0.3.0 (target: 2027 Q1)

**Goal: async support and framework integration.**

### Async streaming API

- `tokio`-based async read/write: `EasyDoc::read_events_async()`
- Async `EventSink` trait: `async fn on_event()`
- Backpressure-aware streaming for large document pipelines

### Web framework integration

- `easydoc-axum`: extractors, response types, multipart upload
- `easydoc-actix`: equivalent integration for Actix Web
- `easydoc-warp`: filters and reply types
- All framework crates are optional, feature-gated

### Document conversion pipeline

- Chain multiple transformations: `Pipeline::new().from_docx().to_markdown().to_html()`
- Custom pipeline stages via `Transform` trait
- Error recovery and partial output on conversion failure

### Template DSL

- Handlebars-like template syntax for document generation
- Conditionals: `{{#if condition}}...{{/if}}`
- Loops: `{{#each items}}...{{/each}}`
- Nested object access: `{{user.name}}`

### Internationalization (i18n)

- Unicode-aware text statistics (word count, character count)
- RTL text support in DOCX output
- Locale-aware number and date formatting in template fill

---

## 0.5.0 (target: 2027 Q2)

**Goal: API stabilization candidate for 1.0.**

- Full review of all public APIs against [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Deprecation warnings for any APIs that will change at 1.0
- Comprehensive security audit (external if resources allow)
- Performance optimization pass based on real-world usage data
- RFC: long-term support version policy

---

## 1.0.0 (target: 2027 Q4+)

**Goal: production-ready stable release.**

- API permanently stable under semver
- Long-term support: minimum 12 months of bug fixes per major release
- Full cargo feature matrix validation (all combinations of optional features)
- Compliance with Rust API Guidelines
- Published security policy and vulnerability response process
- Benchmark baselines locked and tracked over time

---

## Non-Goals

The following are explicitly out of scope for easydoc-rust:

- **GUI document editor.** Use LibreOffice, OnlyOffice, or similar tools for visual editing. easydoc-rust is a programmatic library.
- **PDF conversion.** Use dedicated PDF libraries (e.g. `printpdf`, `weasyprint`). PDF is a fundamentally different format from OOXML.
- **Cloud storage integration.** S3, OSS, Azure Blob, etc. belong in the application layer, not in a document library.
- **Legacy DOC format (Word 95/97).** The binary DOC format is obsolete. OOXML (.docx) is the standard. Read-only DOC support exists via `office_oxide` and will remain read-only.
- **Real-time collaboration.** OT/CRDT document editing is a separate domain.

---

## Long-Term Vision

- Become the **default DOCX library in the Rust ecosystem**, analogous to what EasyExcel is for Java.
- Share low-level OOXML infrastructure with `easyexcel-rust` for XLSX and DOCX interoperability.
- Maintain an AI-agent-friendly API surface (already a design goal with `ViewMode::Annotated` and `EventSink` streaming).
- Contribute upstream improvements to `docx-rs` and `office_oxide` where beneficial.

---

## How to Influence This Roadmap

- Open an [issue](https://github.com/easy-4-rust/easydoc-rust/issues) to request features or report bugs.
- Vote on existing issues with :+1: reactions to help prioritize.
- Pull requests are welcome for any item listed above. See [CONTRIBUTING.md](CONTRIBUTING.md) if available, or open a draft PR to discuss approach first.
