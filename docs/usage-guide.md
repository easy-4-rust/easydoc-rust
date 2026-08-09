# easydoc-rust Usage Guide &middot; 使用指南

> **Version**: 0.1.0 | **Date**: 2026-07-21 | **Language**: English / 中文

---

## Table of Contents 目录

1. [Installation 安装](#1-installation-安装)
2. [Quick Start 快速开始](#2-quick-start-快速开始)
3. [Write Documents 写入文档](#3-write-documents-写入文档)
   - [3.1 Quick Table Write 快捷表格写入](#31-quick-table-write-快捷表格写入)
   - [3.2 Full Document Builder 完整文档构建器](#32-full-document-builder-完整文档构建器)
   - [3.3 Paragraphs and Runs 段落与文本运行](#33-paragraphs-and-runs-段落与文本运行)
   - [3.4 Tables from Data 从数据生成表格](#34-tables-from-data-从数据生成表格)
   - [3.5 Images 图片插入](#35-images-图片插入)
   - [3.6 Page Breaks 分页](#36-page-breaks-分页)
4. [Read Documents 读取文档](#4-read-documents-读取文档)
   - [4.1 Text Extraction 文本提取](#41-text-extraction-文本提取)
   - [4.2 Table Extraction 表格提取](#42-table-extraction-表格提取)
   - [4.3 Format Detection 格式检测](#43-format-detection-格式检测)
5. [Template Fill 模板填充](#5-template-fill-模板填充)
   - [5.1 Scalar Replacement 标量替换](#51-scalar-replacement-标量替换)
   - [5.2 Collection Expansion 集合展开](#52-collection-expansion-集合展开)
   - [5.3 Fill Configuration 填充配置](#53-fill-configuration-填充配置)
6. [Convert to Markdown 转换 Markdown](#6-convert-to-markdown-转换-markdown)
   - [6.1 Quick Conversion 快速转换](#61-quick-conversion-快速转换)
   - [6.2 Full Conversion with Options 完整转换](#62-full-conversion-with-options-完整转换)
   - [6.3 MarkdownBuilder API](#63-markdownbuilder-api)
   - [6.4 Supported Markdown Elements](#64-supported-markdown-elements)
   - [6.5 MarkdownResult](#65-markdownresult)
7. [Semantic Document Reading 语义文档读取](#7-semantic-document-reading-语义文档读取)
   - [7.1 Read as DocumentContent](#71-read-as-documentcontent)
   - [7.2 DocumentContent Model](#72-documentcontent-model)
8. [Style System 样式系统](#8-style-system-样式系统)
   - [8.1 FontConfig 字体配置](#81-fontconfig-字体配置)
   - [8.2 ParagraphStyle 段落样式](#82-paragraphstyle-段落样式)
   - [8.3 TableStyle 表格样式](#83-tablestyle-表格样式)
   - [8.4 Color 颜色](#84-color-颜色)
9. [Advanced Features 高级特性](#9-advanced-features-高级特性)
   - [9.1 #[derive(DocxRow)] 派生宏](#91-derivedocxrow-派生宏)
   - [9.2 Custom Converters 自定义转换器](#92-custom-converters-自定义转换器)
   - [9.3 Write Lifecycle Hooks 写入生命周期钩子](#93-write-lifecycle-hooks-写入生命周期钩子)
   - [9.4 Read Listeners 读取监听器](#94-read-listeners-读取监听器)
10. [Error Handling 错误处理](#10-error-handling-错误处理)
11. [Real-World Examples 实战案例](#11-real-world-examples-实战案例)
12. [API Reference 接口速查](#12-api-reference-接口速查)
## 1. Installation 安装

Add to your `Cargo.toml`:

```toml
[dependencies]
easydoc = "0.1"
serde = { version = "1", features = ["derive"] }  # needed for template fill
```

MSRV: Rust 1.88+

---

## 2. Quick Start 快速开始

```rust
use easydoc::prelude::*;
use std::collections::HashMap;

fn main() -> easydoc::Result<()> {
    // 1. Write a table in one line
    EasyDoc::write_table("users.docx", &users)
        .title("User List")
        .do_write()?;

    // 2. Read text from a document
    let text = EasyDoc::read_text("report.docx")?;

    // 3. Fill a template
    let mut data = HashMap::new();
    data.insert("name".into(), "Alice".into());
    EasyDoc::fill_template("tpl.docx", "out.docx", &data)?;

    Ok(())
}
```

---

## 3. Write Documents 写入文档

### 3.1 Quick Table Write 快捷表格写入

The simplest API: convert a `Vec<Struct>` directly into a DOCX table.

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
    .title("User Report")           // optional heading above the table
    .need_header(true)              // include column headers (default)
    .header_style(TableStyle::header())  // bold white-on-blue headers
    .banded_rows(true)              // alternating row colors
    .do_write()?;                   // execute and save
```

**Build method chain:**

| Method | Purpose |
|:---|:---|
| `.title("...")` | Add a heading above the table |
| `.need_header(bool)` | Show/hide the header row (default: true) |
| `.header_style(TableStyle)` | Style for the header row |
| `.banded_rows(bool)` | Enable zebra striping |

### 3.2 Full Document Builder 完整文档构建器

For documents with mixed content (headings, paragraphs, tables, images):

```rust
EasyDoc::document("report.docx")
    // Document metadata
    .title("Annual Report 2026")
    .author("Zhang San")

    // Headings
    .add_heading("Chapter 1: Overview", HeadingLevel::H1)
    .add_heading("1.1 Background", HeadingLevel::H2)

    // Paragraphs with styled runs
    .add_paragraph(
        Paragraph::new()
            .add_text("This quarter we achieved ")
            .add_run(Run::new("record growth").bold().color(0x00AA00))
            .add_text(" across all metrics.")
            .alignment(HorizontalAlignment::Both)
    )

    // Table from data
    .add_table(
        Table::from_data(&users)
            .header_style(TableStyle::header())
            .banded_rows(true)
    )

    // Page break
    .add_page_break()

    // Content on page 2
    .add_paragraph(Paragraph::new().add_text("Page 2 content."))

    // Save
    .save()?;   // shortcut: build() + write to file
```

**Available document elements:**

| Method | Element | Description |
|:---|:---|:---|
| `.add_heading(text, level)` | Heading | H1-H6, rendered bold with size 28 |
| `.add_paragraph(para)` | Paragraph | Block of text with optional styling |
| `.add_table(table)` | Table | From `Table::from_data(&[T])` |
| `.add_image(image)` | Image | PNG/JPEG via `DocImage::new(path)` |
| `.add_page_break()` | PageBreak | Hard page break |

### 3.3 Paragraphs and Runs 段落与文本运行

A `Paragraph` contains `Run`s — each run has independent formatting:

```rust
Paragraph::new()
    // Plain text (default style)
    .add_text("Normal text. ")

    // Bold run
    .add_run(Run::new("Bold text").bold())

    // Colored run
    .add_run(Run::new("Red text").color(0xFF0000))

    // Sized and fonted run
    .add_run(
        Run::new("Large Arial")
            .size(28)           // half-points: 28 = 14pt
            .font("Arial")
            .bold()
            .italic()
    )

    // Underlined
    .add_run(Run::new("Important").underline())

    // Alignment
    .alignment(HorizontalAlignment::Center)
```

**Run formatting methods:**

| Method | Type | Example |
|:---|:---|:---|
| `.bold()` | toggle | `.bold()` |
| `.italic()` | toggle | `.italic()` |
| `.underline()` | toggle | `.underline()` |
| `.size(u32)` | half-points | `.size(24)` = 12pt |
| `.color(u32)` | hex RGB | `.color(0xFF0000)` = red |
| `.font("...")` | string | `.font("Times New Roman")` |

### 3.4 Tables from Data 从数据生成表格

Tables are created from any type implementing `DocxRow`:

```rust
// Option A: Use #[derive(DocxRow)]
#[derive(DocxRow)]
struct Product {
    #[docx(name = "SKU", order = 0)]
    sku: String,
    #[docx(name = "Price", order = 1)]
    price: f64,
    #[docx(ignore)]  // skip this field
    internal_id: u64,
}

// Option B: Manual implementation
impl DocxRow for Product {
    fn schema() -> &'static [TableColumn] {
        static S: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| vec![
            TableColumn::new("SKU", "sku", 0).order(0),
            TableColumn::new("Price", "price", 1).order(1).width(0.5),
        ]);
        &*S
    }

    fn from_row(row: &RowData) -> Result<Self> {
        Ok(Product {
            sku: /* extract from row.cells[0] */,
            price: /* extract from row.cells[1] */,
            internal_id: 0,
        })
    }

    fn to_row(&self) -> Result<Vec<CellData>> {
        Ok(vec![
            CellData::new(self.sku.clone()),
            CellData::new(self.price.to_string()),
        ])
    }

    fn from_row_with_converters(r: &RowData, reg: &ConverterRegistry) -> Result<Self> {
        Self::from_row(r)
    }
    fn to_row_with_converters(&self, reg: &ConverterRegistry) -> Result<Vec<CellData>> {
        self.to_row()
    }
}

// Use in document or standalone
let table = Table::from_data(&products)
    .header_style(TableStyle::header())
    .banded_rows(true);
```

### 3.5 Images 图片插入

```rust
EasyDoc::document("with_image.docx")
    .add_paragraph(Paragraph::new().add_text("Before the image."))
    .add_image(DocImage::new("chart.png"))
    .add_paragraph(Paragraph::new().add_text("After the image."))
    .save()?;
```

Supported formats: PNG, JPEG (via docx-rs's `image` crate backend).

### 3.6 Page Breaks 分页

```rust
EasyDoc::document("multi_page.docx")
    .add_paragraph(Paragraph::new().add_text("Page 1 content"))
    .add_page_break()
    .add_paragraph(Paragraph::new().add_text("Page 2 content"))
    .save()?;
```

---

## 4. Read Documents 读取文档

### 4.1 Text Extraction 文本提取

Extracts all plain text from DOCX or DOC files:

```rust
// Synchronous: reads entire document at once
let text = EasyDoc::read_text("document.docx")?;
println!("{text}");

// Also works for legacy .doc files
let text = EasyDoc::read_text("legacy.doc")?;
```

### 4.2 Table Extraction 表格提取

Extract tables and deserialize each row into typed structs:

```rust
#[derive(DocxRow)]
struct User {
    #[docx(name = "Name", order = 0)]
    name: String,
    #[docx(name = "Age", order = 1)]
    age: u32,
    #[docx(name = "Email", order = 2)]
    email: String,
}

// Returns Vec<Vec<User>> — each inner Vec is one table
let tables: Vec<Vec<User>> = EasyDoc::read_tables::<User>("users.docx")?;

for (i, table) in tables.iter().enumerate() {
    println!("Table {i}: {} rows", table.len());
    for user in table {
        println!("  {} - {}", user.name, user.email);
    }
}

// Flatten all tables into a single Vec
let all_users: Vec<User> = EasyDoc::read("users.docx").do_read::<User>()?;
```

Note: Header rows (first row marked as header in the table) are automatically skipped during extraction.

### 4.3 Format Detection 格式检测

```rust
use easydoc::{detect_format, DocumentFormat};

match detect_format(path) {
    Some(DocumentFormat::Docx) => println!("Office Open XML (.docx)"),
    Some(DocumentFormat::Doc) => println!("Legacy Word Binary (.doc)"),
    None => println!("Unknown or unsupported format"),
}
```

Detection works by checking both file extension and magic bytes:
- `.docx` extension or `PK\x03\x04` (ZIP) magic → `DocumentFormat::Docx`
- `.doc` extension or `\xD0\xCF\x11\xE0...` (OLE2/CFB) magic → `DocumentFormat::Doc`

---

## 5. Template Fill 模板填充

### 5.1 Scalar Replacement 标量替换

Create a DOCX template with `{key}` placeholders, then fill with data:

```rust
use std::collections::HashMap;

// 1. Create the template (or use Word)
EasyDoc::document("template.docx")
    .add_paragraph(Paragraph::new().add_text("Dear {name},"))
    .add_paragraph(Paragraph::new().add_text("Your order {order_id} is confirmed."))
    .add_paragraph(Paragraph::new().add_text("Total: {total}"))
    .save()?;

// 2. Fill the template
let mut data = HashMap::new();
data.insert("name".into(), "Alice".into());
data.insert("order_id".into(), "ORD-12345".into());
data.insert("total".into(), "$99.99".into());

EasyDoc::fill_template("template.docx", "output.docx", &data)?;
```

**How it works:**
1. The template DOCX is opened as a ZIP archive
2. `word/document.xml` is extracted and `{key}` placeholders are identified
3. Each `{key}` is replaced with the corresponding value from the data map
4. All other ZIP entries (styles, images, headers, footers) are preserved unchanged
5. A new valid DOCX is written

### 5.2 Collection Expansion 集合展开

For templates with repeating data (list items in tables), use `{.field}` placeholders:

```rust
#[derive(Debug, serde::Serialize)]
struct InvoiceItem {
    item: String,
    qty: String,
    price: String,
}

let items = vec![
    InvoiceItem { item: "Widget".into(), qty: "10".into(), price: "$5.00".into() },
    InvoiceItem { item: "Gadget".into(), qty: "5".into(), price: "$20.00".into() },
];

EasyDoc::fill_template_list(
    "template.docx",
    "output.docx",
    &items,
    "items",    // the collection field name
)?;
```

The template should contain a table row with placeholders like:
```
| Item        | Qty | Price  |
| {.item}     | {.qty} | {.price} |
```

The engine finds the row containing `{.`, replicates it N times, and replaces each `{.field}` with the corresponding data value.

**Named collections** use `{prefix.field}` syntax for multiple independent lists:
```
| {order.item} | {order.qty} |
```

### 5.3 Fill Configuration 填充配置

```rust
use easydoc::{FillConfig, FillDirection};

let config = FillConfig::new()
    .direction(FillDirection::Vertical)  // expand rows (default) or columns
    .force_new_row(true)                 // create new rows instead of in-place
    .auto_style(false);                  // don't inherit template cell styles
```

---

## 6. Convert to Markdown 转换 Markdown

### 6.1 Quick Conversion 快速转换

```rust
// Convert DOCX/DOC to Markdown string
let markdown = EasyDoc::to_markdown("document.docx")?;
```

### 6.2 Full Conversion with Options 完整转换

```rust
use easydoc::prelude::*;

let result = EasyDoc::markdown("document.docx")
    .image_directory("output/assets")     // extract images here
    .image_reference_prefix("assets")     // Markdown image path prefix
    .include_front_matter(true)           // YAML front matter
    .write_to("output/document.md")?;    // atomic write

println!("Markdown: {} chars", result.markdown.len());
println!("Images extracted: {}", result.assets.len());

for warning in &result.warnings {
    eprintln!("Conversion fallback: {}", warning.message);
}
```

### 6.3 MarkdownBuilder API

```rust
MarkdownBuilder::new(path)              // source DOCX/DOC
    .image_directory(dir)               // image output directory
    .image_reference_prefix(prefix)     // image reference prefix in Markdown
    .include_front_matter(bool)         // YAML front matter (title/author/subject/keywords)
    .do_convert()                       // -> Result<MarkdownResult>
    .write_to(output)                   // -> Result<MarkdownResult> (atomic write)
```

### 6.4 Supported Markdown Elements

| Element | Output Format | Notes |
|---|---|---|
| Headings H1–H6 | `## **text**` | Bold text in headings |
| Bold | `**text**` | |
| Italic | `*text*` | |
| Strikethrough | `~~text~~` | |
| Hyperlinks | `[text](url)` | |
| GFM tables | `\| col \|` | Auto column width |
| Merged cells | HTML `<table>` | + warning |
| Nested lists | `1. item` / `- item` | With start number |
| Code blocks | ` ```lang ``` ` | |
| Footnotes | `[^id]: text` | |
| Endnotes | `[^endnote-id]: text` | |
| Images | `![alt](path)` | With extraction |
| Thematic break | `---` | |
| Page break | `<!-- page-break -->` | |
| Column break | `<!-- column-break -->` | |
| Front matter | `---\ntitle: '...'` | YAML |

### 6.5 MarkdownResult

```rust
pub struct MarkdownResult {
    pub markdown: String,           // Generated Markdown text
    pub assets: Vec<ExtractedAsset>, // Extracted images
    pub warnings: Vec<ConversionWarning>, // Degradation warnings
}

pub struct ExtractedAsset {
    pub path: PathBuf,      // File path on disk
    pub reference: String,  // Reference used in Markdown
}

pub struct ConversionWarning {
    pub message: String,    // Human-readable fallback description
}
```

---

## 7. Semantic Document Reading 语义文档读取

### 7.1 Read as DocumentContent

```rust
use easydoc::easydoc_reader::read_document;

let doc = read_document(std::path::Path::new("document.docx"))?;

// Access metadata
println!("Title: {:?}", doc.metadata.title);
println!("Author: {:?}", doc.metadata.author);

// Iterate blocks
for block in &doc.blocks {
    match block {
        DocumentBlock::Heading { level, runs } => {
            println!("H{}: {}", level, runs.iter().map(|r| r.text.as_str()).collect::<String>());
        }
        DocumentBlock::Paragraph(runs) => {
            println!("{}", runs.iter().map(|r| r.text.as_str()).collect::<String>());
        }
        DocumentBlock::Table(table) => {
            println!("Table: {} rows", table.rows.len());
        }
        DocumentBlock::List(list) => {
            println!("List: {} items, ordered={}", list.items.len(), list.ordered);
        }
        DocumentBlock::Image(image) => {
            println!("Image: {:?}, {} bytes", image.alt_text, image.data.as_ref().map_or(0, |d| d.len()));
        }
        _ => {}
    }
}
```

### 7.2 DocumentContent Model

```rust
pub struct DocumentContent {
    pub metadata: DocumentMeta,
    pub blocks: Vec<DocumentBlock>,
}

pub enum DocumentBlock {
    Heading { level: u8, runs: Vec<DocumentTextRun> },
    Paragraph(Vec<DocumentTextRun>),
    Table(DocumentTable),
    List(DocumentList),
    Image(DocumentImage),
    ThematicBreak,
    PageBreak,
    ColumnBreak,
    CodeBlock { language: Option<String>, code: String },
    TextBox(Vec<DocumentBlock>),
    Footnote { id: u32, blocks: Vec<DocumentBlock> },
    Endnote { id: u32, blocks: Vec<DocumentBlock> },
}

pub struct DocumentTextRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub hyperlink: Option<String>,
}
```

---

## 8. Style System 样式系统


### 8.1 FontConfig 字体配置

```rust
use easydoc::{FontConfig, Color};

// Default font (11pt, black, not bold/italic/underline)
let default_font = FontConfig::new();

// Pre-built presets
let bold_font = FontConfig::bold();            // bold, default size
let header_font = FontConfig::header();        // bold, white, for table headers

// Fluent builder
let custom_font = FontConfig::new()
    .name("Times New Roman")
    .size(24)            // 12pt = 24 half-points
    .with_bold(true)
    .with_italic(false)
    .with_underline(false)
    .color(Color::RED);
```

### 8.2 ParagraphStyle 段落样式

```rust
use easydoc::{ParagraphStyle, HorizontalAlignment};

let style = ParagraphStyle::new()
    .alignment(HorizontalAlignment::Both)    // justified
    .first_line_indent(480)                  // ~2 characters at 12pt
    .space_after(120)                        // 6pt after paragraph
    .line_spacing(360);                      // 1.5 line spacing
```

| Alignment | Description |
|:---|:---|
| `HorizontalAlignment::Left` | Left-aligned |
| `HorizontalAlignment::Center` | Centered |
| `HorizontalAlignment::Right` | Right-aligned |
| `HorizontalAlignment::Both` | Justified |

### 8.3 TableStyle 表格样式

```rust
let style = TableStyle::new()
    .banded_rows(true)                      // alternating row colors
    .auto_width(true)                       // auto-fit column widths
    .borders(true)                          // show borders
    .header_background(Color::HEADER_BLUE); // header row background

// Pre-built styles
let header_style = TableStyle::header();   // bold white text on blue
let simple_style = TableStyle::simple();   // no borders, no banding
```

### 8.4 Color 颜色

```rust
use easydoc::Color;

// Named constants
let black  = Color::BLACK;       // #000000
let white  = Color::WHITE;       // #FFFFFF
let red    = Color::RED;         // #FF0000
let blue   = Color::HEADER_BLUE; // #4472C4

// From RGB
let green = Color::rgb(0, 128, 0);

// From hex
let orange = Color::from_hex(0xFF8C00);

// To hex
let hex: u32 = orange.to_hex();  // 0xFF8C00
```

---

## 9. Advanced Features 高级特性

### 9.1 #[derive(DocxRow)] 派生宏

```rust
#[derive(DocxRow)]
#[docx(banded_rows = true)]         // struct-level attributes
struct Employee {
    #[docx(name = "Employee Name", width = 0.35, order = 0)]
    name: String,

    #[docx(name = "Department", width = 0.25, order = 1)]
    department: String,

    #[docx(name = "Salary", width = 0.20, order = 2, format = "$#,##0.00")]
    salary: f64,

    #[docx(name = "Start Date", width = 0.20, order = 3, format = "%Y-%m-%d")]
    start_date: chrono::NaiveDate,

    #[docx(ignore)]  // completely excluded from read/write
    password_hash: String,
}
```

**Struct-level attributes:**

| Attribute | Type | Example |
|:---|:---|:---|
| `banded_rows` | bool | `#[docx(banded_rows = true)]` |
| `auto_width` | bool | `#[docx(table_width = Auto)]` |

**Field-level attributes:**

| Attribute | Type | Example |
|:---|:---|:---|
| `name` | string | `#[docx(name = "Full Name")]` |
| `index` | usize | `#[docx(index = 0)]` |
| `order` | u32 | `#[docx(order = 1)]` |
| `width` | f64 | `#[docx(width = 0.3)]` (0.0–1.0 page fraction) |
| `format` | string | `#[docx(format = "%Y-%m-%d")]` |
| `ignore` | flag | `#[docx(ignore)]` |

What the derive generates:
- `schema()` — column metadata array
- `from_row(&RowData)` — row-to-struct deserialization
- `to_row(&self)` — struct-to-row serialization

### 9.2 Custom Converters 自定义转换器

Register custom type converters for specialized formatting:

```rust
use easydoc::{DocConverter, DocValue, TableColumn, ConverterRegistry, Result};

struct PhoneConverter;

impl DocConverter<String> for PhoneConverter {
    fn support_type() -> std::any::TypeId {
        std::any::TypeId::of::<String>()
    }

    fn to_doc_value(&self, value: &String, _column: &TableColumn) -> Result<DocValue> {
        // Format phone numbers for display
        let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() == 10 {
            Ok(DocValue::String(format!(
                "({}) {}-{}",
                &digits[0..3], &digits[3..6], &digits[6..10]
            )))
        } else {
            Ok(DocValue::String(value.clone()))
        }
    }

    fn from_doc_value(&self, value: &DocValue, _column: &TableColumn) -> Result<String> {
        match value {
            DocValue::String(s) => Ok(s.chars().filter(|c| c.is_ascii_digit()).collect()),
            _ => Ok(String::new()),
        }
    }
}

// Register the converter
let mut registry = ConverterRegistry::new();
registry.register::<String, PhoneConverter>(PhoneConverter);
```

### 9.3 Write Lifecycle Hooks 写入生命周期钩子

Intercept and modify the write process at any level:

```rust
use easydoc::{
    DocWriteHandler, DocWriteContext, ParagraphContext,
    TableWriteContext, CellContext, Result,
};

struct LoggingHandler;

impl DocWriteHandler for LoggingHandler {
    fn before_document(&mut self, ctx: &DocWriteContext) -> Result<()> {
        println!("Starting document: {}", ctx.path);
        Ok(())
    }

    fn before_table(&mut self, ctx: &TableWriteContext) -> Result<()> {
        println!("Writing table {} with {} rows", ctx.index, ctx.row_count);
        Ok(())
    }

    fn before_cell(&mut self, ctx: &CellContext) -> Result<()> {
        println!("  Cell [{},{}] = {:?}", ctx.row, ctx.column, ctx.value);
        Ok(())
    }

    fn after_document(&mut self, ctx: &DocWriteContext) -> Result<()> {
        println!("Document complete: {}", ctx.path);
        Ok(())
    }

    // All other methods have no-op defaults
}
```

Hook levels:
- `before_document` / `after_document` — once per document
- `before_paragraph` / `after_paragraph` — once per paragraph
- `before_table` / `after_table` — once per table
- `before_cell` / `after_cell` — once per table cell

### 9.4 Read Listeners 读取监听器

For streaming reads of large documents:

```rust
use easydoc::{DocReadListener, DocReadContext, DocReadListener, ErrorAction, CollectListener};

// Built-in: CollectListener collects all items
let mut listener = CollectListener(Vec::new());
listener.invoke("item1".to_string(), &DocReadContext {
    path: "test.docx".into(),
    index: 0,
})?;

assert_eq!(listener.0, vec!["item1"]);

// Custom listener
struct ProgressListener {
    count: usize,
}

impl DocReadListener<String> for ProgressListener {
    fn invoke(&mut self, data: String, _ctx: &DocReadContext) -> Result<()> {
        self.count += 1;
        if self.count % 100 == 0 {
            println!("Read {} items...", self.count);
        }
        Ok(())
    }

    fn on_complete(&mut self, _ctx: &DocReadContext) {
        println!("Finished reading {} items", self.count);
    }

    fn on_error(&mut self, err: &DocError, _ctx: &DocReadContext) -> ErrorAction {
        eprintln!("Skipping error: {err}");
        ErrorAction::Skip   // continue reading
    }

    fn has_next(&self, _ctx: &DocReadContext) -> bool {
        self.count < 10000  // stop after 10k items
    }
}
```

---

## 10. Error Handling 错误处理

All operations return `easydoc::Result<T>` (alias for `Result<T, DocError>`):

```rust
use easydoc::{DocError, Result};

match EasyDoc::read_text("file.docx") {
    Ok(text) => println!("{text}"),
    Err(DocError::Io(e)) => eprintln!("File I/O error: {e}"),
    Err(DocError::Format(msg)) => eprintln!("Invalid format: {msg}"),
    Err(DocError::Template { placeholder, message }) => {
        eprintln!("Template error at {placeholder}: {message}")
    }
    Err(DocError::Conversion { field, value, message }) => {
        eprintln!("Cannot convert field '{field}' with value '{value}': {message}")
    }
    Err(DocError::Unsupported(msg)) => eprintln!("Not supported: {msg}"),
    Err(DocError::Zip(msg)) => eprintln!("ZIP error: {msg}"),
    Err(DocError::Document(msg)) => eprintln!("Document error: {msg}"),
}
```

**Error variants:**

| Variant | When |
|:---|:---|
| `DocError::Io` | File not found, permission denied, disk full |
| `DocError::Zip` | Corrupt DOCX, ZIP packaging failure |
| `DocError::Format` | Invalid document structure |
| `DocError::Template` | Missing placeholder, placeholder syntax error |
| `DocError::Conversion` | Type mismatch between cell value and struct field |
| `DocError::Unsupported` | Feature not available for this format |
| `DocError::Document` | Engine-specific errors |

---

## 11. Real-World Examples 实战案例

### 11.1 Business Report Generator

```rust
#[derive(DocxRow)]
struct SalesRecord {
    #[docx(name = "Region", order = 0)]
    region: String,
    #[docx(name = "Q1", order = 1)]
    q1: f64,
    #[docx(name = "Q2", order = 2)]
    q2: f64,
    #[docx(name = "Growth", order = 3)]
    growth: String,
}

fn generate_sales_report(path: &str, records: &[SalesRecord]) -> easydoc::Result<()> {
    EasyDoc::document(path)
        .title("Quarterly Sales Report")
        .author("Finance Department")
        .add_heading("Executive Summary", HeadingLevel::H1)
        .add_paragraph(
            Paragraph::new()
                .add_text("This report summarizes sales performance ")
                .add_run(Run::new("across all regions").bold())
                .add_text(format!(" for {} records.", records.len()))
        )
        .add_heading("Sales Data", HeadingLevel::H2)
        .add_table(
            Table::from_data(records)
                .header_style(TableStyle::header())
                .banded_rows(true)
        )
        .add_page_break()
        .add_heading("Notes", HeadingLevel::H2)
        .add_paragraph(
            Paragraph::new()
                .add_text("All figures in thousands USD.")
                .alignment(HorizontalAlignment::Center)
        )
        .save()
}
```

### 11.2 Template-Based Invoice

```rust
fn generate_invoice(
    template: &str,
    output: &str,
    customer: &str,
    items: &[InvoiceLine],
    total: f64,
) -> easydoc::Result<()> {
    let mut data = HashMap::new();
    data.insert("customer".into(), customer.to_owned());
    data.insert("date".into(), chrono::Local::now().format("%Y-%m-%d").to_string());
    data.insert("total".into(), format!("${total:.2}"));

    // First, fill scalar placeholders
    EasyDoc::fill_template(template, output, &data)?;

    // Then, expand line items
    // (Collection expansion in table rows)
    EasyDoc::fill_template_list(output, output, items, "items")?;

    Ok(())
}
```

### 11.3 Document Analyzer

```rust
fn analyze_document(path: &str) -> easydoc::Result<DocumentInfo> {
    let format = detect_format(std::path::Path::new(path));
    let text = EasyDoc::read_text(path)?;
    let tables: Vec<Vec<DynamicRow>> = EasyDoc::read_tables::<DynamicRow>(path)
        .unwrap_or_default();

    Ok(DocumentInfo {
        path: path.to_owned(),
        format,
        char_count: text.len(),
        word_count: text.split_whitespace().count(),
        table_count: tables.len(),
        total_rows: tables.iter().map(|t| t.len()).sum(),
    })
}
```

---

## 12. API Reference 接口速查

## 12. API Reference 接口速查

### EasyDoc Static Factory

```rust
// Write
EasyDoc::document(path) -> DocBuilder
EasyDoc::write_table(path, &[T]) -> TableWriteBuilder<T>

// Read
EasyDoc::read(path) -> DocReadBuilder
EasyDoc::read_text(path) -> Result<String>
EasyDoc::read_tables::<T>(path) -> Result<Vec<Vec<T>>>

// Template
EasyDoc::fill_template(tpl, out, &HashMap<K,V>) -> Result<()>
EasyDoc::fill_template_list(tpl, out, &[T], field) -> Result<()>

// Edit
EasyDoc::edit(path) -> Result<DocEditor>

// Markdown
EasyDoc::markdown(path) -> MarkdownBuilder
EasyDoc::to_markdown(path) -> Result<String>
EasyDoc::write_markdown(source, output) -> Result<MarkdownResult>
```

### DocBuilder

```rust
.title("T") -> Self
.author("A") -> Self
.add_heading("text", HeadingLevel::H1) -> Self
.add_paragraph(Paragraph) -> Self
.add_table(Table) -> Self
.add_image(DocImage) -> Self
.add_page_break() -> Self
.build() -> Result<DocWriteExecutor>
.save() -> Result<()>
.save_to_bytes() -> Result<Vec<u8>>
.save_to_writer(W: Write+Seek) -> Result<()>
```

### TableWriteBuilder<T>

```rust
.title("T") -> Self
.need_header(bool) -> Self
.header_style(TableStyle) -> Self
.banded_rows(bool) -> Self
.do_write() -> Result<()>
.do_write_to_bytes() -> Result<Vec<u8>>
.do_write_to_writer(W: Write+Seek) -> Result<()>
```

### MarkdownBuilder

```rust
.image_directory(dir) -> Self
.image_reference_prefix(prefix) -> Self
.include_front_matter(bool) -> Self
.options(MarkdownOptions) -> Self
.do_convert() -> Result<MarkdownResult>
.write_to(output) -> Result<MarkdownResult>
```

### DocEditor

```rust
.replace_text(old, new) -> Self
.save() -> Result<()>
.save_as(path) -> Result<()>
```

### Standalone Functions

```rust
detect_format(path) -> Option<DocumentFormat>
read_document(path) -> Result<DocumentContent>
render_document(&DocumentContent, MarkdownOptions) -> Result<MarkdownResult>
```

---

> *"The best API is the one you don't need to look up."* — easydoc-rust philosophy
