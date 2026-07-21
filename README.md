# easydoc-rs

**Easy DOC/DOCX document operations in Rust.**  |  **Rust 快捷 DOC/DOCX 文档操作库。**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)

> `easydoc-rs` is the DOC/DOCX counterpart of [`easyexcel-rs`](https://github.com/hiwepy/easyexcel-rs), following the same fluent builder + trait extension + proc-macro architecture for ergonomic document manipulation.

---

## Features 功能

| Feature 功能 | Status |
|:---|:---:|
| **Write DOCX** — paragraphs, headings, tables, page breaks, styled runs | ✅ |
| **Quick Table Write** — `Vec<Struct>` -> DOCX table in one line | ✅ |
| **Template Fill** — `{key}` placeholder replacement with ZIP preservation | ✅ |
| **Read DOCX/DOC** — text extraction, table extraction via office_oxide | ✅ |
| **Format Auto-Detection** — DOCX (ZIP magic) vs DOC (OLE2 magic) | ✅ |
| **`#[derive(DocxRow)]`** — compile-time struct-to-row mapping | ✅ |
| **Style System** — FontConfig, ParagraphStyle, TableStyle, Color | ✅ |
| **Extensible Converters** — custom `DocConverter<T>` registry | ✅ |
| **Write Lifecycle Hooks** — `DocWriteHandler` at document/paragraph/table/cell level | ✅ |
| **Streaming Read Listener** — `DocReadListener<T>` event-driven parsing | ✅ |

---

## Quick Start 快速开始

Add to your `Cargo.toml`:

```toml
[dependencies]
easydoc = "0.1"
```

### Write a table from struct data 表格写入

```rust
use easydoc::prelude::*;

#[derive(DocxRow)]
#[docx(banded_rows = true)]
struct User {
    #[docx(name = "Name", width = 0.3, order = 0)]
    name: String,
    #[docx(name = "Age", width = 0.15, order = 1)]
    age: u32,
    #[docx(name = "Email", width = 0.55, order = 2)]
    email: String,
}

let users = vec![
    User { name: "Alice".into(), age: 30, email: "alice@e.com".into() },
    User { name: "Bob".into(), age: 25, email: "bob@e.com".into() },
];

EasyDoc::write_table("users.docx", &users)
    .title("User Report")
    .header_style(TableStyle::header())
    .banded_rows(true)
    .do_write()?;
```

### Build a full document 构建文档

```rust
EasyDoc::document("report.docx")
    .title("Annual Report")
    .author("Zhang San")
    .add_heading("Chapter 1: Overview", HeadingLevel::H1)
    .add_paragraph(
        Paragraph::new()
            .add_text("This is body text with ")
            .add_run(Run::new("highlighted").bold().color(0xFF0000))
            .add_text(" content.")
            .alignment(HorizontalAlignment::Both)
    )
    .add_table(Table::from_data(&users).banded_rows(true))
    .add_page_break()
    .save()?;
```

### Template fill 模板填充

```rust
use std::collections::HashMap;

let mut data = HashMap::new();
data.insert("name".into(), "Alice".into());
data.insert("date".into(), "2026-07-21".into());

EasyDoc::fill_template("template.docx", "output.docx", &data)?;
```

### Read documents 读取文档

```rust
// Extract all text
let text = EasyDoc::read_text("document.docx")?;

// Extract tables into typed structs
let tables: Vec<Vec<User>> = EasyDoc::read_tables::<User>("document.docx")?;

// Both DOCX and DOC are supported transparently
let text = EasyDoc::read_text("legacy.doc")?;
```

---

## Architecture 架构

```
easydoc-rs/
├── Cargo.toml                          workspace manifest
├── crates/
│   ├── easydoc/                        facade — EasyDoc static factory
│   ├── easydoc-core/                   core types, traits, errors, styles
│   ├── easydoc-derive/                 proc-macro #[derive(DocxRow)]
│   ├── easydoc-writer/                 DOCX generation via docx-rs
│   ├── easydoc-reader/                 DOCX/DOC reading via office_oxide
│   └── easydoc-template/               placeholder replacement, ZIP-preserving
```

For detailed architecture, see [docs/architecture.md](docs/architecture.md).

---

## Backend Dependencies 后端依赖

| Function 功能 | Crate | Version |
|:---|:---|:---|
| DOCX Write | [`docx-rs`](https://crates.io/crates/docx-rs) | 0.4.20 |
| DOCX/DOC Read | [`office_oxide`](https://crates.io/crates/office_oxide) | 0.1.7 |

---

## Design Principles 设计原则

| # | Principle 原则 | Inherited From 继承自 |
|:---|:---|:---|
| 1 | **Static Factory** — `EasyDoc` is the single entry point | easyexcel-rs `EasyExcel` |
| 2 | **Fluent Builder** — `mut self -> Self` with `#[must_use]` | easyexcel-rs builder pattern |
| 3 | **Trait Extension** — `DocxRow`, `DocConverter`, `DocWriteHandler`, `DocReadListener` | easyexcel-rs traits |
| 4 | **Proc-Macro Code Gen** — `#[derive(DocxRow)]` at compile time | easyexcel-rs `#[derive(ExcelRow)]` |
| 5 | **Backend Agnostic** — unified API, swappable engines | easyexcel-rs multi-format |
| 6 | **Single Error Type** — `DocError` enum with `thiserror` | easyexcel-rs `ExcelError` |
| 7 | **Zero Unsafe** — `#![forbid(unsafe_code)]` in every crate | easyexcel-rs safety policy |

---

## Testing 测试

```bash
# Run all tests (11 passing)
cargo test --workspace
```

Tests cover: write table, write document, round-trip write+read text, round-trip write+read table, template scalar fill with multiple placeholders, template fill end-to-end, image insertion, style builders, converters, format detection, error variants, lifecycle hooks.

---

## Documentation 文档

| Document | Description |
|:---|:---|
| [Usage Guide 使用指南](docs/usage-guide.md) | Comprehensive guide with real-world examples and API reference |
| [Architecture Design 架构设计](docs/architecture.md) | Full architecture, data flow, design decisions |
| [API Reference 接口速查](#10-api-reference-接口速查) | Quick-reference in the Usage Guide |

---

## License

Apache-2.0 — see [LICENSE](LICENSE) for details.

## Related Projects 相关项目

- [`easyexcel-rs`](https://github.com/hiwepy/easyexcel-rs) — the Excel counterpart
- [`easypdf-rs`](https://github.com/hiwepy/easypdf-rs) — PDF document operations
