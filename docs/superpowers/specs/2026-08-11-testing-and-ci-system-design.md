# 测试与 CI 体系设计

- **日期**：2026-08-11
- **作者**：ZCode Agent（协同设计）
- **状态**：已部分实现，本文档为补全设计
- **依赖**：所有 crate 的测试、Cargo workspace、GitHub Actions

## 1. 目标与范围

为 easydoc-rust 建立**多层次、自动化**的测试与 CI 体系，覆盖单元测试、集成测试、模糊测试、保真度基准测试、跨平台 CI 和代码质量门禁。

**核心需求**：

1. **单元测试**：每个 crate 内部模块的独立测试（当前 174+ 通过）。
2. **集成测试**：跨 crate 的端到端测试（write → read → verify）。
3. **Fuzz 测试**：proptest 属性测试 + cargo-fuzz 模糊测试。
4. **保真度基准测试**：DOCX → Markdown → DOCX 的往返保真度验证。
5. **Golden tests**：真实文档集的输出对比测试。
6. **跨平台 CI**：Linux / macOS / Windows 的 GitHub Actions 矩阵。
7. **代码质量门禁**：fmt + clippy + doc + test 全通过才能合并。

**非目标**：

- 不做性能基准测试（Criterion）——Phase 5 设计目标。
- 不做安全审计（依赖第三方工具）。
- 不做负载测试（库不是服务）。

## 2. 总体架构

```
┌───────────────────────────────────────────────────────────────────┐
│                        测试金字塔                                  │
│                                                                   │
│                          ┌─────────┐                              │
│                          │  E2E    │  端到端测试                   │
│                          │ (golden)│  真实文档集                   │
│                        ┌─┴─────────┴─┐                            │
│                        │  集成测试    │  跨 crate 往返             │
│                      ┌─┴─────────────┴─┐                          │
│                      │    Fuzz 测试     │  proptest + cargo-fuzz   │
│                    ┌─┴─────────────────┴─┐                        │
│                    │     单元测试         │  每个模块独立           │
│                  ┌─┴─────────────────────┴─┐                      │
│                  │     静态分析              │  fmt + clippy + doc  │
│                  └───────────────────────────┘                      │
└───────────────────────────────────────────────────────────────────┘
```

```
┌───────────────────────────────────────────────────────────────────┐
│                        CI 流水线                                   │
│                                                                   │
│  git push / PR                                                    │
│      │                                                            │
│      ▼                                                            │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐      │
│  │ cargo fmt│──►│cargo     │──►│cargo doc │──►│cargo test│      │
│  │ --check  │   │clippy    │   │--no-deps │   │--workspace│     │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘      │
│      │              │              │              │               │
│      ▼              ▼              ▼              ▼               │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐      │
│  │ Linux    │   │ macOS    │   │ Windows  │   │ Coverage │      │
│  │ (ubuntu) │   │ (latest) │   │ (latest) │   │ (llvm-   │      │
│  │          │   │          │   │          │   │  cov)    │      │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘      │
│                                                                   │
│  全部通过 → ✅ 可合并                                              │
│  任一失败 → ❌ 阻止合并                                            │
└───────────────────────────────────────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 测试层次

| 层次 | 工具 | 位置 | 覆盖目标 |
|---|---|---|---|
| 静态分析 | `cargo fmt --check` | CI | 代码格式 |
| 静态分析 | `cargo clippy -D warnings` | CI | 代码质量 |
| 静态分析 | `cargo doc --no-deps` | CI | 文档完整性 |
| 单元测试 | `#[cfg(test)]` | 每个 crate 的 `src/` | 模块内部逻辑 |
| 集成测试 | `#[test]` | `crates/*/tests/` | 跨 crate 端到端 |
| 属性测试 | `proptest` | `crates/*/tests/` | 边界条件、不变量 |
| 模糊测试 | `cargo-fuzz` | `fuzz/` | 崩溃、panic、OOM |
| Golden tests | 输出对比 | `tests/fixtures/` | 回归检测 |
| 覆盖率 | `llvm-cov` | CI | 行/函数覆盖率 |

