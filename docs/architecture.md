# easydoc-rust Architecture Design Document &middot; 架构设计文档

> **Version**: 0.1.0 &nbsp;|&nbsp; **Date**: 2026-07-21 &nbsp;|&nbsp; **Status**: All Phases Complete
> **Author**: easydoc-rust team &nbsp;|&nbsp; **License**: Apache-2.0

---

## Table of Contents 目录

1. [Project Vision 项目愿景](#1-project-vision-项目愿景)
2. [Design Goals 设计目标](#2-design-goals-设计目标)
3. [Crate Architecture 包架构](#3-crate-architecture-包架构)
4. [Dependency Graph 依赖图](#4-dependency-graph-依赖图)
5. [Data Flow 数据流](#5-data-flow-数据流)
6. [Core Abstractions 核心抽象](#6-core-abstractions-核心抽象)
7. [Builder Pattern Design 构建器模式设计](#7-builder-pattern-design-构建器模式设计)
8. [Multi-Engine Strategy 多引擎策略](#8-multi-engine-strategy-多引擎策略)
9. [Error Handling 错误处理](#9-error-handling-错误处理)
10. [Template Engine 模板引擎](#10-template-engine-模板引擎)
11. [Derive Macro 派生宏](#11-derive-macro-派生宏)
12. [Conventions from easyexcel-rust 继承约定](#12-conventions-from-easyexcel-rust-继承约定)
13. [DOC vs Excel Paradigm Differences DOC与Excel范式差异](#13-doc-vs-excel-paradigm-differences-doc与excel范式差异)

---

## 1. Project Vision 项目愿景

`easydoc-rust` aims to provide **the same developer experience for DOC/DOCX operations** that
[easyexcel-rust](https://github.com/easy-4-rust/easyexcel-rust) provides for Excel:

> **Type-safe Builders + Compile-time Reflection + Multi-engine Backends = Ergonomic document manipulation in idiomatic Rust.**

The library covers three primary use cases:

| Use Case 用例 | Description 描述 | EasyExcel Analogy 类比 |
|:---|:---|:---|
| **Write** 写入 | Create new DOCX with paragraphs, tables, images, styles | `EasyExcel.write()` |
| **Read** 读取 | Extract text and tables from DOCX/DOC | `EasyExcel.read()` |
| **Fill** 填充 | Replace `{key}` placeholders in DOCX templates | `EasyExcel.fill()` |

---

## 2. Design Goals 设计目标

| # | Goal 目标 | Rationale 理由 |
|:---|:---|:---|
| G1 | **Pure Rust, zero unsafe** | `#![forbid(unsafe_code)]` in every crate. Aligns with easyexcel-rust's safety policy. |
| G2 | **Fluent Builder API** | `mut self -> Self` with `#[must_use]`. Method chains read like natural language. |
| G3 | **Multi-engine backend** | docx-rs for writing, office_oxide for reading -- swappable without API changes. |
| G4 | **Compile-time reflection** | `#[derive(DocxRow)]` replaces runtime annotation scanning. |
| G5 | **Extensibility via traits** | `DocxRow`, `DocReadListener`, `DocWriteHandler`, `DocConverter<T>` -- users plug in custom logic. |
| G6 | **Single error type** | `DocError` enum with `thiserror`, `type Result<T> = ...` -- no scattered error types. |
| G7 | **Separation of concerns** | Core types != engine implementations != facade. Each crate has one job. |
| G8 | **Follow easyexcel-rust conventions** | Naming, structure, quality gates -- consistency across the ecosystem. |

---

## 3. Crate Architecture 包架构

```
easydoc-rust/
├── Cargo.toml                     Virtual workspace (edition 2024, resolver="3")
│
├── crates/
│   ├── easydoc/                   FACADE -- user-facing entry point
│   │   ├── src/lib.rs             Re-exports, prelude module
│   │   └── src/easy_doc.rs        EasyDoc struct, all static factory methods
│   │
│   ├── easydoc-core/              CORE -- zero engine dependency
│   │   └── src/
│   │       ├── lib.rs             Flat re-exports
│   │       ├── error.rs           DocError enum (7 variants), Result<T> alias
│   │       ├── types.rs           DocValue, CellData, RowData, TableData, HeadingLevel, etc.
│   │       ├── traits.rs          DocxRow, DocConverter, DocReadListener, DocWriteHandler
│   │       ├── converter/
│   │       │   ├── mod.rs         Converter trait re-export
│   │       │   └── registry.rs    ConverterRegistry with TypeId-based dispatch
│   │       ├── style/
│   │       │   ├── color.rs       Color (RGB, from_hex, to_hex, named constants)
│   │       │   ├── font.rs        FontConfig (name, size, bold, italic, underline, color)
│   │       │   ├── paragraph.rs   ParagraphStyle (alignment, indent, spacing)
│   │       │   └── table.rs       TableStyle (header/content fonts, banded, borders)
│   │       └── metadata/
│   │           ├── column.rs      TableColumn (name, field_name, index, order, width, format)
│   │           └── document.rs    DocumentMeta (title, author, subject, page size)
│   │
│   ├── easydoc-derive/            PROC-MACRO -- compile-time code gen
│   │   └── src/
│   │       ├── lib.rs             #[proc_macro_derive(DocxRow, attributes(docx))]
│   │       └── implementation.rs  Parsing #[docx(...)] attributes, generating impl blocks
│   │
│   ├── easydoc-writer/            WRITER -- docx-rs backend
│   │   └── src/
│   │       ├── lib.rs             Paragraph, Run, Table, DocImage builders
│   │       ├── builder/
│   │       │   ├── doc_builder.rs        DocBuilder (add_heading, add_paragraph, add_table, ...)
│   │       │   └── table_builder.rs      TableWriteBuilder<T> (one-liner table write)
│   │       ├── executor/
│   │       │   ├── write_executor.rs     DocWriteExecutor (render builder -> docx-rs -> file)
│   │       │   └── table_executor.rs     TableWriteExecutor<T> (render data -> docx-rs table)
│   │       ├── handler/                  DocWriteHandler trait re-export
│   │       └── style/
│   │           ├── banded_rows.rs        Zebra striping strategy
│   │           └── auto_width.rs         Column width calculation
│   │
│   ├── easydoc-reader/            READER -- office_oxide backend
│   │   └── src/
│   │       ├── lib.rs             read_text(), read_tables<T>(), DocReadBuilder, detect_format()
│   │       ├── builder/
│   │       │   └── read_builder.rs       DocReadBuilder (do_read<T>())
│   │       ├── extractor/
│   │       │   ├── mod.rs                DocumentFormat enum, detect_format()
│   │       │   ├── text.rs               extract_text() via office_oxide::Document::plain_text()
│   │       │   └── table.rs              extract_tables<T>() via office_oxide IR
│   │       └── listener/
│   │           └── collect.rs            CollectListener<T> for sync reads
│   │
│   └── easydoc-template/          TEMPLATE -- ZIP-preserving fill
│       └── src/
│           ├── lib.rs             fill_template(), fill_template_list()
│           ├── placeholder.rs     Placeholder detection ({key}, {.field}, {prefix.field})
│           ├── fill_executor.rs   ZIP-level modification, scalar + collection expansion
│           └── fill_config.rs     FillConfig (direction, force_new_row, auto_style)
```

### Crate responsibility matrix 包职责矩阵

| Crate | External deps | Depends on | Role |
|:---|:---|:---|:---|
| **easydoc** | serde | all sub-crates | Builder entry points + re-exports |
| **easydoc-core** | thiserror, chrono, zip | -- | Shared types, traits, errors |
| **easydoc-derive** | syn, quote, proc-macro2 | -- (dev: easydoc-core) | Derive macro only |
| **easydoc-writer** | docx-rs | easydoc-core | DOCX creation |
| **easydoc-reader** | office_oxide | easydoc-core | DOCX/DOC text and table extraction |
| **easydoc-template** | serde, serde_json, zip | easydoc-core, easydoc-writer | Template placeholder fill |

---

## 4. Dependency Graph 依赖图

```
                        ┌──────────┐
                        │ easydoc  │  <-- user depends on this
                        │ (facade) │
                        └────┬─────┘
           ┌─────┬─────┬─────┼─────┬─────┐
           v     v     v     v     v     v
         core  derive writer reader tmpl
          │              │      │      │
          │           docx-rs office  zip
          │                   oxide
          │
      thiserror, chrono, zip
```

Key observations:

1. **easydoc-core has zero writer/reader engine dependencies** -- it defines *what* a document element is, not *how* to render it.
2. **Writer and Reader use different engines** -- docx-rs (write) and office_oxide (read), mirroring easyexcel-rust's calamine + rust_xlsxwriter split.
3. **Template shares zip with core** -- ZIP manipulation used for both error conversion and template fill.
4. **Facade depends on everyone** -- it wires sub-crates together and provides the ergonomic `EasyDoc::document()` / `EasyDoc::read()` entry points.

---

## 5. Data Flow 数据流

### 5.1 Write Flow 写入流程

```
User code                    easydoc facade              easydoc-writer           docx-rs
---------                    --------------              ---------------           -------
EasyDoc::document("out.docx")
  .add_paragraph(...)
  .add_table(...)
  .save()?
        │
        v
  DocBuilder::save()
        │
        ├── DocBuilder::build() -> DocWriteExecutor
        │
        └── DocWriteExecutor::save()
              │
              ├── Create docx_rs::Docx::new()
              ├── For each element:
              │    ├── Heading    -> docx_rs::Paragraph + Run(bold, size=28)
              │    ├── Paragraph  -> docx_rs::Paragraph + Runs(with fonts/styles)
              │    ├── Table      -> docx_rs::Table + TableRows + TableCells
              │    ├── Image      -> (future: docx_rs::Pic)
              │    └── PageBreak  -> docx_rs::Run::add_break(BreakType::Page)
              ├── docx.build() -> XMLDocx
              └── xml_docx.pack(File) -> valid .docx written to disk
```

### 5.2 Quick Table Write Flow 快捷表格写入

```
User code                    easydoc facade              easydoc-writer           docx-rs
---------                    --------------              ---------------           -------
EasyDoc::write_table("out.docx", &users)
  .title("Report")
  .header_style(...)
  .do_write()?
        │
        v
  TableWriteBuilder::do_write()
        │
        └── TableWriteExecutor::execute()
              │
              ├── Create docx_rs::Docx::new()
              ├── Optional title heading paragraph
              ├── Build header row from T::schema()
              ├── Build data rows from item.to_row() for each item
              ├── docx.add_table(Table::new(rows))
              └── docx.build().pack(file)
```

### 5.3 Read Flow 读取流程

```
User code                    easydoc facade              easydoc-reader            office_oxide
---------                    --------------              ---------------            ------------
EasyDoc::read_text("in.docx")
        │
        v
  read_text(path)
        │
        └── extractor::text::extract_text(path)
              │
              ├── office_oxide::Document::open(path)  <-- unified DOCX/DOC parser
              └── doc.plain_text() -> String

EasyDoc::read_tables::<T>("in.docx")
        │
        v
  read_tables::<T>(path)
        │
        └── extractor::table::extract_tables::<T>(path)
              │
              ├── office_oxide::Document::open(path)
              ├── doc.to_ir() -> DocumentIR
              ├── For each Section -> Element::Table:
              │    ├── Skip header rows (is_header=true)
              │    ├── For each TableRow -> TableCell:
              │    │    └── Flatten Paragraph -> InlineContent::Text -> String
              │    ├── Build RowData from cell strings
              │    └── T::from_row(&RowData) -> Result<T>
              └── Return Vec<Vec<T>>
```

### 5.4 Template Fill Flow 模板填充流程

```
User code                    easydoc facade              easydoc-template          zip crate
---------                    --------------              ----------------          ---------
EasyDoc::fill_template("tpl.docx", "out.docx", &data)
        │
        v
  fill_template(template, output, data)
        │
        └── fill_scalar(template, output, data)
              │
              ├── Read template bytes
              ├── Open as ZipArchive (read)
              ├── Create ZipWriter (write)
              ├── For each ZIP entry:
              │    ├── If "word/document.xml":
              │    │    └── replace_scalar_placeholders(xml, data)
              │    │         ├── Placeholder::find_all(xml)
              │    │         └── For each {key}: xml.replace("{key}", data[key])
              │    └── Else: copy entry unchanged
              ├── Write all entries to output ZIP
              └── finish() -> valid DOCX with replaced placeholders
```

---

## 6. Core Abstractions 核心抽象

### 6.1 Type Hierarchy 类型层级

```
DocError (enum)           -- Central error type, 7 variants
  ├── Io(io::Error)       -- Wraps std I/O errors
  ├── Zip(String)         -- ZIP packaging errors from docx-rs
  ├── Format(String)      -- Invalid document format
  ├── Template {...}      -- Placeholder resolution failures
  ├── Conversion {...}    -- Type conversion errors (field, value, message)
  ├── Unsupported(String) -- Operation not available
  └── Document(String)    -- Generic document-level errors

DocValue (enum)           -- Universal value bridge (like CellValue in easyexcel-rust)
  ├── String(String)
  ├── Bool(bool)
  ├── Int(i64)
  ├── Float(f64)
  ├── DateTime(DateTime<Utc>)
  ├── Date(NaiveDate)
  ├── NaiveDateTime
  ├── Empty
  ├── RichText(Vec<RichRun>)
  └── Image(ImageData)

CellData (struct)         -- value: DocValue, alignment, col_span, row_span
RowData (struct)          -- cells: Vec<CellData>, height
TableData (struct)        -- headers: Option<Vec<String>>, rows: Vec<Vec<String>>

Color (struct)            -- r: u8, g: u8, b: u8 + from_hex/to_hex + named constants
FontConfig (struct)       -- name, size (half-pts), bold, italic, underline, color
ParagraphStyle (struct)   -- alignment, first_line_indent, left/right indent, spacing
TableStyle (struct)       -- header_font, content_font, header_background, banded_rows, borders

TableColumn (struct)      -- name, field_name, index, order, width, format, ignored
DocumentMeta (struct)     -- title, author, subject, keywords, page_width/height, landscape

HeadingLevel (enum)       -- H1..H6
HorizontalAlignment (enum) -- Left, Center, Right, Both
ErrorAction (enum)        -- Continue, Skip, Stop
```

### 6.2 Trait Hierarchy 特质层级

```
DocxRow
  ├── schema() -> &'static [TableColumn]         -- Column metadata (generated by derive)
  ├── from_row(&RowData) -> Result<Self>          -- Deserialise row -> struct
  ├── from_row_with_converters(&RowData, &Registry) -> Result<Self>
  ├── to_row(&self) -> Result<Vec<CellData>>      -- Serialise struct -> row
  └── to_row_with_converters(&self, &Registry) -> Result<Vec<CellData>>

DocConverter<T>
  ├── support_type() -> TypeId
  ├── to_doc_value(&T, &TableColumn) -> Result<DocValue>
  └── from_doc_value(&DocValue, &TableColumn) -> Result<T>

DocReadListener<T>
  ├── invoke(T, &DocReadContext) -> Result<()>    -- Called for each data item
  ├── invoke_table(&TableData, &DocReadContext)    -- Called for each table
  ├── on_complete(&DocReadContext)                  -- After all content
  ├── on_error(&DocError, &DocReadContext) -> ErrorAction
  └── has_next(&DocReadContext) -> bool             -- Early termination

DocWriteHandler
  ├── order() -> i32                               -- Execution order
  ├── before_document / after_document              -- Document lifecycle
  ├── before_paragraph / after_paragraph            -- Paragraph lifecycle
  ├── before_table / after_table                    -- Table lifecycle
  └── before_cell / after_cell                      -- Cell lifecycle
```

---

## 7. Builder Pattern Design 构建器模式设计

### 7.1 Pattern Rules

| Rule | Code Pattern | Why |
|:---|:---|:---|
| Owned self | `pub fn method(mut self, ...) -> Self` | Enables chaining, prevents accidental reuse |
| Must-use | `#[must_use]` on all builder structs | Compiler warns if chain result is discarded |
| Build consumes | `pub fn build(self) -> Result<Executor>` | Builder -> product, builder is consumed |
| Terminal methods | `do_write()`, `save()`, `do_read()` | Execute and consume in one call |

### 7.2 Builder State Machine

```
EasyDoc::write_table(path, data)
        │
        v
  TableWriteBuilder<T>
        ├── title("...")      -> TableWriteBuilder (stay)
        ├── need_header(bool)  -> TableWriteBuilder (stay)
        ├── header_style(...)  -> TableWriteBuilder (stay)
        ├── banded_rows(bool)  -> TableWriteBuilder (stay)
        └── do_write()         -> Result<()> (terminal, consumes builder)


EasyDoc::document(path)
        │
        v
  DocBuilder
        ├── title("...")       -> DocBuilder (stay)
        ├── author("...")      -> DocBuilder (stay)
        ├── add_heading(...)   -> DocBuilder (stay)
        ├── add_paragraph(...) -> DocBuilder (stay)
        ├── add_table(...)     -> DocBuilder (stay)
        ├── add_image(...)     -> DocBuilder (stay)
        ├── add_page_break()   -> DocBuilder (stay)
        ├── build()            -> Result<DocWriteExecutor>
        └── save()             -> Result<()> (terminal, build + save)


EasyDoc::read(path)
        │
        v
  DocReadBuilder
        └── do_read::<T>()     -> Result<Vec<T>> (terminal)
```

---

## 8. Multi-Engine Strategy 多引擎策略

### 8.1 Engine Selection Map 引擎选择图

```
User calls EasyDoc::document("out.docx")
        │
        v
  Always docx-rs backend (only DOCX is writable)
        └── docx_rs::Docx -> XMLDocx -> pack(file)

User calls EasyDoc::read_text("in.docx" / "in.doc")
        │
        v
  office_oxide::Document::open(path)  <-- unified DOCX + DOC parser
        └── doc.plain_text() -> String
```

### 8.2 Engine Comparison 引擎对比

| Dimension 维度 | docx-rs 0.4.20 | office_oxide 0.1.7 |
|:---|:---|:---|
| **Paradigm** | Builder-based DOCX construction | Unified document IR (read + edit + convert) |
| **Write DOCX** | Full OOXML generation | Via EditableDocument |
| **Read DOCX** | Write-focused | Text, tables, IR extraction |
| **Read DOC** | No legacy support | Pure Rust CFB/OLE2 parser |
| **Write DOC** | No | No (Rust ecosystem gap) |
| **WASM** | Yes | No |
| **Maturity** | ~219K downloads, 408 stars | Newer, rapidly evolving |
| **License** | MIT | MIT / Apache-2.0 |

### 8.3 Why Two Engines

Following easyexcel-rust's precedent (calamine for reading + rust_xlsxwriter for writing):

- **docx-rs** is the most mature DOCX writer in the Rust ecosystem (~219K downloads, WASM support, clean builder API). It is the clear choice for writing.
- **office_oxide** is the only pure-Rust crate capable of reading both DOCX and DOC. It provides a unified IR with text, table, and metadata extraction.

This dual-engine approach gives us the best of both worlds: battle-tested writing + universal reading.

---

## 9. Error Handling 错误处理

### 9.1 Error Taxonomy 错误分类

```
DocError
├── Io         <-- wraps std::io::Error (file not found, permission denied, etc.)
├── Zip        <-- ZIP packaging errors from docx-rs
├── Format     <-- invalid or corrupted document format
├── Template   <-- placeholder resolution failures ({placeholder}, message)
├── Conversion <-- type conversion errors (field, value, message)
├── Unsupported <-- operation not available for this format/configuration
└── Document   <-- catch-all for engine-specific errors
```

### 9.2 Error Flow

```
Engine error (docx-rs / office_oxide / io::Error / zip::ZipError)
        │
        v
  Mapped to DocError variant in the engine crate
        │
        v
  Propagated via ? through builder chain
        │
        v
  User receives easydoc::Result<T>
```

### 9.3 Design Decisions

| Decision | Rationale |
|:---|:---|
| Single `DocError` enum | Users only need one error type in their code |
| `thiserror` derive | Automatic `Display` + `Error` + `From` impls |
| `type Result<T> = ...` | Less typing, consistent across the codebase |
| Engine errors wrapped, not exposed | Engine can be swapped without changing error type |
| No `anyhow` in library code | Library should expose structured errors; `anyhow` is for applications |
| Manual `From<ZipError>` | Different zip versions between docx-rs (0.6) and workspace (7.2) require explicit mapping |

---

## 10. Template Engine 模板引擎

### 10.1 Placeholder Types

| Placeholder | Pattern | Example | Usage |
|:---|:---|:---|:---|
| **Scalar** | `{key}` | `{name}`, `{date}` | Single value replacement |
| **Collection** | `{.field}` | `{.items}` | Expands to N copies in table rows |
| **Named Collection** | `{prefix.field}` | `{user.name}` | Named list with field access |

### 10.2 ZIP-Preserving Strategy

Unlike naive byte-level replacement, the template engine operates at the ZIP entry level:

1. Open template as `ZipArchive`
2. For each entry:
   - Non-XML entries (images, styles): copy unchanged to preserve fidelity
   - `word/document.xml`: parse, find placeholders, replace, write back
3. Close `ZipWriter` producing a valid DOCX with all original styles, images, and structure intact

This mirrors easyexcel-template's approach for XLSX templates.

### 10.3 Collection Expansion

For `{.field}` placeholders in table rows:

1. Find `<w:tr>` (or `<w:p>` for paragraph-level) containing `{.`
2. Extract the row as a template
3. For each data item, clone the template row and replace `{.field}` with actual values
4. Replace the single template row with N expanded rows

---

## 11. Derive Macro 派生宏

### 11.1 `#[derive(DocxRow)]` Architecture

```
User writes:
  #[derive(DocxRow)]
  #[docx(banded_rows = true)]
  struct User {
      #[docx(name = "Name", width = 0.3, order = 0)]
      name: String,
      #[docx(name = "Age", width = 0.15, order = 1)]
      age: u32,
      #[docx(ignore)]
      internal_id: String,
  }

        │  proc_macro expansion
        v

Generated code:
  impl DocxRow for User {
      fn schema() -> &'static [TableColumn] {
          static SCHEMA: LazyLock<Vec<TableColumn>> = LazyLock::new(|| vec![
              TableColumn { name: "Name", field_name: "name", index: 0, order: 0,
                            width: Some(0.3), format: None, ignored: false },
              TableColumn { name: "Age",  field_name: "age",  index: 1, order: 1,
                            width: Some(0.15), format: None, ignored: false },
          ]);
          &*SCHEMA
      }

      fn from_row(row: &RowData) -> Result<Self> { /* cell-by-cell deserialisation */ }
      fn to_row(&self) -> Result<Vec<CellData>> { /* field-by-field serialisation */ }
      fn from_row_with_converters(...) -> Result<Self> { /* delegate to from_row */ }
      fn to_row_with_converters(...) -> Result<Vec<CellData>> { /* delegate to to_row */ }
  }
```

### 11.2 Attribute Parsing Pipeline

```
proc_macro::TokenStream
        │
        v
  syn::parse2 -> DeriveInput
        │
        ├── parse_struct_attrs(&input.attrs)
        │     └── #[docx(banded_rows = true, auto_width = true)]
        │         -> StructConfig { banded_rows, auto_width }
        │
        └── collect_fields(&input.data)
              │
              For each named field:
              ├── #[docx(name = "X")]    -> overrides header name
              ├── #[docx(index = N)]     -> sets column index
              ├── #[docx(order = N)]     -> sets sort order
              ├── #[docx(width = 0.3)]   -> sets column width fraction
              ├── #[docx(format = "...")] -> sets date/number format
              ├── #[docx(ignore)]         -> skips field
              └── no annotation           -> uses field name as header
```

### 11.3 Supported Attributes 支持的属性

**Struct-level 结构体级别:**
| Attribute | Type | Description |
|:---|:---|:---|
| `banded_rows` | bool | Enable zebra striping |
| `auto_width` / `table_width` | bool | Enable auto column width |

**Field-level 字段级别:**
| Attribute | Type | Description |
|:---|:---|:---|
| `name` | string | Column header text |
| `index` | usize | Zero-based column index |
| `order` | u32 | Sort order (lower = leftmost) |
| `width` | f64 | Column width as fraction of page (0.0-1.0) |
| `format` | string | Date/time format pattern |
| `ignore` | -- | Skip field during read/write |

---

## 12. Conventions from easyexcel-rust 继承约定

| Convention 约定 | easyexcel-rust | easydoc-rust | Notes |
|:---|:---|:---|:---|
| **Workspace** | Virtual manifest + shared `[workspace.dependencies]` | Same | `resolver = "3"`, edition 2024 |
| **Crate naming** | `easyexcel`, `easyexcel-core`, `easyexcel-derive`, ... | `easydoc`, `easydoc-core`, `easydoc-derive`, ... | Same pattern |
| **MSRV** | 1.88 | 1.88 | Explicit `rust-version` in `[workspace.package]` |
| **Edition** | 2024 | 2024 | |
| **License** | Apache-2.0 | Apache-2.0 | |
| **unsafe** | `#![forbid(unsafe_code)]` | Same | Workspace-level lint |
| **Lints** | `clippy::pedantic`, `clippy::all`, `missing_docs` | Same | |
| **Error type** | `thiserror` derive, single enum | `DocError` | Seven variants |
| **Result alias** | `pub type Result<T> = ...` | Same | |
| **Builder** | `mut self -> Self`, `#[must_use]` | Same | Owned builder pattern |
| **Facade** | Thin crate with path deps on sub-crates | `crates/easydoc` | Only path deps on sub-crates |
| **Derive macro** | `syn`/`quote`/`proc-macro2` | Same | `#[derive(DocxRow)]` |
| **Static factory** | `EasyExcel` struct | `EasyDoc` struct | Same pattern, domain-adapted |
| **Trait extension points** | `ExcelRow`, `Converter<T>`, `ReadListener<T>`, `WriteHandler` | `DocxRow`, `DocConverter<T>`, `DocReadListener<T>`, `DocWriteHandler` | Same count, same roles |
| **Converter registry** | `ConverterRegistry` with `TypeId` dispatch | Same | Built-in + custom converters |
| **Template fill** | ZIP-preserving XLSX modification | ZIP-preserving DOCX modification | Same strategy |
| **Read/Write split** | calamine (read) + rust_xlsxwriter (write) | office_oxide (read) + docx-rs (write) | Same dual-engine pattern |
| **Tests** | Integration tests with tempdir + ZIP validation | Same | 7 integration + 4 unit tests |

---

## 13. DOC vs Excel Paradigm Differences DOC与Excel范式差异

Understanding these differences is critical for API design:

| Dimension | Excel (easyexcel-rust) | DOC (easydoc-rust) |
|:---|:---|:---|
| **Layout model** | Grid-based (rows x columns) | Flow-based (paragraphs, headings, sections) |
| **Data unit** | Cell (A1, B2, ...) at row/col intersection | Paragraph / Table cell in document flow |
| **Streaming** | Row-by-row SAX parsing | Paragraph-by-paragraph or page-by-page |
| **Header** | Row 1 with column names | Table header rows, heading paragraphs |
| **Style** | Per-cell or per-column formatting | Per-run (character) or per-paragraph formatting |
| **Template** | `{key}` in cells, row expansion with `{.field}` | `{key}` in paragraphs and table cells |
| **Multiple entities** | Workbook -> Sheet1, Sheet2, ... | Single document -> Sections, Pages |
| **Format detection** | Extension + magic bytes (XLSX PK, XLS OLE2) | Extension + magic bytes (DOCX PK, DOC OLE2) |
| **Memory model** | SXSSF (streaming write to disk) | In-memory document assembly, stream to disk at finish |

### Design implications:

1. **DocxRow maps to table rows, not document structure** -- Unlike `ExcelRow` which maps to the primary data unit, `DocxRow` maps specifically to table rows. Paragraphs use `Paragraph`/`Run` builders instead.
2. **No "sheet" abstraction** -- Documents are flat sequences of block elements (paragraphs, tables). The `DocumentElement` enum captures this.
3. **Tables are embedded in document flow** -- Not the primary container. A document is primarily paragraphs; tables are an element type within that flow.
4. **Styles cascade differently** -- DOC runs inherit from paragraph styles; Excel cells have independent formatting. The `FontConfig` on `Run` captures character-level overrides.

---

## Appendix A: Quality Gates

| Gate | Command | Status |
|:---|:---|:---:|
| Format | `cargo fmt --all -- --check` | ✅ |
| Lint | `cargo clippy --workspace -- -D warnings` | ✅ |
| Build | `cargo check --workspace` | ✅ |
| Test | `cargo test --workspace` | ✅ (11/11) |
| Docs | `cargo doc --workspace --no-deps` | 🚧 |

## Appendix B: File Inventory

| File | Lines | Purpose |
|:---|:---:|:---|
| `Cargo.toml` | 39 | Workspace manifest |
| `crates/easydoc-core/src/error.rs` | 62 | DocError + Result |
| `crates/easydoc-core/src/types.rs` | 193 | DocValue, CellData, RowData, TableData, etc. |
| `crates/easydoc-core/src/traits.rs` | 187 | DocxRow, DocConverter, DocReadListener, DocWriteHandler |
| `crates/easydoc-core/src/converter/registry.rs` | 398 | ConverterRegistry + FallbackConvert impls |
| `crates/easydoc-core/src/style/` | 236 | Color, FontConfig, ParagraphStyle, TableStyle |
| `crates/easydoc-core/src/metadata/` | 110 | TableColumn, DocumentMeta |
| `crates/easydoc-derive/src/implementation.rs` | 258 | Derive macro implementation |
| `crates/easydoc-writer/src/lib.rs` | 249 | Paragraph, Run, Table, DocImage builders |
| `crates/easydoc-writer/src/builder/` | 160 | DocBuilder, TableWriteBuilder |
| `crates/easydoc-writer/src/executor/` | 240 | DocWriteExecutor, TableWriteExecutor |
| `crates/easydoc-reader/src/extractor/` | 132 | text extraction, table extraction, format detection |
| `crates/easydoc-template/src/` | 450 | Placeholder detection, ZIP-preserving fill |
| `crates/easydoc/src/easy_doc.rs` | 90 | EasyDoc static factory |
| `crates/easydoc/src/lib.rs` | 52 | Facade re-exports + prelude |
| **Total** | **~2,800** | |

---

> *"Design is not just what it looks like and feels like. Design is how it works."* -- Steve Jobs
