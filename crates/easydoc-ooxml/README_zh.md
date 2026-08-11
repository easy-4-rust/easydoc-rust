<a id="readme-top"></a>

<div align="center">

# easydoc-ooxml

**easydoc-rust 工作区的安全 OOXML ZIP 包底层原语**

[![Crates.io](https://img.shields.io/crates/v/easydoc-ooxml)](https://crates.io/crates/easydoc-ooxml)
[![docs.rs](https://img.shields.io/docsrs/easydoc-ooxml)](https://docs.rs/easydoc-ooxml)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#rust-基线)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[项目定位](#1-项目定位) | [API](#2-api) | [安全限制](#3-安全限制) |
[快速开始](#4-快速开始) | [质量](#5-质量)

</div>

---

> **当前版本**：`0.1.0-alpha.1`
> **MSRV**：Rust `1.88`
> **Edition**：`2024`
> **成熟度**：预览
> **最后核验**：2026-08-11

---

## 1. 项目定位

**easydoc-ooxml 提供原子文件写入和安全 ZIP 包重写功能，用于 OOXML 文档。它是 `easydoc-template` 以及工作区其他 crate 使用的底层构建块。**

### 1.1 是什么

| 维度 | 内容 |
|---|---|
| crate | `easydoc-ooxml` |
| 当前版本 | `0.1.0-alpha.1` |
| MSRV / Edition | `1.88` / `2024` |
| unsafe 策略 | `deny`（crate 级 `#![deny(unsafe_code)]`） |
| 许可证 | `Apache-2.0` |

### 1.2 不是什么

- 不是通用 ZIP 库；专为 OOXML 包重写设计。
- 不是 DOCX 解析器；操作 ZIP 条目但不解释 XML 内容。
- 不是任何 Java 库的移植；是 easydoc-rust 工作区的原创基础设施。

### 1.3 为什么需要原子写入

OOXML 文档（`.docx`、`.xlsx`、`.pptx`）是 ZIP 归档。如果写入操作中途被中断（崩溃、断电、磁盘满），输出文件将处于损坏状态——部分写入，ZIP 结构损坏。`AtomicFile` 解决了这个问题：

```text
普通写入（危险）：
  打开目标 -> 写入字节 -> 中途崩溃 -> 文件损坏

AtomicFile::write（安全）：
  在同一目录创建临时文件
  -> 将所有字节写入临时文件
  -> flush() + sync_all()
  -> persist() 原子替换目标文件
  -> persist() 之前的任何崩溃都不会影响目标文件
```

---

## 2. API

### 2.1 AtomicFile

```rust
use easydoc_ooxml::AtomicFile;
use std::io::Write;

AtomicFile::write("output.docx", |file| {
    file.write_all(b"content")?;
    Ok(())
})?;
```

`AtomicFile::write` 在目标文件所在目录创建临时文件，调用提供的闭包写入内容，然后通过 `persist()` 原子替换目标文件。如果闭包或 persist 失败，原始目标文件保持不变。

### 2.2 PackageRewriter

```rust
use easydoc_ooxml::PackageRewriter;

PackageRewriter::default().rewrite("input.docx", "output.docx", |name, content| {
    if name == "word/document.xml" {
        // 转换此条目
        Ok(Some(modified_content))
    } else {
        // 保留不变
        Ok(None)
    }
})?;
```

`PackageRewriter` 打开 OOXML ZIP 归档，遍历所有条目，并应用转换函数。返回 `Some(bytes)` 替换条目；返回 `None)` 保留原始字节。输出通过 `AtomicFile` 原子写入。

关键特性：
- 保留每个条目的压缩方式、时间戳和 Unix 权限
- 处理前验证归档限制
- 使用 `AtomicFile` 实现防损坏输出

### 2.3 PackageLimits

```rust
use easydoc_ooxml::{PackageRewriter, PackageLimits};

let limits = PackageLimits {
    max_entries: 5_000,
    max_entry_bytes: 128 * 1024 * 1024,
    max_total_bytes: 512 * 1024 * 1024,
    max_compression_ratio: 500,
};
let rewriter = PackageRewriter::new(limits);
```

---

## 3. 安全限制

`PackageLimits` 防止试图耗尽内存或磁盘的畸形或恶意 OOXML 包。

| 限制 | 默认值 | 目的 |
|---|---|---|
| `max_entries` | 10,000 | ZIP 条目最大数量 |
| `max_entry_bytes` | 256 MB | 单个条目最大解压大小 |
| `max_total_bytes` | 1 GB | 所有条目最大解压总大小 |
| `max_compression_ratio` | 1,000 | 单个条目最大压缩比（ZIP bomb 防护） |

验证在任何条目读入内存之前执行。如果超出任何限制，返回 `DocError::Format`，不产生输出。

---

## 4. 快速开始

### 4.1 安装

```toml
[dependencies]
easydoc-ooxml = "0.1.0-alpha.1"
```

### 4.2 原子文件写入

```rust
use easydoc_ooxml::AtomicFile;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AtomicFile::write("important.docx", |file| {
        file.write_all(b"PK\x03\x04...")?;
        Ok(())
    })?;
    // 如果写入过程中进程崩溃，important.docx 保持不变。
    Ok(())
}
```

### 4.3 重写 OOXML 包

```rust
use easydoc_ooxml::PackageRewriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    PackageRewriter::default().rewrite(
        "input.docx",
        "output.docx",
        |name, content| {
            if name == "word/document.xml" {
                let xml = std::str::from_utf8(content)?;
                let modified = xml.replace("占位符", "实际值");
                Ok(Some(modified.into_bytes()))
            } else {
                Ok(None) // 保留不变
            }
        },
    )?;
    Ok(())
}
```

---

## 5. 质量

### 5.1 构建门禁

```bash
cargo fmt --all -- --check
cargo clippy -p easydoc-ooxml -- -D warnings
cargo check -p easydoc-ooxml
cargo test -p easydoc-ooxml
```

### 5.2 测试类型

| 类型 | 目的 | 范围 |
|---|---|---|
| 单元测试 | AtomicFile 写入、PackageRewriter 重写、限制验证 | `src/` |
| 集成测试 | 使用真实 DOCX 文件的端到端 ZIP 往返 | `tests/` |

---

## 6. 项目结构

```text
crates/easydoc-ooxml/
├── Cargo.toml
└── src/
    ├── lib.rs               # 公共 API 重导出
    ├── atomic_file.rs       # AtomicFile：临时文件 + flush + sync + persist
    ├── package_limits.rs    # PackageLimits：安全边界
    └── package_rewriter.rs  # PackageRewriter：ZIP 条目遍历 + 转换
```

---

## 7. 许可证

采用 [Apache-2.0](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE) 许可证。

---

<div align="center">

[返回顶部](#readme-top) · [docs.rs](https://docs.rs/easydoc-ooxml) · [crates.io](https://crates.io/crates/easydoc-ooxml) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
