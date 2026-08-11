<a id="readme-top"></a>

<div align="center">

# easydoc-template

**easydoc-rust 工作区的 DOCX 模板填充引擎**

[![Crates.io](https://img.shields.io/crates/v/easydoc-template)](https://crates.io/crates/easydoc-template)
[![docs.rs](https://img.shields.io/docsrs/easydoc-template)](https://docs.rs/easydoc-template)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#rust-基线)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[项目定位](#1-项目定位) | [模板语义](#2-模板语义) | [快速开始](#3-快速开始) |
[配置](#4-配置) | [上游兼容](#5-上游兼容) | [质量](#6-质量)

</div>

---

> **状态**：alpha 预发布（最新版见 [crates.io](https://crates.io/crates/easydoc-template)）
> **MSRV**：Rust `1.88`
> **Edition**：`2024`
> **成熟度**：预览
> **最后核验**：2026-08-11

---

## 1. 项目定位

**easydoc-template 检测 DOCX 模板中的 `{key}` 和 `{.field}` 占位符，并用提供的数据替换，同时保留 ZIP 结构。**

### 1.1 是什么

| 维度 | 内容 |
|---|---|
| crate | `easydoc-template` |
| 状态 | Alpha 预发布（最新版见 crates.io） |
| MSRV / Edition | `1.88` / `2024` |
| unsafe 策略 | `deny`（crate 级 `#![deny(unsafe_code)]`） |
| 许可证 | `Apache-2.0` |

### 1.2 不是什么

- 不是完整的 DOCX 编辑库；仅对已有模板执行占位符替换。
- 不是邮件合并引擎；不拆分文档或处理条件段落。
- 不是 Java poi-tl 或 hutool 模板的 1:1 移植；将填充概念适配到 Rust 的所有权模型。

### 1.3 处理流水线

```text
DOCX 模板（ZIP）
        |
        v
PackageRewriter 打开 ZIP，验证资源限制
        |
        v
提取 word/document.xml 为 UTF-8
        |
        +-- 标量：  {key}  ->  值
        +-- 列表：  {.field}  ->  每项复制一行
        |
        v
XML 文本替换（跨节点感知）
        |
        v
通过 AtomicFile 写入新 DOCX（临时文件 + flush + sync + persist）
```

---

## 2. 模板语义

### 2.1 占位符语法

| 语法 | 类型 | 示例 | 行为 |
|---|---|---|---|
| `{key}` | 标量 | `{name}`、`{date}` | 替换为单个值 |
| `{.field}` | 集合 | `{.name}`、`{.age}` | 每个数据项复制一行/段落 |
| `{prefix.field}` | 命名集合 | `{user.name}` | 命名组内的字段 |

跨越多个 `<w:t>` 节点的占位符（被 Word 拆分）可正确处理。

### 2.2 作用域与扩展方向

| 作用域 | 扩展方向 | 行为 |
|---|---|---|
| 段落（`<w:p>`） | 纵向 | 每个集合项复制一段落 |
| 表格行（`<w:tr>`） | 纵向 | 每个集合项复制一行 |

### 2.3 样式继承

填充的单元格默认继承占位符单元格的段落和 run 属性（由 `FillConfig.auto_style` 控制）。

### 2.4 缺失与空值

- 缺失的标量键：占位符文本保持不变。
- 空值：替换为空字符串。
- 值中的 XML 特殊字符（`&`、`<`、`>`、`"`、`'`）会被转义。

### 2.5 幂等性

每次 `fill_template` / `fill_template_list` 调用都从模板读取并写入新文件。模板文件不会被原地修改。多次使用相同输入调用产生相同输出。

---

## 3. 快速开始

### 3.1 安装

```toml
[dependencies]
easydoc-template = "0.1.0-alpha"
serde = { version = "1", features = ["derive"] }
```

### 3.2 标量填充

```rust
use std::collections::HashMap;
use easydoc_template::fill_template;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = HashMap::new();
    data.insert("name".to_owned(), "张三".to_owned());
    data.insert("date".to_owned(), "2026-08-11".to_owned());

    fill_template(
        std::path::Path::new("template.docx"),
        std::path::Path::new("output.docx"),
        &data,
    )?;
    Ok(())
}
```

### 3.3 列表填充

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
        Item { name: "张三".into(), age: 30 },
        Item { name: "李四".into(), age: 25 },
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
        .register("title", "月度报告")
        .register("author", "财务部")
        .do_fill()?;
    Ok(())
}
```

---

## 4. 配置

### 4.1 FillConfig

| 字段 | 类型 | 默认值 | 描述 |
|---|---|---|---|
| `direction` | `FillDirection` | `Vertical` | 集合扩展方向 |
| `force_new_row` | `bool` | `true` | 每个集合项插入新行 |
| `auto_style` | `bool` | `true` | 继承占位符单元格样式 |

### 4.2 FillDirection

| 变体 | 行为 |
|---|---|
| `Vertical` | 集合项作为新行扩展（默认） |
| `Horizontal` | 集合项作为新列扩展 |

---

## 5. 上游兼容

**上游项目**：Java [EasyExcel](https://github.com/alibaba/easyexcel) 4.0.3 模板填充（`easyexcel-template`）

| 上游能力 | Rust 对应 | 状态 | 差异 |
|---|---|---|---|
| `{key}` 标量填充 | `fill_template()` | 稳定 | -- |
| `{.field}` 集合填充 | `fill_template_list()` | 稳定 | -- |
| 样式继承 | `FillConfig.auto_style` | 稳定 | -- |
| 纵向扩展 | `FillDirection::Vertical` | 稳定 | -- |
| 横向扩展 | `FillDirection::Horizontal` | 稳定 | -- |
| `FillConfig` builder | `TemplateFillBuilder` | 稳定 | 方法链式调用 |

---

## 6. 质量

### 6.1 构建门禁

```bash
cargo fmt --all -- --check
cargo clippy -p easydoc-template -- -D warnings
cargo check -p easydoc-template
cargo test -p easydoc-template
```

### 6.2 测试类型

| 类型 | 目的 | 范围 |
|---|---|---|
| 单元测试 | 占位符解析、XML 转义、跨节点替换 | `fill_executor.rs` |
| 集成测试 | 使用真实 DOCX 文件的端到端模板填充 | `tests/` |
| 文档测试 | 公共 API 示例 | `cargo test --doc` |

---

## 7. 项目结构

```text
crates/easydoc-template/
├── Cargo.toml
└── src/
    ├── lib.rs                 # 公共 API 重导出
    ├── fill_config.rs         # FillConfig、FillDirection
    ├── fill_executor.rs       # 核心填充逻辑、跨节点替换
    ├── fill_template.rs       # 标量填充入口
    ├── fill_template_list.rs  # 列表填充入口
    └── placeholder.rs         # 占位符检测与解析
```

---

## 8. 许可证

采用 [Apache-2.0](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE) 许可证。

---

<div align="center">

[返回顶部](#readme-top) · [docs.rs](https://docs.rs/easydoc-template) · [crates.io](https://crates.io/crates/easydoc-template) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
