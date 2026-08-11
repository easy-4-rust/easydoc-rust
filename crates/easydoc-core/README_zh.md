<a id="readme-top"></a>

<div align="center">

# easydoc-core

**easydoc-rust DOC/DOCX 文档操作 workspace 的核心类型、trait 和错误模型。**

[![Crates.io](https://img.shields.io/crates/v/easydoc-core)](https://crates.io/crates/easydoc-core)
[![docs.rs](https://img.shields.io/docsrs/easydoc-core)](https://docs.rs/easydoc-core)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#3-rust-基线与平台支持)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[定位](#1-项目定位与状态) · [Trait](#2-扩展-trait) ·
[数据模型](#3-数据模型) · [错误](#4-错误模型) · [Features](#5-cargo-features) ·
[上游兼容](#6-上游兼容) · [许可证](#8-许可证)

</div>

---

> **当前版本**：`0.1.0-alpha.1`
> **MSRV**：Rust `1.88`
> **Edition**：`2024`
> **成熟度**：Alpha
> **最后核验**：2026-08-11

## 1. 项目定位与状态

### 1.1 是什么

**`easydoc-core` 是 `easydoc-rust` workspace 的基础 crate。** 它定义了 6 个扩展 trait、语义文档数据模型、统一错误类型以及所有其他 crate 依赖的样式/元数据原语。

| 维度 | 内容 |
|---|---|
| crate | `easydoc-core` |
| 当前版本 | `0.1.0-alpha.1` |
| MSRV / Edition | `1.88` / `2024` |
| 默认 features | `[]`（空） |
| 可选 features | `serde` |
| `unsafe` 策略 | `deny`（crate 级别） |
| 许可证 | `Apache-2.0` |

### 1.2 不是什么

- 不是 DOCX 解析器或生成器 -- 那些在 `easydoc-reader` 和 `easydoc-writer` 中。
- 不是用户入口 -- 请使用 `easydoc`（门面 crate）。
- 不耦合任何特定后端（`docx-rs`、`office_oxide` 等）。

### 1.3 状态证据

| 声明 | 当前值 | 证据 |
|---|---|---|
| crate 可构建 | 是 | `cargo check -p easydoc-core` |
| 测试 | 各模块单元测试 | `cargo test -p easydoc-core` |
| MSRV | `1.88` | `Cargo.toml` 中 `rust-version` |
| `unsafe_code` | `deny` | crate 级别 lint |

## 2. 扩展 Trait

6 个扩展 trait 构成 `easydoc-rust` 可扩展性的骨干，对应 Java EasyExcel 4.0.3 的扩展点。

| Trait | 用途 | Java EasyExcel 对应 | 定义位置 |
|---|---|---|---|
| `DocxRow` | 结构体 <-> 表格行双向映射 | `@ExcelProperty` + 反射 | `traits.rs` |
| `DocConverter<T>` | Rust 类型 <-> `DocValue` 转换 | `Converter<T>` 接口 | `traits.rs` |
| `DocReadListener<T>` | 流式读取回调 | `ReadListener<T>` | `traits.rs` |
| `DocWriteHandler` | 写入生命周期钩子（文档/段落/表格/单元格） | `WriteHandler` | `traits.rs` |
| `DocumentReader` | 统一读取入口 trait（后端抽象） | --（easydoc-rust 自创） | `traits.rs` |
| `EventSink` | SAX 事件消费接口 | `ReadListener<T>` 回调 | `traits.rs` |

### 2.1 DocxRow

将 Rust 结构体映射到/从 DOCX 表格行。通常通过 `easydoc-derive` 的 `#[derive(DocxRow)]` 实现。

```rust,ignore
#[derive(DocxRow)]
struct User {
    #[docx(name = "姓名", order = 0)]
    name: String,
    #[docx(name = "年龄", order = 1)]
    age: u32,
}
```

方法：`schema()`、`from_row()`、`from_row_with_converters()`、`to_row()`、`to_row_with_converters()`。

### 2.2 DocConverter\<T\>

Rust 类型 `T` 与 `DocValue` 之间的双向转换。通过 `ConverterRegistry` 或 builder 的 `register_converter` 注册。

```rust,ignore
impl DocConverter<String> for MyConverter {
    fn support_type() -> TypeId { TypeId::of::<String>() }
    fn to_doc_value(&self, value: &String, col: &TableColumn) -> Result<DocValue> { ... }
    fn from_doc_value(&self, value: &DocValue, col: &TableColumn) -> Result<String> { ... }
}
```

### 2.3 DocReadListener\<T\>

在流式读取过程中接收已解析内容。方法：`invoke()`、`invoke_table()`、`on_complete()`、`on_error()`、`has_next()`。

### 2.4 DocWriteHandler

文档、段落、表格和单元格级别的写入生命周期拦截器。所有方法均有空默认实现。方法：`order()`、`before_document()`、`after_document()`、`before_paragraph()`、`after_paragraph()`、`before_table()`、`after_table()`、`before_cell()`、`after_cell()`。

### 2.5 DocumentReader

后端无关的读取接口。实现者提供 `read_model()` 和 `read_events()`。无直接 Java 对应 -- 这是 `easydoc-rust` 自创的抽象。

### 2.6 EventSink

在 SAX 流式过程中消费 `DocumentEvent` 实例。内置的 `ContentCollector` 实现将事件收集为 `DocumentContent`。

事件类型：`DocumentStart`、`Heading`、`Paragraph`、`Table`、`List`、`Image`、`PageBreak`、`ColumnBreak`、`CodeBlock`、`Section`、`DocumentEnd`。

## 3. 数据模型

语义文档模型与后端无关 -- 它没有直接的 Java EasyExcel 对应（Java EasyExcel 不处理 DOCX）。

### 3.1 模型层级

```text
DocumentContent
├── metadata: DocumentMeta（title、author、...）
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

### 3.2 关键类型

| 类型 | 用途 |
|---|---|
| `DocumentContent` | 顶层文档：metadata + blocks |
| `DocumentBlock` | 所有块类型的枚举（段落、表格、列表、图片等） |
| `DocumentTextRun` | 富文本片段（文本 + 加粗/斜体/字号/颜色/字体/删除线/超链接） |
| `DocumentTable` | 表格，包含行 |
| `DocumentTableRow` | 表格行，包含单元格 + `is_header` 标志 |
| `DocumentTableCell` | 单元格，包含嵌套块 + 合并跨度（`grid_span`、`v_merge`） |
| `DocumentList` | 有序/无序列表，包含列表项 |
| `DocumentImage` | 图片，包含替代文本、扩展名和二进制数据 |
| `DocumentMeta` | 文档元数据（标题、作者、描述） |

### 3.3 数据类型（DocValue）

`DocValue` 是连接 Rust 类型和 DOCX 内容的通用值枚举。

| 变体 | Rust 类型 | 说明 |
|---|---|---|
| `String(String)` | `String` / `&str` | 纯文本 |
| `Bool(bool)` | `bool` | 布尔值 |
| `Int(i64)` | `i32` / `u32` / `i64` | 整数 |
| `Float(f64)` | `f64` | 浮点数 |
| `DateTime(DateTime<Utc>)` | `chrono::DateTime<Utc>` | UTC 日期时间 |
| `Date(NaiveDate)` | `chrono::NaiveDate` | 仅日期 |
| `NaiveDateTime(NaiveDateTime)` | `chrono::NaiveDateTime` | 无时区日期时间 |
| `Empty` | `Option::None` | 空值 |
| `RichText(Vec<RichRun>)` | -- | 格式化文本片段 |
| `Image(ImageData)` | -- | 图片字节 + 元数据 |

为 `String`、`&str`、`bool`、`i32`、`u32`、`i64`、`f64`、`DateTime<Utc>`、`NaiveDate`、`NaiveDateTime` 和 `Option<T>` 提供了 `From` 实现。

### 3.4 辅助类型

| 类型 | 用途 |
|---|---|
| `CellData` | 单个表格单元格：值 + 对齐 + 合并跨度 |
| `RowData` | 单元格行 + 行高提示 |
| `TableData` | 提取的表格：可选表头 + 字符串行 |
| `HeadingLevel` | H1..H6 枚举 |
| `HorizontalAlignment` | Left / Center / Right / Both |
| `ErrorAction` | Continue / Skip / Stop（用于读取监听器） |
| `TableColumn` | 列元数据：名称、索引、格式、宽度 |

## 4. 错误模型

所有操作返回 `easydoc_core::Result<T>`（`Result<T, DocError>` 的别名）。

| 变体 | 场景 | Java 对应 | 来源 |
|---|---|---|---|
| `DocError::Io` | 文件或网络 I/O | `IOException` | `std::io::Error` |
| `DocError::Zip` | ZIP 归档错误 | `ExcelAnalysisException`（ZIP） | `zip::ZipError` |
| `DocError::Format` | 无效/不支持的格式 | `ExcelAnalysisException` | -- |
| `DocError::Template` | 占位符解析/处理错误 | `ExcelAnalysisException`（模板） | -- |
| `DocError::Conversion` | 单元格/字段值转换失败 | `ExcelDataConvertException` | -- |
| `DocError::Unsupported` | 不支持的操作 | `UnsupportedOperationException` | -- |
| `DocError::Document` | 通用文档错误 | `ExcelAnalysisException` / `ExcelGenerateException` | -- |

Java EasyExcel 将错误分散在 7 个 `RuntimeException` 子类中；`easydoc-core` 将它们统一为一个惯用的 Rust 枚举。

## 5. Cargo Features

| Feature | 默认 | 效果 | 依赖 |
|---|:---:|---|---|
| `serde` | 否 | 为数据模型类型启用 `serde::Serialize`/`Deserialize` | `serde`、`serde_json` |

```toml
# 最小（无 serde）
[dependencies]
easydoc-core = "0.1.0-alpha.1"

# 带 serde 支持
easydoc-core = { version = "0.1.0-alpha.1", features = ["serde"] }
```

## 6. 上游兼容

`easydoc-core` 将其 trait 体系映射到 Java EasyExcel 4.0.3 扩展点。

### 6.1 Trait 映射

| Java EasyExcel 4.0.3 | Rust easydoc-core | 映射类型 |
|---|---|---|
| `@ExcelProperty` 注解 + 反射 | `DocxRow` trait + derive 宏 | 惯用替代 |
| `Converter<T>` 接口 | `DocConverter<T>` trait | 行为等价 |
| `ReadListener<T>` | `DocReadListener<T>` + `EventSink` | 行为等价 |
| `WriteHandler` | `DocWriteHandler` | 行为等价 |
| `ReadCellData` / `WriteCellData` | `DocValue` 枚举 | 惯用替代 |
| `ExcelAnalysisException` 等 | `DocError` 枚举 | 统一替代 |

### 6.2 语言语义映射

| Java 机制 | Rust 设计 | 原因 |
|---|---|---|
| 受检/非受检异常 | `Result<T, DocError>` | 显式错误传播 |
| `null` | `Option<T>` | 空值安全 |
| 注解 + 反射 | trait + derive 宏 | 编译期元数据 |
| 接口继承 | trait + 组合 | 显式能力边界 |
| 全局单例 | `OnceLock<Arc<_>>` 或显式 context | 生命周期和测试隔离 |

## 7. 构建与测试

```bash
cargo check -p easydoc-core
cargo test -p easydoc-core
cargo test -p easydoc-core --features serde
cargo clippy -p easydoc-core -- -D warnings
cargo doc -p easydoc-core --no-deps
```

## 8. 许可证

Apache-2.0 -- 详见 [LICENSE](../../LICENSE)。

---

<div align="center">

[返回顶部](#readme-top) · [docs.rs](https://docs.rs/easydoc-core) · [crates.io](https://crates.io/crates/easydoc-core) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
