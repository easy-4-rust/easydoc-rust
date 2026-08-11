<a id="readme-top"></a>

<div align="center">

# easydoc-ooxml

**Safe OOXML ZIP package primitives for the easydoc-rust workspace**

[![Crates.io](https://img.shields.io/crates/v/easydoc-ooxml)](https://crates.io/crates/easydoc-ooxml)
[![docs.rs](https://img.shields.io/docsrs/easydoc-ooxml)](https://docs.rs/easydoc-ooxml)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#rust-baseline)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE)

[English](README.md) | [简体中文](README_zh.md)

[Overview](#1-overview) | [API](#2-api) | [Security Limits](#3-security-limits) |
[Quick Start](#4-quick-start) | [Quality](#5-quality)

</div>

---

> **Current version**: `0.1.0-alpha.1`
> **MSRV**: Rust `1.88`
> **Edition**: `2024`
> **Maturity**: Preview
> **Last verified**: 2026-08-11

---

## 1. Overview

**easydoc-ooxml provides atomic file writing and safe ZIP package rewriting for OOXML documents. It is the low-level building block used by `easydoc-template` and other crates in the workspace.**

### 1.1 What it is

| Dimension | Value |
|---|---|
| Crate | `easydoc-ooxml` |
| Current version | `0.1.0-alpha.1` |
| MSRV / Edition | `1.88` / `2024` |
| unsafe policy | `deny` (crate-level `#![deny(unsafe_code)]`) |
| License | `Apache-2.0` |

### 1.2 What it is not

- Not a general-purpose ZIP library; it is purpose-built for OOXML package rewriting.
- Not a DOCX parser; it operates on ZIP entries without interpreting the XML content.
- Not a 1:1 port of any Java library; it is original infrastructure for the easydoc-rust workspace.

### 1.3 Why atomic writing matters

OOXML documents (`.docx`, `.xlsx`, `.pptx`) are ZIP archives. If a write operation is interrupted mid-way (crash, power loss, disk full), the output file is left in a corrupted state -- partially written, with a broken ZIP structure. `AtomicFile` solves this:

```text
Normal write (dangerous):
  Open target -> Write bytes -> Crash midway -> Corrupted file

AtomicFile::write (safe):
  Create temp file in same directory
  -> Write all bytes to temp
  -> flush() + sync_all()
  -> persist() atomically replaces target
  -> Crash at any point before persist() leaves target unchanged
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

`AtomicFile::write` creates a temporary file in the same directory as the target, calls the provided closure to write content, then atomically replaces the target via `persist()`. If the closure or persist fails, the original target file is left unchanged.

### 2.2 PackageRewriter

```rust
use easydoc_ooxml::PackageRewriter;

PackageRewriter::default().rewrite("input.docx", "output.docx", |name, content| {
    if name == "word/document.xml" {
        // Transform this entry
        Ok(Some(modified_content))
    } else {
        // Preserve unchanged
        Ok(None)
    }
})?;
```

`PackageRewriter` opens an OOXML ZIP archive, iterates all entries, and applies a transform function. Returning `Some(bytes)` replaces the entry; returning `None` preserves the original bytes. The output is written atomically via `AtomicFile`.

Key properties:
- Preserves compression method, timestamps, and Unix permissions per entry
- Validates archive limits before processing
- Uses `AtomicFile` for corruption-safe output

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

## 3. Security Limits

`PackageLimits` protects against malformed or malicious OOXML packages that attempt to exhaust memory or disk.

| Limit | Default | Purpose |
|---|---|---|
| `max_entries` | 10,000 | Maximum number of ZIP entries |
| `max_entry_bytes` | 256 MB | Maximum uncompressed size per entry |
| `max_total_bytes` | 1 GB | Maximum total uncompressed size |
| `max_compression_ratio` | 1,000 | Maximum compression ratio per entry (ZIP bomb protection) |

Validation runs before any entry is read into memory. If any limit is exceeded, a `DocError::Format` is returned and no output is produced.

---

## 4. Quick Start

### 4.1 Installation

```toml
[dependencies]
easydoc-ooxml = "0.1.0-alpha.1"
```

### 4.2 Atomic file write

```rust
use easydoc_ooxml::AtomicFile;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AtomicFile::write("important.docx", |file| {
        file.write_all(b"PK\x03\x04...")?;
        Ok(())
    })?;
    // If the process crashes during write, important.docx is unchanged.
    Ok(())
}
```

### 4.3 Rewrite an OOXML package

```rust
use easydoc_ooxml::PackageRewriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    PackageRewriter::default().rewrite(
        "input.docx",
        "output.docx",
        |name, content| {
            if name == "word/document.xml" {
                let xml = std::str::from_utf8(content)?;
                let modified = xml.replace("placeholder", "actual value");
                Ok(Some(modified.into_bytes()))
            } else {
                Ok(None) // preserve unchanged
            }
        },
    )?;
    Ok(())
}
```

---

## 5. Quality

### 5.1 Build gates

```bash
cargo fmt --all -- --check
cargo clippy -p easydoc-ooxml -- -D warnings
cargo check -p easydoc-ooxml
cargo test -p easydoc-ooxml
```

### 5.2 Test types

| Type | Purpose | Scope |
|---|---|---|
| Unit tests | AtomicFile write, PackageRewriter rewrite, limit validation | `src/` |
| Integration tests | End-to-end ZIP round-trip with real DOCX files | `tests/` |

---

## 6. Project Structure

```text
crates/easydoc-ooxml/
├── Cargo.toml
└── src/
    ├── lib.rs               # Public API re-exports
    ├── atomic_file.rs       # AtomicFile: temp + flush + sync + persist
    ├── package_limits.rs    # PackageLimits: security bounds
    └── package_rewriter.rs  # PackageRewriter: ZIP entry iteration + transform
```

---

## 7. License

Licensed under [Apache-2.0](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE).

---

<div align="center">

[Back to top](#readme-top) · [docs.rs](https://docs.rs/easydoc-ooxml) · [crates.io](https://crates.io/crates/easydoc-ooxml) · [Issues](https://github.com/easy-4-rust/easydoc-rust/issues)

</div>