### 3.2 测试文件分布

```
easydoc-rust/
├── crates/
│   ├── easydoc-core/tests/         core 模型测试
│   ├── easydoc-ooxml/tests/
│   │   └── package_rewriter_test.rs   安全限制 + 保真度测试
│   ├── easydoc-reader/tests/       读取 + 格式检测测试
│   ├── easydoc-writer/tests/
│   │   └── writer_test.rs             写入 + 往返 + derive 测试
│   ├── easydoc-template/tests/
│   │   └── binary_fidelity_test.rs    模板填充 + XML 转义测试
│   ├── easydoc-markdown/tests/
│   │   └── markdown_conversion_test.rs Markdown 转换测试
│   └── easydoc-mcp/tests/
│       └── server_test.rs             MCP 协议测试
├── tests/                              [待建] 跨 crate 集成测试
│   ├── integration_test.rs
│   └── fixtures/                       [待建] 真实文档集
│       ├── sample.docx
│       ├── sample.doc
│       ├── tables.docx
│       ├── images.docx
│       └── complex.docx
├── fuzz/                               [待建] cargo-fuzz 目标
│   ├── Cargo.toml
│   └── fuzz_targets/
│       ├── fuzz_reader.rs
│       ├── fuzz_template.rs
│       └── fuzz_markdown.rs
└── .github/workflows/
    └── ci.yml                          [待建] GitHub Actions
```

## 4. 关键数据流

### 4.1 单元测试流程

```
cargo test --workspace
    │
    ├── easydoc-core: 模型创建、序列化、trait 实现
    ├── easydoc-ooxml: PackageLimits、PackageRewriter、AtomicFile
    ├── easydoc-reader: 格式检测、文本提取、表格提取、语义提取
    ├── easydoc-writer: 文档构建、表格写入、往返、derive
    ├── easydoc-template: 占位符填充、XML 转义、跨 Run
    ├── easydoc-markdown: Markdown 转换、front matter、图片
    ├── easydoc-mcp: MCP 协议、工具调用、资源读取
    └── easydoc: 门面 API 集成测试
    │
    ▼
174+ tests passed, 0 failed, 8 ignored
```

### 4.2 集成测试流程（待建）

```
#[test]
fn test_write_read_modify_write_roundtrip() {
    // 1. 写入文档
    EasyDoc::document("test.docx")
        .add_heading("Title", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("Hello"))
        .add_table(Table::from_data(&users))
        .save()?;

    // 2. 读取为语义模型
    let content = EasyDoc::load("test.docx")?;
    assert_eq!(content.blocks.len(), 3);

    // 3. 修改
    if let DocumentBlock::Paragraph(ref mut runs) = content.blocks[1] {
        runs[0].text = "World".into();
    }

    // 4. 写回
    EasyDoc::write_content(&content, "test2.docx")?;

    // 5. 验证
    let text = EasyDoc::read_text("test2.docx")?;
    assert!(text.contains("World"));
}
```

### 4.3 属性测试流程（待建）

```rust
proptest! {
    #[test]
    fn escape_xml_text_never_panics(s in ".*") {
        // 任意字符串都不会 panic
        let _ = easydoc_template::escape_xml_text(&s);
    }

    #[test]
    fn escape_xml_text_is_idempotent(s in "[a-zA-Z0-9 ]*") {
        // 纯文本两次转义结果相同
        let once = escape_xml_text(&s);
        let twice = escape_xml_text(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn roundtrip_docx_read_write(text in "[a-zA-Z0-9 .,!?]{1,1000}") {
        // 任意文本写入后读取内容一致
        EasyDoc::document("prop.docx")
            .add_paragraph(Paragraph::new().add_text(&text))
            .save()?;
        let read = EasyDoc::read_text("prop.docx")?;
        prop_assert!(read.contains(&text));
    }
}
```

### 4.4 Fuzz 测试流程（待建）

