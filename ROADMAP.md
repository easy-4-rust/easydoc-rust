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

- MD-to-DOCX missing: HTML tags, blockquotes `>`, task lists `- [ ]`, footnotes, strikethrough, front matter, math `$...$`
- serde `Vec<u8>` serializes as JSON array instead of base64
- MCP `resources/subscribe` notification not implemented
- Nested list: unbalanced ilvl (e.g. 0 to 2 skipping 1) creates empty intermediate containers
- Write throughput: ~600 rows/s at 1K rows (XML serialization bottleneck, superlinear growth)
- MSRV 1.88.0 not validated outside CI matrix
- `DocumentList.start_number` field not applied by writer
- `<m:spre>` (pre-sub-superscript) OMML structure not supported
- Nested table alignment: `column_span`/`row_span` defaults may be inconsistent

---

## 0.1.0 (target: 2026 Q3-Q4)

**Goal: first stable release with frozen API and complete Markdown-to-DOCX coverage.**

### API stability

- Freeze the public API surface of all 9 crates
- Commit to backward compatibility within the 0.x line (no breaking changes until 1.0)
- `#[non_exhaustive]` on all public enums for future extensibility
- Run `cargo-semver-checks` in CI on every PR

### Markdown-to-DOCX completion

- HTML tags: `<br>`, `<hr>`, `<em>`, `<strong>`, `<code>`, `<a>`, `<img>`
- Blockquotes (`>`) with nesting
- Task lists (`- [ ]`, `- [x]`)
- Footnotes (`[^1]`)
- Strikethrough (`~~text~~`)
- Front matter (YAML) pass-through to document properties
- Math formulas: `$...$` inline, `$$...$$` block (LaTeX-to-OMML or plain text fallback)

### Bug fixes and correctness

- `DocumentList.start_number` applied by writer
- Nested list: balanced ilvl (no empty intermediate containers)
- `<m:spre>` OMML structure support
- Nested table `column_span`/`row_span` default consistency
- serde `Vec<u8>` serialized as base64 string (with feature flag for backward compat)

### Performance

- Write throughput target: 2000+ rows/s at 1K rows (3x improvement)
- XML serialization optimization: streaming writer, reduced allocations
- Benchmark regression gate in CI (Criterion, fail on >10% regression)

### Quality and testing

- Target: 1000+ tests across the workspace
- Property-based tests (proptest) for round-trip: write then read produces equivalent content
- Fuzz testing: ZIP parsing, Markdown parser, OOXML SAX parser
- docs.rs metadata: categories, keywords, rustdoc-args for clean rendering
- Golden test suite: known DOCX files with expected output snapshots

### MCP

- `resources/subscribe` notification support
- Configurable root directory for `DirectoryResourceProvider`

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
