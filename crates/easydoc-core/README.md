<a id="readme-top"></a>

<div align="center">

# easydoc-core

**Core types, traits, and error model for the easydoc-rust DOC/DOCX document operations workspace.**

[![Crates.io](https://img.shields.io/crates/v/easydoc-core)](https://crates.io/crates/easydoc-core)
[![docs.rs](https://img.shields.io/docsrs/easydoc-core)](https://docs.rs/easydoc-core)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#3-rust-baseline--platform-support)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[Positioning](#1-project-positioning--status) · [Traits](#2-extension-traits) ·
[Data Model](#3-data-model) · [Errors](#4-error-model) · [Features](#5-cargo-features) ·
[Upstream](#6-upstream-compatibility) · [License](#8-license)

</div>

---

> **Status**: alpha pre-release (latest on [crates.io](https://crates.io/crates/easydoc-core))
> **MSRV**: Rust `1.88`
> **Edition**: `2024`
> **Maturity**: Alpha
> **Last verified**: 2026-08-11

## 1. Project Positioning & Status

### 1.1 What It Is

**`easydoc-core` is the foundation crate of the `easydoc-rust` workspace.** It defines the 6 extension traits, the semantic document data model, the unified error type, and the style/metadata primitives that all other crates depend on.

| Dimension | Value |
|---|---|
| Crate | `easydoc-core` |
| Status | Alpha pre-release (latest on crates.io) |
| MSRV / Edition | `1.88` / `2024` |
| Default features | `[]` (empty) |
| Optional features | `serde` |
| `unsafe` policy | `deny` (crate-level) |
| License | `Apache-2.0` |

### 1.2 What It Is Not

- Not a DOCX parser or generator -- those live in `easydoc-reader` and `easydoc-writer`.
- Not the user-facing entry point -- use `easydoc` (the facade crate) for that.
- Not coupled to any specific backend (`docx-rs`, `office_oxide`, etc.).

### 1.3 Status Evidence

| Claim | Value | Evidence |
|---|---|---|
| Crate builds | Yes | `cargo check -p easydoc-core` |
| Tests | Unit tests in each module | `cargo test -p easydoc-core` |
| MSRV | `1.88` | `rust-version` in `Cargo.toml` |
| `unsafe_code` | `deny` | crate-level lint |

## 2. Extension Traits

The 6 extension traits form the backbone of `easydoc-rust` extensibility. They correspond to the extension points in Java EasyExcel 4.0.3.

| Trait | Purpose | Java EasyExcel Equivalent | Defined In |
|---|---|---|---|
| `DocxRow` | Struct <-> table row bidirectional mapping | `@ExcelProperty` + reflection | `traits.rs` |
| `DocConverter<T>` | Rust type <-> `DocValue` conversion | `Converter<T>` interface | `traits.rs` |
| `DocReadListener<T>` | Streaming read callbacks | `ReadListener<T>` | `traits.rs` |
| `DocWriteHandler` | Write lifecycle hooks (document/paragraph/table/cell) | `WriteHandler` | `traits.rs` |
| `DocumentReader` | Unified read entry trait (backend abstraction) | -- (easydoc-rust original) | `traits.rs` |
| `EventSink` | SAX event consumption interface | `ReadListener<T>` callbacks | `traits.rs` |

### 2.1 DocxRow

Maps a Rust struct to/from DOCX table rows. Typically implemented via `#[derive(DocxRow)]` from `easydoc-derive`.

```rust,ignore
#[derive(DocxRow)]
struct User {
    #[docx(name = "Name", order = 0)]
    name: String,
    #[docx(name = "Age", order = 1)]
    age: u32,
}
```

Methods: `schema()`, `from_row()`, `from_row_with_converters()`, `to_row()`, `to_row_with_converters()`.

### 2.2 DocConverter\<T\>

Bidirectional conversion between a Rust type `T` and `DocValue`. Registered via `ConverterRegistry` or builder's `register_converter`.

```rust,ignore
impl DocConverter<String> for MyConverter {
    fn support_type() -> TypeId { TypeId::of::<String>() }
    fn to_doc_value(&self, value: &String, col: &TableColumn) -> Result<DocValue> { ... }
    fn from_doc_value(&self, value: &DocValue, col: &TableColumn) -> Result<String> { ... }
}
```

### 2.3 DocReadListener\<T\>

Receives parsed content during streaming read. Methods: `invoke()`, `invoke_table()`, `on_complete()`, `on_error()`, `has_next()`.

### 2.4 DocWriteHandler

Write lifecycle interceptor at document, paragraph, table, and cell levels. All methods have empty default implementations. Methods: `order()`, `before_document()`, `after_document()`, `before_paragraph()`, `after_paragraph()`, `before_table()`, `after_table()`, `before_cell()`, `after_cell()`.

### 2.5 DocumentReader

Backend-agnostic read interface. Implementations provide `read_model()` and `read_events()`. No direct Java equivalent -- this is an `easydoc-rust` original abstraction.

### 2.6 EventSink

Consumes `DocumentEvent` instances during SAX streaming. The built-in `ContentCollector` implementation collects events into `DocumentContent`.

Event types: `DocumentStart`, `Heading`, `Paragraph`, `Table`, `List`, `Image`, `PageBreak`, `ColumnBreak`, `CodeBlock`, `Section`, `DocumentEnd`.

## 3. Data Model

The semantic document model is backend-independent -- it has no direct Java EasyExcel equivalent (Java EasyExcel does not process DOCX).

### 3.1 Model Hierarchy

```text
DocumentContent
├── metadata: DocumentMeta (title, author, ...)
└── blocks: Vec<DocumentBlock>
    ├── Heading { level, runs }
    ├── Paragraph(runs)
    ├── Table(DocumentTable)
    │   └── rows: Vec<DocumentTableRow>
    │       └── cells: Vec<DocumentTableCell>
    │           └── blocks: Vec<DocumentBlock>
    ├── List(DocumentList)
    │   └── items: Vec<DocumentListItem>
    ├── Image(DocumentImage)
    ├── CodeBlock { language, code }
    ├── TextBox(blocks)
    ├── Footnote { id, blocks }
    ├── Endnote { id, blocks }
    ├── Section { blocks, section_type }
    ├── Math { latex, display }
    ├── ThematicBreak
    ├── PageBreak
    └── ColumnBreak
```

### 3.2 Key Types

| Type | Purpose |
|---|---|
| `DocumentContent` | Top-level document: metadata + blocks |
| `DocumentBlock` | Enum of all block types (paragraph, table, list, image, etc.) |
| `DocumentTextRun` | Rich text segment (text + bold/italic/size/color/font/strikethrough/hyperlink) |
| `DocumentTable` | Table with rows |
| `DocumentTableRow` | Table row with cells + `is_header` flag |
| `DocumentTableCell` | Cell with nested blocks + merge spans (`grid_span`, `v_merge`) |
| `DocumentList` | Ordered/unordered list with items |
| `DocumentImage` | Image with alt text, extension, and binary data |
| `DocumentMeta` | Document metadata (title, author, description) |

### 3.3 Data Types (DocValue)

`DocValue` is the universal value enum bridging Rust types and DOCX content.

| Variant | Rust Type | Notes |
|---|---|---|
| `String(String)` | `String` / `&str` | Plain text |
| `Bool(bool)` | `bool` | Boolean |
| `Int(i64)` | `i32` / `u32` / `i64` | Integer |
| `Float(f64)` | `f64` | Floating point |
| `DateTime(DateTime<Utc>)` | `chrono::DateTime<Utc>` | UTC datetime |
| `Date(NaiveDate)` | `chrono::NaiveDate` | Date only |
| `NaiveDateTime(NaiveDateTime)` | `chrono::NaiveDateTime` | Timezone-free datetime |
| `Empty` | `Option::None` | Null value |
| `RichText(Vec<RichRun>)` | -- | Formatted text segments |
| `Image(ImageData)` | -- | Image bytes + metadata |

`From` implementations are provided for `String`, `&str`, `bool`, `i32`, `u32`, `i64`, `f64`, `DateTime<Utc>`, `NaiveDate`, `NaiveDateTime`, and `Option<T>`.

### 3.4 Supporting Types

| Type | Purpose |
|---|---|
| `CellData` | Single table cell: value + alignment + merge spans |
| `RowData` | Row of cells + height hint |
| `TableData` | Extracted table: optional headers + rows of strings |
| `HeadingLevel` | H1..H6 enum |
| `HorizontalAlignment` | Left / Center / Right / Both |
| `ErrorAction` | Continue / Skip / Stop (for read listeners) |
| `TableColumn` | Column metadata: name, index, format, width |

## 4. Error Model

All operations return `easydoc_core::Result<T>` (alias for `Result<T, DocError>`).

| Variant | Scenario | Java Equivalent | Source |
|---|---|---|---|
| `DocError::Io` | File or network I/O | `IOException` | `std::io::Error` |
| `DocError::Zip` | ZIP archive error | `ExcelAnalysisException` (ZIP) | `zip::ZipError` |
| `DocError::Format` | Invalid/unsupported format | `ExcelAnalysisException` | -- |
| `DocError::Template` | Placeholder parse/process error | `ExcelAnalysisException` (template) | -- |
| `DocError::Conversion` | Cell/field value conversion failure | `ExcelDataConvertException` | -- |
| `DocError::Unsupported` | Operation not supported | `UnsupportedOperationException` | -- |
| `DocError::Document` | General document error | `ExcelAnalysisException` / `ExcelGenerateException` | -- |

Java EasyExcel spreads errors across 7 `RuntimeException` subclasses; `easydoc-core` unifies them into a single idiomatic Rust enum.

## 5. Cargo Features

| Feature | Default | Effect | Dependencies |
|---|:---:|---|---|
| `serde` | No | Enables `serde::Serialize`/`Deserialize` on data model types | `serde`, `serde_json` |

```toml
# Minimal (no serde)
[dependencies]
easydoc-core = "0.1.0-alpha"

# With serde support
easydoc-core = { version = "0.1.0-alpha", features = ["serde"] }
```

## 6. Upstream Compatibility

`easydoc-core` maps its trait system to Java EasyExcel 4.0.3 extension points.

### 6.1 Trait Mapping

| Java EasyExcel 4.0.3 | Rust easydoc-core | Mapping Type |
|---|---|---|
| `@ExcelProperty` annotation + reflection | `DocxRow` trait + derive macro | Idiomatic replacement |
| `Converter<T>` interface | `DocConverter<T>` trait | Behavioural equivalent |
| `ReadListener<T>` | `DocReadListener<T>` + `EventSink` | Behavioural equivalent |
| `WriteHandler` | `DocWriteHandler` | Behavioural equivalent |
| `ReadCellData` / `WriteCellData` | `DocValue` enum | Idiomatic replacement |
| `ExcelAnalysisException` etc. | `DocError` enum | Unified replacement |

### 6.2 Language Semantic Mapping

| Java Mechanism | Rust Design | Reason |
|---|---|---|
| Checked/unchecked exceptions | `Result<T, DocError>` | Explicit error propagation |
| `null` | `Option<T>` | Null-safety |
| Annotations + reflection | Trait + derive macro | Compile-time metadata |
| Interface inheritance | Trait + composition | Explicit capability boundaries |
| Global singleton | `OnceLock<Arc<_>>` or explicit context | Lifecycle and test isolation |

## 7. Build & Test

```bash
cargo check -p easydoc-core
cargo test -p easydoc-core
cargo test -p easydoc-core --features serde
cargo clippy -p easydoc-core -- -D warnings
cargo doc -p easydoc-core --no-deps
```

## 8. License

Apache-2.0 -- see [LICENSE](../../LICENSE) for details.

---

<div align="center">

[Back to top](#readme-top) · [docs.rs](https://docs.rs/easydoc-core) · [crates.io](https://crates.io/crates/easydoc-core) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