```rust
// fuzz/fuzz_targets/fuzz_reader.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // 尝试将任意字节作为 DOCX 打开
    // 不应 panic，只应返回 Err
    let _ = std::fs::write("/tmp/fuzz.docx", data);
    let _ = easydoc::EasyDoc::read_text("/tmp/fuzz.docx");
});
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | 单元测试放在 `src/` 内（`#[cfg(test)]`） | 与代码同文件，便于维护 | 不能测试公开 API 边界 |
| 2 | 集成测试放在 `tests/` 目录 | 测试公开 API，模拟真实使用 | 编译时间较长 |
| 3 | proptest 做属性测试 | 自动生成边界用例 | 需要定义合理的策略（strategy） |
| 4 | cargo-fuzz 做模糊测试 | 发现 panic 和崩溃 | 需要单独的 fuzz crate |
| 5 | Golden tests 用真实文档 | 覆盖真实场景 | 文档文件占用仓库空间 |
| 6 | CI 用 GitHub Actions | 免费、与 GitHub 集成 | macOS runner 有分钟限制 |
| 7 | 覆盖率用 llvm-cov | Rust 原生支持，精度高 | 仅在 Linux CI 上运行 |

### 5.1 当前测试状态（2026-08-10）

| 指标 | 值 |
|---|---|
| 测试总数 | 174+ |
| 通过 | 174+ |
| 失败 | 0 |
| 忽略 | 8 |
| 行覆盖率 | 73%+ |
| 函数覆盖率 | 79%+ |
| clippy warnings | 0 |
| doc warnings | 0 |
| fmt diff | 无 |

### 5.2 覆盖率目标

| 阶段 | 行覆盖率目标 | 函数覆盖率目标 |
|---|---|---|
| 当前 | 73%+ | 79%+ |
| Phase 4 | 80%+ | 85%+ |
| Phase 5 | 85%+ | 90%+ |

## 6. 测试与验收

### 6.1 CI 门禁检查清单

| 检查项 | 命令 | 阈值 |
|---|---|---|
| 格式 | `cargo fmt --all -- --check` | 无 diff |
| 静态分析 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| 文档 | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | 0 warnings |
| 测试 | `cargo test --workspace` | 0 failures |
| 覆盖率 | `cargo llvm-cov --workspace` | >= 73% |

### 6.2 测试矩阵

| 平台 | Rust 版本 | 说明 |
|---|---|---|
| ubuntu-latest | stable | 主要测试平台 |
| macos-latest | stable | macOS 路径/文件系统差异 |
| windows-latest | stable | Windows 路径/文件系统差异 |
| ubuntu-latest | 1.88 (MSRV) | 最低支持版本 |

### 6.3 待补充测试

- **proptest 属性测试**：XML 转义幂等性、roundtrip 一致性、Unicode 边界。
- **cargo-fuzz 模糊测试**：reader、template、markdown 三个 fuzz target。
- **Golden tests**：`tests/fixtures/` 收集 10+ 真实文档，建立输出快照。
- **跨平台路径测试**：中文路径、空格路径、长路径。
- **内存压力测试**：100MB 文档的处理不 OOM。
- **并发安全测试**：多线程同时读写不同文件。

## 7. 迁移路径

### Phase 5 实施顺序

1. **GitHub Actions CI**：建立基本的 fmt + clippy + test + 多平台矩阵。
2. **proptest 属性测试**：为 `easydoc-ooxml` 和 `easydoc-template` 添加属性测试。
3. **cargo-fuzz 模糊测试**：为 `easydoc-reader` 添加 fuzz target。
4. **Golden tests**：收集真实文档，建立输出快照。
5. **覆盖率集成**：llvm-cov 报告集成到 CI。
6. **测试 fixture 收集**：`tests/fixtures/` 目录建立。

## 8. 引用

- 架构文档：`docs/easydoc-rust-Architecture.zh_CN.md` 第 14 节「测试与验证」
- Roadmap：`docs/roadmap.md` Phase 5（benchmarks、golden tests、fuzz tests）
- 源码：各 crate 的 `tests/` 目录
