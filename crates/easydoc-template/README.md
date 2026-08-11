<a id="readme-top"></a>

<div align="center">

# easydoc-template

**DOCX template fill engine for the easydoc-rust workspace**

[![Crates.io](https://img.shields.io/crates/v/easydoc-template)](https://crates.io/crates/easydoc-template)
[![docs.rs](https://img.shields.io/docsrs/easydoc-template)](https://docs.rs/easydoc-template)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#rust-baseline)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[Overview](#1-overview) | [Template Semantics](#2-template-semantics) | [Quick Start](#3-quick-start) |
[Configuration](#4-configuration) | [Upstream Mapping](#5-upstream-compatibility) | [Quality](#6-quality)

</div>

---

> **Current version**: `0.1.0-alpha.1`
> **MSRV**: Rust `1.88`
> **Edition**: `2024`
> **Maturity**: Preview
> **Last verified**: 2026-08-11

---

## 1. Overview

**easydoc-template detects `{key}` and `{.field}` placeholders in DOCX templates and replaces them with provided data, preserving the ZIP structure.**

### 1.1 What it is

| Dimension | Value |
|---|---|
| Crate | `easydoc-template` |
| Current version | `0.1.0-alpha.1` |
| MSRV / Edition | `1.88` / `2024` |
| unsafe policy | `deny` (crate-level `#![deny(unsafe_code)]`) |
| License | `Apache-2.0` |

### 1.2 What it is not

- Not a full DOCX authoring library; it only performs placeholder replacement inside existing templates.
- Not a mail-merge engine; it does not split documents or handle conditional sections.
- Not a 1:1 port of Java poi-tl or hutool template; it adapts the fill concept to Rust's ownership model.

### 1.3 Processing pipeline

```text
DOCX template (ZIP)
        |
        v
PackageRewriter opens ZIP, validates limits
        |
        v
word/document.xml extracted as UTF-8
        |
        +-- Scalar:  {key}  ->  value
        +-- List:    {.field}  ->  replicated row per item
        |
        v
XML text replacement (cross-node aware)
        |
        v
New DOCX written via AtomicFile (temp + flush + sync + persist)
```

---

## 2. Template Semantics

### 2.1 Placeholder syntax

| Syntax | Type | Example | Behavior |
|---|---|---|---|
| `{key}` | Scalar | `{name}`, `{date}` | Replaced with a single value |
| `{.field}` | Collection | `{.name}`, `{.age}` | Row/paragraph replicated per item |
| `{prefix.field}` | Named collection | `{user.name}` | Field within a named group |

Placeholders spanning multiple `<w:t>` nodes (split by Word) are handled correctly.

### 2.2 Scope and expansion

| Scope | Expansion direction | Behavior |
|---|---|---|
| Paragraph (`<w:p>`) | Vertical | Paragraph replicated per collection item |
| Table row (`<w:tr>`) | Vertical | Row replicated per collection item |

### 2.3 Style inheritance

Filled cells inherit the placeholder cell's paragraph and run properties by default (controlled by `FillConfig.auto_style`).

### 2.4 Missing and empty values

- Missing scalar keys: placeholder text is preserved unchanged.
- Empty values: replaced with an empty string.
- XML special characters in values (`&`, `<`, `>`, `"`, `'`) are escaped.

### 2.5 Idempotency

Each `fill_template` / `fill_template_list` call reads the template fresh and writes a new file. The template file is never modified in place. Multiple calls with the same input produce identical output.

---

## 3. Quick Start

### 3.1 Installation

```toml
[dependencies]
easydoc-template = "0.1.0-alpha.1"
serde = { version = "1", features = ["derive"] }
```

### 3.2 Scalar fill

```rust
use std::collections::HashMap;
use easydoc_template::fill_template;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = HashMap::new();
    data.insert("name".to_owned(), "Alice".to_owned());
    data.insert("date".to_owned(), "2026-08-11".to_owned());

    fill_template(
        std::path::Path::new("template.docx"),
        std::path::Path::new("output.docx"),
        &data,
    )?;
    Ok(())
}
```

### 3.3 List fill

```rust
use serde::Serialize;
use easydoc_template::fill_template_list;

#[derive(Serialize, Debug)]
struct Item {
    name: String,
    age: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let items = vec![
        Item { name: "Alice".into(), age: 30 },
        Item { name: "Bob".into(), age: 25 },
    ];

    fill_template_list(
        std::path::Path::new("template.docx"),
        std::path::Path::new("output.docx"),
        &items,
        "items",
    )?;
    Ok(())
}
```

### 3.4 Builder API

```rust
use easydoc_template::TemplateFillBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    TemplateFillBuilder::new("template.docx", "output.docx")
        .register("title", "Monthly Report")
        .register("author", "Finance Team")
        .do_fill()?;
    Ok(())
}
```

---

## 4. Configuration

### 4.1 FillConfig

| Field | Type | Default | Description |
|---|---|---|---|
| `direction` | `FillDirection` | `Vertical` | Expansion direction for collections |
| `force_new_row` | `bool` | `true` | Insert a new row per collection item |
| `auto_style` | `bool` | `true` | Inherit placeholder cell styles |

### 4.2 FillDirection

| Variant | Behavior |
|---|---|
| `Vertical` | Collection items expand as new rows (default) |
| `Horizontal` | Collection items expand as new columns |

---

## 5. Upstream Compatibility

**Upstream**: Java [EasyExcel](https://github.com/alibaba/easyexcel) 4.0.3 template fill (`easyexcel-template`)

| Java capability | Rust equivalent | Status | Difference |
|---|---|---|---|
| `{key}` scalar fill | `fill_template()` | Stable | -- |
| `{.field}` collection fill | `fill_template_list()` | Stable | -- |
| Style inheritance | `FillConfig.auto_style` | Stable | -- |
| Vertical expansion | `FillDirection::Vertical` | Stable | -- |
| Horizontal expansion | `FillDirection::Horizontal` | Stable | -- |
| `FillConfig` builder | `TemplateFillBuilder` | Stable | Method chaining |

---

## 6. Quality

### 6.1 Build gates

```bash
cargo fmt --all -- --check
cargo clippy -p easydoc-template -- -D warnings
cargo check -p easydoc-template
cargo test -p easydoc-template
```

### 6.2 Test types

| Type | Purpose | Scope |
|---|---|---|
| Unit tests | Placeholder parsing, XML escaping, cross-node replacement | `fill_executor.rs` |
| Integration tests | End-to-end template fill with real DOCX files | `tests/` |
| Doc tests | Public API examples | `cargo test --doc` |

---

## 7. Project Structure

```text
crates/easydoc-template/
├── Cargo.toml
└── src/
    ├── lib.rs                 # Public API re-exports
    ├── fill_config.rs         # FillConfig, FillDirection
    ├── fill_executor.rs       # Core fill logic, cross-node replacement
    ├── fill_template.rs       # Scalar fill entry point
    ├── fill_template_list.rs  # List fill entry point
    └── placeholder.rs         # Placeholder detection and parsing
```

---

## 8. License

Licensed under [Apache-2.0](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE).

---

<div align="center">

[Back to top](#readme-top) · [docs.rs](https://docs.rs/easydoc-template) · [crates.io](https://crates.io/crates/easydoc-template) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
