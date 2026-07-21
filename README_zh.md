# easydoc-rs

**Rust 快捷 DOC/DOCX 文档操作库。**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)

> `easydoc-rs` 是 [`easyexcel-rs`](https://github.com/hiwepy/easyexcel-rs) 在 DOC/DOCX 领域的平行实现，遵循相同的流式 Builder + Trait 扩展 + Proc-Macro 架构模式。

---

## 功能特性

| 功能 | 状态 |
|:---|:---:|
| **写入 DOCX** — 段落、标题、表格、分页、格式化 Run | ✅ |
| **快捷表格写入** — 一行代码 `Vec<Struct>` → DOCX 表格 | ✅ |
| **模板填充** — `{key}` 占位符替换，ZIP 结构保留 | ✅ |
| **读取 DOCX/DOC** — 文本提取、表格提取 (via office_oxide) | ✅ |
| **格式自动检测** — DOCX (ZIP 魔数) vs DOC (OLE2 魔数) | ✅ |
| **`#[derive(DocxRow)]`** — 编译时 struct 到表格行映射 | ✅ |
| **样式系统** — FontConfig、ParagraphStyle、TableStyle、Color | ✅ |
| **可扩展转换器** — 自定义 `DocConverter<T>` 注册表 | ✅ |
| **写入生命周期钩子** — `DocWriteHandler` 文档/段落/表格/单元格级别 | ✅ |
| **流式读取监听器** — `DocReadListener<T>` 事件驱动解析 | ✅ |

---

## 快速开始

在 `Cargo.toml` 中添加：

```toml
[dependencies]
easydoc = "0.1"
```

### 从结构体数据写表格

```rust
use easydoc::prelude::*;

#[derive(DocxRow)]
#[docx(banded_rows = true)]
struct User {
    #[docx(name = "姓名", width = 0.3, order = 0)]
    name: String,
    #[docx(name = "年龄", width = 0.15, order = 1)]
    age: u32,
    #[docx(name = "邮箱", width = 0.55, order = 2)]
    email: String,
}

let users = vec![
    User { name: "张三".into(), age: 30, email: "zhangsan@e.com".into() },
    User { name: "李四".into(), age: 25, email: "lisi@e.com".into() },
];

EasyDoc::write_table("users.docx", &users)
    .title("用户报表")
    .header_style(TableStyle::header())
    .banded_rows(true)
    .do_write()?;
```

### 构建完整文档

```rust
EasyDoc::document("report.docx")
    .title("年度报告")
    .author("张三")
    .add_heading("第一章：概述", HeadingLevel::H1)
    .add_paragraph(
        Paragraph::new()
            .add_text("这是正文内容，其中")
            .add_run(Run::new("高亮部分").bold().color(0xFF0000))
            .add_text(" 已标注。")
            .alignment(HorizontalAlignment::Both)
    )
    .add_table(Table::from_data(&users).banded_rows(true))
    .add_page_break()
    .save()?;
```

### 模板填充

```rust
use std::collections::HashMap;

let mut data = HashMap::new();
data.insert("name".into(), "张三".into());
data.insert("date".into(), "2026-07-21".into());

EasyDoc::fill_template("template.docx", "output.docx", &data)?;
```

### 读取文档

```rust
// 提取全部文本
let text = EasyDoc::read_text("document.docx")?;

// 提取表格并反序列化为结构体
let tables: Vec<Vec<User>> = EasyDoc::read_tables::<User>("document.docx")?;

// 透明支持 DOCX 和 DOC 格式
let text = EasyDoc::read_text("legacy.doc")?;
```

---

## 项目架构

```
easydoc-rs/
├── Cargo.toml                          workspace 清单
├── crates/
│   ├── easydoc/                        门面 — EasyDoc 静态工厂
│   ├── easydoc-core/                   核心类型、trait、错误、样式
│   ├── easydoc-derive/                 proc-macro #[derive(DocxRow)]
│   ├── easydoc-writer/                 DOCX 生成 (via docx-rs)
│   ├── easydoc-reader/                 DOCX/DOC 读取 (via office_oxide)
│   └── easydoc-template/              占位符替换、ZIP 保留式修改
```

详细架构见 [docs/architecture.md](docs/architecture.md)。

---

## 后端依赖

| 功能 | Crate | 版本 |
|:---|:---|:---|
| DOCX 写入 | [`docx-rs`](https://crates.io/crates/docx-rs) | 0.4.20 |
| DOCX/DOC 读取 | [`office_oxide`](https://crates.io/crates/office_oxide) | 0.1.7 |

---

## 设计原则

| # | 原则 | 继承自 |
|:---|:---|:---|
| 1 | **静态工厂** — `EasyDoc` 是所有操作的唯一入口 | easyexcel-rs `EasyExcel` |
| 2 | **流式 Builder** — `mut self -> Self` + `#[must_use]` | easyexcel-rs builder 模式 |
| 3 | **Trait 扩展** — `DocxRow`、`DocConverter`、`DocWriteHandler`、`DocReadListener` | easyexcel-rs traits |
| 4 | **Proc-Macro 代码生成** — `#[derive(DocxRow)]` 编译时展开 | easyexcel-rs `#[derive(ExcelRow)]` |
| 5 | **后端无关** — 统一 API，可替换引擎 | easyexcel-rs 多格式 |
| 6 | **单一错误类型** — `DocError` 枚举 + `thiserror` | easyexcel-rs `ExcelError` |
| 7 | **零 unsafe** — `#![forbid(unsafe_code)]` 每个 crate | easyexcel-rs 安全策略 |

---

## 测试

```bash
# 运行所有测试（11 个全部通过）
cargo test --workspace
```

测试覆盖：
- 写简单表格，验证生成有效 DOCX ZIP 结构
- 写完整文档（段落、标题、样式）
- 往返测试：写 -> 读文本 -> 验证内容
- 往返测试：写表格 -> 读表格 -> 验证数据
- 模板标量填充（多占位符）
- 模板填充端到端
- 图片插入、样式构建器、类型转换器、格式检测、错误变体、生命周期钩子

---

## 文档

| 文档 | 说明 |
|:---|:---|
| [使用指南](docs/usage-guide.md) | 完整使用指南，含实战案例和 API 速查 |
| [架构设计](docs/architecture.md) | 完整架构、数据流、设计决策 |
| [API 速查](#10-api-reference-接口速查) | 使用指南中的快速参考 |

---

## License

Apache-2.0 — 详见 [LICENSE](LICENSE)。

## 相关项目

- [`easyexcel-rs`](https://github.com/hiwepy/easyexcel-rs) — Excel 对应实现
- [`easypdf-rs`](https://github.com/hiwepy/easypdf-rs) — PDF 文档操作
