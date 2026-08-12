# 输入安全模型设计

- **日期**：2026-08-11
- **作者**：ZCode Agent（协同设计）
- **状态**：已部分实现，本文档为补全设计
- **依赖**：easydoc-ooxml（PackageLimits、PackageRewriter、AtomicFile）、easydoc-mcp（路径穿越防护）、easydoc-reader（格式检测）

## 1. 目标与范围

为 easydoc-rust 建立**纵深防御**的安全模型，覆盖文件输入的全部攻击面：ZIP bomb、路径遍历、XML 注入、资源耗尽、格式混淆等。

**核心需求**：

1. **ZIP 资源限制**：`PackageLimits` 限制条目数（10,000）、单条目大小（256MB）、总大小（1GB）、压缩比（1000:1）。
2. **原子输出**：`AtomicFile` 确保写入失败时原文件不被损坏。
3. **二进制保真**：`PackageRewriter` 对未修改条目逐字节保留。
4. **路径穿越防护**：MCP 资源读取时 canonicalize + starts_with 校验。
5. **XML 转义**：模板填充时对 `&`、`<`、`>`、`"`、`'` 进行转义。
6. **格式检测**：magic bytes 校验，防止伪装扩展名的攻击。
7. **跨 Run 占位符安全**：`replace_across_text_nodes()` 确保拆分占位符的完整替换。

**非目标**：

- 不提供 OOXML schema 校验（信任 docx-rs 输出）。
- 不做宏病毒检测（依赖操作系统/杀毒软件）。
- 不提供加密/DRM 支持。
- 不做网络层安全（MCP 仅 stdio，无 HTTP 暴露）。

## 2. 总体架构

```
┌───────────────────────────────────────────────────────────────────┐
│                        外部输入                                    │
│  文件路径 / 文件内容 / JSON 参数 / 用户数据                         │
└───────────────────────────┬───────────────────────────────────────┘
                            │
            ┌───────────────┼───────────────┐
            ▼               ▼               ▼
     ┌──────────┐    ┌──────────┐    ┌──────────┐
     │ 路径校验  │    │ 格式检测  │    │ 参数验证  │
     │ (MCP)    │    │ (reader) │    │ (tools)  │
     └────┬─────┘    └────┬─────┘    └────┬─────┘
          │               │               │
          └───────┬───────┴───────────────┘
                  ▼
     ┌──────────────────────────────────┐
     │       easydoc-ooxml              │
     │                                  │
     │  PackageLimits                   │
     │  ├─ max_entries: 10,000          │
     │  ├─ max_entry_size: 256MB        │
     │  ├─ max_total_size: 1GB          │
     │  └─ max_compression_ratio: 1000  │
     │                                  │
     │  PackageRewriter                 │
     │  ├─ ZIP 重写（仅修改目标 XML）    │
     │  └─ 未修改条目逐字节保留          │
     │                                  │
     │  AtomicFile                      │
     │  ├─ temp file → write → persist  │
     │  └─ 失败时原文件不变              │
     └──────────────────────────────────┘
                  │
                  ▼
     ┌──────────────────────────────────┐
     │       easydoc-template           │
     │                                  │
     │  XML 转义                        │
     │  ├─ & → &amp;                    │
     │  ├─ < → &lt;                     │
     │  ├─ > → &gt;                     │
     │  ├─ " → &quot;                   │
     │  └─ ' → &apos;                   │
     │                                  │
     │  跨 Run 占位符                   │
     │  └─ replace_across_text_nodes()  │
     └──────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 安全层分布

| 安全层 | 所在 crate | 组件 | 防护目标 |
|---|---|---|---|
| 路径校验 | easydoc-mcp | `safe_resolve_path()` | 路径穿越 |
| 格式检测 | easydoc-reader | `detect_format()` | 伪装扩展名 |
| ZIP 资源限制 | easydoc-ooxml | `PackageLimits` | ZIP bomb |
| ZIP 安全重写 | easydoc-ooxml | `PackageRewriter` | 二进制篡改 |
| 原子输出 | easydoc-ooxml | `AtomicFile` | 写入失败损坏 |
| XML 转义 | easydoc-template | `escape_xml_text()` | XML 注入 |
| 占位符安全 | easydoc-template | `replace_across_text_nodes()` | 拆分占位符不完整替换 |
| 参数验证 | easydoc-mcp | 各 tool handler | 缺失/非法参数 |

### 3.2 `PackageLimits` 默认值

```rust
pub struct PackageLimits {
    pub max_entries: usize,              // 默认 10,000
    pub max_entry_decompressed: u64,     // 默认 256 MB
    pub max_total_decompressed: u64,     // 默认 1 GB
    pub max_compression_ratio: f64,      // 默认 1,000:1
}
```

### 3.3 `AtomicFile` 写入流程

```
AtomicFile::create(target_path)
    │
    ├── 1. 在 target_path 同目录创建临时文件
    │      (tempfile::NamedTempFile::new_in(dir))
    │
    ├── 2. write_all(content)
    │
    ├── 3. flush()
    │
    ├── 4. sync_all()  ← 确保数据落盘
    │
    └── 5. persist(target_path)  ← 原子 rename
         │
         ├── 成功 → target 被替换
         └── 失败 → 临时文件被删除，target 不变
