<a id="readme-top"></a>

<div align="center">

# easydoc-derive

**Derive macros for typed DOCX table row mapping in the easydoc-rust workspace**

[![Crates.io](https://img.shields.io/crates/v/easydoc-derive)](https://crates.io/crates/easydoc-derive)
[![docs.rs](https://img.shields.io/docsrs/easydoc-derive)](https://docs.rs/easydoc-derive)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#rust-baseline)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[Overview](#1-overview) | [Quick Start](#2-quick-start) | [Attribute Reference](#3-attribute-reference) |
[Upstream Mapping](#4-upstream-compatibility) | [Quality](#5-quality)

</div>

---

> **Status**: alpha pre-release (latest on [crates.io](https://crates.io/crates/easydoc-derive))
> **MSRV**: Rust `1.88`
> **Edition**: `2024`
> **Maturity**: Preview
> **Last verified**: 2026-08-11

---

## 1. Overview

**easydoc-derive is a proc-macro crate that provides `#[derive(DocxRow)]` for mapping Rust structs to DOCX table rows in the easydoc-rust workspace.**

### 1.1 What it is

| Dimension | Value |
|---|---|
| Crate | `easydoc-derive` |
| Status | Alpha pre-release (latest on crates.io) |
| MSRV / Edition | `1.88` / `2024` |
| Type | proc-macro crate |
| unsafe policy | `forbid` (workspace lint) |
| License | `Apache-2.0` |

### 1.2 What it is not

- Not a standalone DOCX generator; it requires `easydoc-core` for the `DocxRow` trait and supporting types.
- Not a general-purpose serialization framework; it is purpose-built for DOCX table rows.
- Not a 1:1 port of Java EasyExcel annotations; it adapts the annotation-driven model to Rust derive macros.

### 1.3 Status

| Claim | Evidence |
|---|---|
| Compiles | `cargo check -p easydoc-derive` |
| Tests | trybuild compile-fail tests + unit tests |
| MSRV | CI MSRV job (Rust 1.88) |
| crates.io | Published | [crates.io](https://crates.io/crates/easydoc-derive) / [docs.rs](https://docs.rs/easydoc-derive) |

---

## 2. Quick Start

### 2.1 Installation

```toml
[dependencies]
easydoc-derive = "0.1.0-alpha"
easydoc-core = "0.1.0-alpha"
```

### 2.2 Minimal example

```rust
use easydoc_derive::DocxRow;

#[derive(DocxRow)]
#[docx(banded_rows = true)]
struct Report {
    #[docx(name = "ID", order = 0, width = "2cm")]
    id: u32,

    #[docx(name = "Amount", order = 1, format = "#,##0.00", align = "right")]
    amount: f64,

    #[docx(name = "Date", order = 2, format = "yyyy-mm-dd")]
    date: String,

    #[docx(name = "Status", order = 3, converter = StatusConverter)]
    status: String,

    #[docx(name = "Note", order = 4, wrap = true)]
    note: Option<String>,

    #[docx(ignore)]
    internal_id: String,
}
```

The derive generates:
- `schema()` -- returns `&'static [TableColumn]` with column metadata
- `from_row()` / `from_row_with_converters()` -- deserialize a `RowData` into the struct
- `to_row()` / `to_row_with_converters()` -- serialize the struct into `Vec<CellData>`

---

## 3. Attribute Reference

### 3.1 Field attributes

| Attribute | Type | Default | Description |
|---|---|---|---|
| `name` | string literal | field name | Column header text |
| `index` | integer literal | declaration order | Zero-based column index |
| `order` | integer literal | declaration order | Column sort order (lower = left) |
| `width` | string literal | none | Column width (`"2cm"`, `"80px"`, `"auto"`) |
| `format` | string literal | none | Number/date format (`"#,##0.00"`, `"yyyy-mm-dd"`) |
| `align` | string literal | none | Horizontal alignment: `left`, `center`, `right`, `justify`, `both` |
| `converter` | type path | none | Custom converter type (must implement the converter trait) |
| `wrap` | bool literal | `false` | Enable text wrapping |
| `ignore` | flag | — | Skip this field during read and write |

### 3.2 Struct attributes

| Attribute | Type | Default | Description |
|---|---|---|---|
| `banded_rows` | bool literal | `false` | Enable zebra-striping for the table |
| `table_width` / `auto_width` | bool literal | `false` | Auto-fit table width |

### 3.3 Attribute to OOXML mapping

| Attribute | Generated code | OOXML effect |
|---|---|---|
| `name` | `TableColumn.name` | `<w:t>` text content in header row |
| `width` | `TableColumn.width` | `<w:tcW>` cell width |
| `format` | `TableColumn.format` | `<w:numFmt>` or display format |
| `align` | `HorizontalAlignment::*` | `<w:jc>` justification |
| `wrap` | `TableColumn.wrap` | `<w:tcPr><w:wrap/>` |
| `converter` | dispatched via `ConverterRegistry` | Custom value transformation |
| `ignore` | field excluded from schema and row | Field not present in output |

### 3.4 Custom converter example

```rust
use easydoc_core::{DocValue, TableColumn, Converter};

pub struct StatusConverter;

impl Converter<String> for StatusConverter {
    fn to_doc_value(&self, value: &String, _col: &TableColumn) -> easydoc_core::Result<DocValue> {
        let display = match value.as_str() {
            "active" => "Active",
            "inactive" => "Inactive",
            other => other,
        };
        Ok(DocValue::String(display.to_owned()))
    }

    fn from_doc_value(&self, value: &DocValue, _col: &TableColumn) -> easydoc_core::Result<String> {
        match value {
            DocValue::String(s) => Ok(s.clone()),
            other => Ok(format!("{other:?}")),
        }
    }
}
```

---

## 4. Upstream Compatibility

**Upstream**: Java [EasyExcel](https://github.com/alibaba/easyexcel) 4.0.3

| Java mechanism | Rust design | Reason |
|---|---|---|
| `@ExcelProperty` annotation | `#[derive(DocxRow)]` + `#[docx(...)]` | Compile-time metadata, no reflection |
| Reflection-based read/write | Generated `from_row()` / `to_row()` | Static dispatch, type safety |
| `Converter` interface | `Converter<T>` trait + `ConverterRegistry` | Explicit registration, no classpath scanning |
| `null` | `Option<T>` | Explicit nullable handling |
| Exceptions | `Result<T, DocError>` | No hidden control flow |

| Upstream capability | Rust equivalent | Status | Difference |
|---|---|---|---|
| Column name mapping | `name` attribute | Stable | -- |
| Column ordering | `order` attribute | Stable | -- |
| Column width | `width` attribute | Stable | -- |
| Number/date format | `format` attribute | Stable | -- |
| Alignment | `align` attribute | Stable | -- |
| Custom converter | `converter` attribute | Stable | Explicit registry, no classpath |
| Ignore field | `ignore` attribute | Stable | -- |
| Banded rows | `banded_rows` struct attr | Stable | -- |
| Auto width | `table_width` struct attr | Stable | -- |

---

## 5. Quality

### 5.1 Build gates

```bash
cargo fmt --all -- --check
cargo clippy -p easydoc-derive -- -D warnings
cargo check -p easydoc-derive
cargo test -p easydoc-derive
```

### 5.2 Test types

| Type | Purpose | Tool |
|---|---|---|
| Compile-fail | Invalid attribute detection | trybuild |
| Unit tests | Attribute parsing, alignment validation | `cargo test` |
| Doc tests | Public API examples | `cargo test --doc` |

---

## 6. Project Structure

```text
crates/easydoc-derive/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Entry point, derive_docx_row
│   └── implementation.rs   # Token expansion, attribute parsing
└── tests/
    └── trybuild/            # Compile-fail test cases
```

---

## 7. License

Licensed under [Apache-2.0](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE).

---

<div align="center">

[Back to top](#readme-top) · [docs.rs](https://docs.rs/easydoc-derive) · [crates.io](https://crates.io/crates/easydoc-derive) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
