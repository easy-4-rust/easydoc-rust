<a id="readme-top"></a>

<div align="center">

# easydoc-derive

**easydoc-rust 工作区中用于类型化 DOCX 表格行映射的 derive 宏**

[![Crates.io](https://img.shields.io/crates/v/easydoc-derive)](https://crates.io/crates/easydoc-derive)
[![docs.rs](https://img.shields.io/docsrs/easydoc-derive)](https://docs.rs/easydoc-derive)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#rust-基线)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[项目定位](#1-项目定位) | [快速开始](#2-快速开始) | [属性参考](#3-属性参考) |
[上游兼容](#4-上游兼容) | [质量](#5-质量)

</div>

---

> **当前版本**：`0.1.0-alpha.1`
> **MSRV**：Rust `1.88`
> **Edition**：`2024`
> **成熟度**：预览
> **最后核验**：2026-08-11

---

## 1. 项目定位

**easydoc-derive 是一个 proc-macro crate，为 easydoc-rust 工作区提供 `#[derive(DocxRow)]`，用于将 Rust 结构体映射为 DOCX 表格行。**

### 1.1 是什么

| 维度 | 内容 |
|---|---|
| crate | `easydoc-derive` |
| 当前版本 | `0.1.0-alpha.1` |
| MSRV / Edition | `1.88` / `2024` |
| 类型 | proc-macro crate |
| unsafe 策略 | `forbid`（workspace lint） |
| 许可证 | `Apache-2.0` |

### 1.2 不是什么

- 不是独立的 DOCX 生成器；需要 `easydoc-core` 提供 `DocxRow` trait 和支撑类型。
- 不是通用序列化框架；专为 DOCX 表格行设计。
- 不是 Java EasyExcel 注解的 1:1 移植；将注解驱动模型适配为 Rust derive 宏。

### 1.3 状态证据

| 声明 | 证据 |
|---|---|
| 可构建 | `cargo check -p easydoc-derive` |
| 测试 | trybuild 编译失败测试 + 单元测试 |
| MSRV | CI MSRV 任务（Rust 1.88） |
| crates.io | 已发布 | [v0.1.0-alpha.1](https://crates.io/crates/easydoc-derive) / [docs.rs](https://docs.rs/easydoc-derive) |

---

## 2. 快速开始

### 2.1 安装

```toml
[dependencies]
easydoc-derive = "0.1.0-alpha.1"
easydoc-core = "0.1.0-alpha.1"
```

### 2.2 最小示例

```rust
use easydoc_derive::DocxRow;

#[derive(DocxRow)]
#[docx(banded_rows = true)]
struct Report {
    #[docx(name = "序号", order = 0, width = "2cm")]
    id: u32,

    #[docx(name = "金额", order = 1, format = "#,##0.00", align = "right")]
    amount: f64,

    #[docx(name = "日期", order = 2, format = "yyyy-mm-dd")]
    date: String,

    #[docx(name = "状态", order = 3, converter = StatusConverter)]
    status: String,

    #[docx(name = "备注", order = 4, wrap = true)]
    note: Option<String>,

    #[docx(ignore)]
    internal_id: String,
}
```

derive 生成的内容：
- `schema()` -- 返回 `&'static [TableColumn]`，包含列元数据
- `from_row()` / `from_row_with_converters()` -- 将 `RowData` 反序列化为结构体
- `to_row()` / `to_row_with_converters()` -- 将结构体序列化为 `Vec<CellData>`

---

## 3. 属性参考

### 3.1 字段属性

| 属性 | 类型 | 默认值 | 描述 |
|---|---|---|---|
| `name` | 字符串字面量 | 字段名 | 列标题文本 |
| `index` | 整数字面量 | 声明顺序 | 从零开始的列索引 |
| `order` | 整数字面量 | 声明顺序 | 列排序顺序（值越小越靠左） |
| `width` | 字符串字面量 | 无 | 列宽（`"2cm"`、`"80px"`、`"auto"`） |
| `format` | 字符串字面量 | 无 | 数字/日期格式（`"#,##0.00"`、`"yyyy-mm-dd"`） |
| `align` | 字符串字面量 | 无 | 水平对齐：`left`、`center`、`right`、`justify`、`both` |
| `converter` | 类型路径 | 无 | 自定义转换器类型（需实现 converter trait） |
| `wrap` | bool 字面量 | `false` | 启用文本换行 |
| `ignore` | 标志 | — | 读写时跳过此字段 |

### 3.2 结构体属性

| 属性 | 类型 | 默认值 | 描述 |
|---|---|---|---|
| `banded_rows` | bool 字面量 | `false` | 启用表格斑马条纹 |
| `table_width` / `auto_width` | bool 字面量 | `false` | 自动适配表格宽度 |

### 3.3 属性到 OOXML 的映射

| 属性 | 生成的代码 | OOXML 效果 |
|---|---|---|
| `name` | `TableColumn.name` | `<w:t>` 表头文本内容 |
| `width` | `TableColumn.width` | `<w:tcW>` 单元格宽度 |
| `format` | `TableColumn.format` | `<w:numFmt>` 或显示格式 |
| `align` | `HorizontalAlignment::*` | `<w:jc>` 对齐方式 |
| `wrap` | `TableColumn.wrap` | `<w:tcPr><w:wrap/>` |
| `converter` | 通过 `ConverterRegistry` 分发 | 自定义值转换 |
| `ignore` | 字段从 schema 和行中排除 | 输出中不包含该字段 |

### 3.4 自定义转换器示例

```rust
use easydoc_core::{DocValue, TableColumn, Converter};

pub struct StatusConverter;

impl Converter<String> for StatusConverter {
    fn to_doc_value(&self, value: &String, _col: &TableColumn) -> easydoc_core::Result<DocValue> {
        let display = match value.as_str() {
            "active" => "已激活",
            "inactive" => "未激活",
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

## 4. 上游兼容

**上游项目**：Java [EasyExcel](https://github.com/alibaba/easyexcel) 4.0.3

| Java 机制 | Rust 设计 | 原因 |
|---|---|---|
| `@ExcelProperty` 注解 | `#[derive(DocxRow)]` + `#[docx(...)]` | 编译期元数据，无反射 |
| 反射读写 | 生成的 `from_row()` / `to_row()` | 静态分发，类型安全 |
| `Converter` 接口 | `Converter<T>` trait + `ConverterRegistry` | 显式注册，无类路径扫描 |
| `null` | `Option<T>` | 空值显式处理 |
| 异常 | `Result<T, DocError>` | 无隐式控制流 |

| 上游能力 | Rust 对应 | 状态 | 差异 |
|---|---|---|---|
| 列名映射 | `name` 属性 | 稳定 | -- |
| 列排序 | `order` 属性 | 稳定 | -- |
| 列宽 | `width` 属性 | 稳定 | -- |
| 数字/日期格式 | `format` 属性 | 稳定 | -- |
| 对齐 | `align` 属性 | 稳定 | -- |
| 自定义转换器 | `converter` 属性 | 稳定 | 显式注册，无类路径扫描 |
| 忽略字段 | `ignore` 属性 | 稳定 | -- |
| 斑马条纹 | `banded_rows` 结构体属性 | 稳定 | -- |
| 自动宽度 | `table_width` 结构体属性 | 稳定 | -- |

---

## 5. 质量

### 5.1 构建门禁

```bash
cargo fmt --all -- --check
cargo clippy -p easydoc-derive -- -D warnings
cargo check -p easydoc-derive
cargo test -p easydoc-derive
```

### 5.2 测试类型

| 类型 | 目的 | 工具 |
|---|---|---|
| 编译失败测试 | 非法属性检测 | trybuild |
| 单元测试 | 属性解析、对齐验证 | `cargo test` |
| 文档测试 | 公共 API 示例 | `cargo test --doc` |

---

## 6. 项目结构

```text
crates/easydoc-derive/
├── Cargo.toml
├── src/
│   ├── lib.rs              # 入口，derive_docx_row
│   └── implementation.rs   # Token 展开、属性解析
└── tests/
    └── trybuild/            # 编译失败测试用例
```

---

## 7. 许可证

采用 [Apache-2.0](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE) 许可证。

---

<div align="center">

[返回顶部](#readme-top) · [docs.rs](https://docs.rs/easydoc-derive) · [crates.io](https://crates.io/crates/easydoc-derive) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