```

## 4. 关键数据流

### 4.1 ZIP 文件打开安全检查

```
input.docx
    │
    ▼
std::fs::File::open(path)
    │
    ▼
zip::ZipArchive::new(file)
    │
    ├── 检查 1: archive.len() <= max_entries (10,000)
    │   └── 超限 → return Err(DocError::Format("too many entries"))
    │
    ├── 遍历每个条目：
    │   │
    │   ├── 检查 2: entry.size() <= max_entry_decompressed (256MB)
    │   │   └── 超限 → return Err(DocError::Format("entry too large"))
    │   │
    │   ├── 检查 3: 累计 total_size <= max_total_decompressed (1GB)
    │   │   └── 超限 → return Err(DocError::Format("total size exceeded"))
    │   │
    │   └── 检查 4: compression_ratio <= max_compression_ratio (1000:1)
    │       └── 超限 → return Err(DocError::Format("suspicious compression ratio"))
    │
    ▼
ZIP 解析成功
```

### 4.2 XML 转义流程

```
用户数据: "Tom & Jerry <company>"
    │
    ▼
escape_xml_text(input)
    │
    ├── '&' → '&amp;'
    ├── '<' → '&lt;'
    ├── '>' → '&gt;'
    ├── '"' → '&quot;'
    └── ''' → '&apos;'
    │
    ▼
输出: "Tom &amp; Jerry &lt;company&gt;"
    │
    ▼
嵌入 XML: <w:t>Tom &amp; Jerry &lt;company&gt;</w:t>
```

### 4.3 跨 Run 占位符替换

```
模板 DOCX 中的 XML：
<w:r><w:t>Dear {na</w:t></w:r><w:r><w:t>me},</w:t></w:r>

问题：{name} 被拆分到两个 <w:r> 节点中

replace_across_text_nodes() 处理：
    │
    ├── 1. 扫描所有 <w:t> 节点，拼接文本
    │      "Dear {na" + "me}," = "Dear {name},"
    │
    ├── 2. 在拼接文本中找到 {name} 占位符
    │
    ├── 3. 替换为 "Alice"
    │      "Dear Alice,"
    │
    └── 4. 将结果重新分配回 <w:t> 节点
           <w:r><w:t>Dear Ali</w:t></w:r><w:r><w:t>ce,</w:t></w:r>
```

### 4.4 MCP 路径穿越防护

```
URI: "file:///tmp/../../../etc/passwd"
    │
    ▼
strip_prefix("file://") → "/tmp/../../../etc/passwd"
    │
    ▼
PathBuf::from(path).canonicalize() → "/etc/passwd"
    │
    ▼
root.canonicalize() → "/Users/wandl/docs"
    │
    ▼
"/etc/passwd".starts_with("/Users/wandl/docs") → false
    │
    ▼
返回 None（拒绝访问）
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | PackageLimits 用默认值而非可配置 | 安全优先，避免用户误配 | 真实大文档可能需要调高限制 |
| 2 | AtomicFile 在同目录创建临时文件 | 确保 rename 是原子操作 | 需要目标目录可写 |
| 3 | XML 转义覆盖全部 5 个特殊字符 | 符合 XML 规范 | 转义后文本变长 |
| 4 | 路径穿越用 canonicalize | 处理符号链接和 `..` | canonicalize 要求路径存在 |
| 5 | 格式检测同时检查扩展名和 magic bytes | 双重校验更可靠 | 可能误判（如 .docx 改名为 .zip） |
| 6 | MCP 仅支持 stdio | 最简单的安全模型 | 无法远程访问 |

### 5.1 已知安全边界

1. **OOXML 内容不校验**：信任 docx-rs 和 office_oxide 的输出，不检查 XML 结构合法性。
2. **内存不隔离**：大文档可能导致 OOM，但 Rust 的内存安全保证不会出现 buffer overflow。
3. **临时文件泄露**：`AtomicFile` 在写入过程中创建临时文件，如果进程被 kill -9，临时文件可能残留。
4. **符号链接攻击**：`canonicalize()` 会解析符号链接，但如果目标是符号链接指向根目录外的文件，`starts_with` 会拒绝。

## 6. 测试与验收

### 6.1 现有测试

| 测试 | 断言点 | 文件 |
|---|---|---|
| `rejects_packages_over_entry_limit` | 条目数超限返回 Format 错误 | `package_rewriter_test.rs` |
| `rejects_packages_over_size_limit` | 总大小超限返回 Format 错误 | `package_rewriter_test.rs` |
| `rejects_packages_over_compression_ratio` | 压缩比超限返回 Format 错误 | `package_rewriter_test.rs` |
| `preserves_binary_entries_byte_for_byte` | 未修改条目逐字节保留 | `package_rewriter_test.rs` |
| `keeps_existing_target_when_transform_fails` | 写入失败时原文件不变 | `package_rewriter_test.rs` |
| `binary_fidelity_test` | 图片字节不变 + XML 转义正确 | `binary_fidelity_test.rs` |
| `test_escape_xml_text` | 5 个特殊字符正确转义 | `writer_test.rs` |
| `test_cross_run_placeholder` | 拆分占位符正确替换 | `binary_fidelity_test.rs` |
| `test_path_traversal_blocked` | MCP 路径穿越被拒绝 | `server_test.rs` |
| `test_detect_format_docx` | DOCX magic bytes 正确识别 | `writer_test.rs` |
| `test_detect_format_doc` | DOC magic bytes 正确识别 | `writer_test.rs` |

### 6.2 待补充测试

- **ZIP bomb 测试**：构造压缩比 10,000:1 的恶意 ZIP 文件。
- **超大条目测试**：单条目 257MB 的 ZIP 文件。
- **空 ZIP 文件**：0 条目的 ZIP 文件处理。
- **损坏 ZIP 文件**：截断的 ZIP 文件的错误处理。
- **XML 注入测试**：模板占位符包含 XML 标签（如 `<script>`）的转义。
- **Unicode 路径测试**：中文/日文路径的 canonicalize 正确性。
- **符号链接测试**：符号链接指向根目录外的拒绝。
- **临时文件清理**：写入失败后临时文件被正确删除。

## 7. 引用

- 架构文档：`docs/easydoc-rust-Architecture.zh_CN.md` 第 4 节「安全与资源约束」
- 使用指南：`docs/usage-guide.md` 第 5.1 节（模板填充）、第 10 节（错误处理）
- Roadmap：`docs/roadmap.md` Phase 1（基础安全）
- 源码：`crates/easydoc-ooxml/src/`、`crates/easydoc-mcp/src/resources.rs`、`crates/easydoc-template/src/fill_executor.rs`
