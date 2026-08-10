# Roadmap

> Updated: 2026-08-10 | Synced with README.md

---

## Phase 1 -- Infrastructure ✅ Done

- [x] 8-crate workspace structure (`easydoc`, `easydoc-core`, `easydoc-derive`, `easydoc-ooxml`, `easydoc-reader`, `easydoc-writer`, `easydoc-template`, `easydoc-markdown`)
- [x] `easydoc-ooxml`: atomic file write (temp file + persist)
- [x] `PackageLimits`: ZIP entry count, single entry size, total size, compression ratio limits
- [x] `PackageRewriter`: safe ZIP rewrite, unmodified entries preserved byte-for-byte
- [x] Template XML special character escaping (`&`, `<`, `>`, `"`, `'`)
- [x] Cross `<w:t>` node scalar placeholder replacement
- [x] H1-H6 heading write with Word heading styles + outline level
- [x] A4 page, margins, typed units, Chinese font slots

## Phase 2 -- Semantic Model & Markdown ✅ Done

- [x] `DocumentContent` / `DocumentBlock` backend-agnostic semantic model
- [x] `read_document()` converts DOC/DOCX to `DocumentContent`
- [x] `easydoc-markdown`: DOC/DOCX -> Markdown conversion
  - [x] Headings, rich text, hyperlinks
  - [x] GFM tables (pipe escaping, auto column width)
  - [x] Merged cells -> HTML `<table>` + degradation warning
  - [x] Ordered/unordered nested lists
  - [x] Code blocks, footnotes, endnotes
  - [x] Image extraction (configurable directory and reference prefix)
  - [x] YAML front matter
  - [x] Atomic file output
- [x] Writer uses `easydoc-core` semantic model (via `content_renderer`)

## Phase 3 -- Event Chain & Advanced Read ✅ Done

- [x] `DocumentEvent` enum (Heading, Paragraph, Table, Image, PageBreak, etc.)
- [x] `EventSink` trait + SAX streaming read
- [x] `DocumentReader` trait (`read_model()` + `read_events()`)
- [x] `DocWriteHandler` callback integration (`render_with_handler`)
- [x] Writer refactored to use `content_renderer` + core model
- [x] `#[derive(DocxRow)]` proc-macro with full annotation support

## Phase 3.5 -- Derive Macro & ViewMode & SAX Coverage ✅ Done

- [x] Derive macro annotations (`width`, `format`, `align`, `wrap`, `converter`) fully wired to OOXML output
- [x] ViewMode rendering: Plain, Annotated (LLM-friendly), Outline, Stats
- [x] SAX content coverage:
  - [x] OMML formulas (`<m:oMath>` inline + `<m:oMathPara>` display)
  - [x] Lists (`<w:numPr>` + `numbering.xml` parsing, ordered now correct)
  - [x] Hyperlinks (`<w:hyperlink>` rId resolution to real URL)
  - [x] Nested tables
  - [x] Merged cells (gridSpan + vMerge)
  - [x] Image binary extraction from `word/media/*` via rels mapping

## Phase 4 -- Advanced Capabilities

- [ ] Equations (OMML -> LaTeX conversion)
- [ ] Comments and revision tracking
- [ ] Conditional template engine
- [ ] Image template engine
- [ ] Markdown source map (Markdown <-> source position)

## Phase 5 -- Ecosystem

- [ ] `easydoc-cli` command-line tool
- [ ] `easydoc-mcp` MCP integration
- [ ] `easydoc-web` web response adapter
- [ ] Benchmarks (Criterion), golden tests, fuzz tests
- [ ] `tests/fixtures/` real document collection

---

## Non-Goals

- Legacy `.doc` binary format write (read-only via `office_oxide`)
- Full Word layout/rendering engine
- DOCX to PDF pixel-perfect conversion
- Formula recalculation, collaborative editing, full revision tracking
- OCR/LLM image description (injected via trait, not default dependency)
