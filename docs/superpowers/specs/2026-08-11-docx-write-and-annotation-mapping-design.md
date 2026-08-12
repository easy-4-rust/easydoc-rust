# DOCX 写入与注解映射设计

- **日期**：2026-08-11
- **作者**：ZCode Agent（协同设计）
- **状态**：已部分实现，本文档为补全设计
- **依赖**：easydoc-writer（builder/executor/handler/style）、easydoc-core（DocxRow、DocumentContent）、easydoc-derive（#[derive(DocxRow)]）、easydoc-ooxml（AtomicFile）、docx-rs

## 1. 目标与范围

为 easydoc-rust 提供完整的 DOCX 写入链路：从 Rust 结构体（通过 `#[derive(DocxRow)]` 注解）到 OOXML 输出，覆盖文档构建、表格写入、原位编辑、生命周期钩子等全部写入场景。

**核心需求**：

1. `DocBuilder` fluent API 支持 heading / paragraph / table / image / pagebreak 的自由组合。
2. `TableWriteBuilder<T: DocxRow>` 支持从 `Vec<T>` 一键生成带样式表格。
3. `#[derive(DocxRow)]` 宏支持全部注解：`name`、`order`、`width`、`format`、`align`、`wrap`、`converter`、`ignore`。
4. 注解到 OOXML 的完整映射：width → `<w:tcW>`、format → `<w:numFmt>`、align → `<w:jc>`、wrap → `<w:noWrap>`。
5. `DocWriteHandler` 提供 document / paragraph / table / cell 四级钩子。
6. 所有写入通过 `AtomicFile` 实现原子输出（失败不损坏原文件）。
7. `content_renderer` 桥接 `DocumentContent` → docx-rs，使语义模型闭环可写。

**非目标**：

- 不支持 .doc 二进制格式写入。
- 不提供 DOCX 样式模板继承（如从模板 .docx 继承样式定义）。
- 不支持协作编辑、修订追踪的写入（Phase 4 设计目标）。
- 不做 OOXML schema 校验（信任 docx-rs 输出）。

## 2. 总体架构

```
┌──────────────────────────────────────────────────────────────────┐
│                        easydoc (facade)                          │
│  EasyDoc::document("out.docx").add_heading(...).save()          │
│  EasyDoc::write_table("out.docx", &users).do_write()            │
│  EasyDoc::write_content(&content, "out.docx")                   │
└────────────────────────┬─────────────────────────────────────────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
   DocBuilder      TableWriteBuilder   write_content()
   (混合文档)       (纯表格)           (语义模型)
         │               │               │
         ▼               ▼               ▼
  DocWriteExecutor  TableWriteExecutor  content_renderer
         │               │               │
         └───────┬───────┴───────────────┘
                 ▼
           docx_rs::Docx
                 │
                 ▼
         docx.build().pack()
                 │
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│                     easydoc-ooxml                                │
│  AtomicFile::write(bytes)  →  temp file + persist               │
└──────────────────────────────────────────────────────────────────┘
                 │
                 ▼
            output.docx
```

## 3. 模块职责划分

### 3.1 `easydoc-writer/src/` 结构

```
easydoc-writer/src/
├── lib.rs                      公开类型：Paragraph, Run, Table, DocImage
├── builder/
│   ├── doc_builder.rs          DocBuilder — 混合内容文档构建器
│   └── table_builder.rs        TableWriteBuilder<T> — 纯表格构建器
├── doc_editor.rs               DocEditor — 原位编辑（replace_text + save）
├── executor/
│   ├── write_executor.rs       DocWriteExecutor — DocBuilder → docx_rs::Docx
│   └── table_executor.rs       TableWriteExecutor<T> — Vec<T> → docx_rs::Table
├── handler/
│   └── mod.rs                  DocWriteHandler trait — 四级钩子
└── style/
    ├── auto_width.rs           AutoWidthStrategy
    └── banded_rows.rs          BandedRowsStrategy
```

### 3.2 各组件职责

| 组件 | 输入 | 输出 | 职责 |
|---|---|---|---|
| `DocBuilder` | heading/paragraph/table/image/pagebreak | `DocWriteExecutor` | 收集文档元素，fluent API |
| `TableWriteBuilder<T>` | `Vec<T>` + 样式配置 | `TableWriteExecutor<T>` | 表格专用构建器 |
| `DocWriteExecutor` | DocBuilder 内容 | `docx_rs::Docx` → bytes | 转换为 OOXML |
| `TableWriteExecutor<T>` | `Vec<T>` + TableColumn 元数据 | `docx_rs::Table` | 行/列转换 + 注解映射 |
| `DocEditor` | 已有 DOCX 文件 | 修改后的 DOCX | 基于 `PackageRewriter` 的原位编辑 |
| `content_renderer` | `DocumentContent` | `docx_rs::Docx` | 语义模型 → OOXML |
| `DocWriteHandler` | 各级写入事件 | 回调副作用 | 日志、审计、数据验证 |

### 3.3 `#[derive(DocxRow)]` 宏展开

输入：

```rust
#[derive(DocxRow)]
#[docx(banded_rows = true)]
struct User {
    #[docx(name = "Name", order = 0, width = "35%")]
    name: String,
    #[docx(name = "Age", order = 1, format = "#,##0", align = "right")]
    age: u32,
    #[docx(ignore)]
    secret: String,
}
```

生成代码：

```rust
impl DocxRow for User {
    fn schema() -> &'static [TableColumn] {
        static S: LazyLock<Vec<TableColumn>> = LazyLock::new(|| vec![
            TableColumn::new("Name", "name", 0).order(0).width_pct(35),
            TableColumn::new("Age", "age", 1).order(1).format("#,##0").align("right"),
        ]);
        &*S
    }

    fn to_row(&self) -> Result<Vec<CellData>> {
        Ok(vec![
            CellData::new(self.name.clone()),
            CellData::new(self.age.to_string()),
            // secret 被 ignore，不生成
        ])
    }

    fn from_row(row: &RowData) -> Result<Self> {
        Ok(User {
            name: row.cells[0].to_string(),
            age: row.cells[1].parse()?,
            secret: String::new(),
        })
    }

    fn to_row_with_converters(&self, reg: &ConverterRegistry) -> Result<Vec<CellData>> {
        // 使用 reg 做值转换
    }

    fn from_row_with_converters(row: &RowData, reg: &ConverterRegistry) -> Result<Self> {
        // 使用 reg 做值转换
    }
}
```

## 4. 关键数据流

### 4.1 注解 → OOXML 映射

| 注解 | Rust 类型 | OOXML 元素 | 示例 |
|---|---|---|---|
| `width = "2cm"` | 字符串解析 | `<w:tcW w:w="1134" w:type="dxa"/>` | 1cm = 567 twips |
| `width = "80px"` | 字符串解析 | `<w:tcW w:w="1200" w:type="dxa"/>` | 1px = 15 twips (96dpi) |
| `width = "50%"` | 字符串解析 | `<w:tcW w:w="5000" w:type="pct"/>` | 50% = 5000 fiftieths |
| `width = "auto"` | 枚举 | `<w:tcW w:w="0" w:type="auto"/>` | 自动宽度 |
| `format = "#,##0.00"` | 字符串 | `<w:numFmt w:val="decimal"/>` + 自定义格式 | 数字格式 |
| `format = "yyyy-mm-dd"` | 字符串 | `<w:numFmt w:val="dateTime"/>` | 日期格式 |
| `align = "right"` | 字符串 | `<w:jc w:val="right"/>` | 右对齐 |
| `align = "center"` | 字符串 | `<w:jc w:val="center"/>` | 居中 |
| `align = "both"` | 字符串 | `<w:jc w:val="both"/>` | 两端对齐 |
| `wrap = true` | 布尔 | 不生成 `<w:noWrap/>` | 允许换行 |
| `wrap = false` | 布尔 | `<w:noWrap/>` | 禁止换行 |
| `converter = MyConv` | 类型路径 | `ConverterRegistry` 运行时分发 | 自定义转换 |
| `ignore` | 标志 | 跳过该字段 | 不参与读写 |

### 4.2 DocWriteHandler 钩子调用顺序

```
before_document(ctx)           ← 文档开始
    │
    ├── before_paragraph(ctx)  ← 段落 1
    │   └── after_paragraph(ctx)
    │
    ├── before_table(ctx)      ← 表格 1
    │   ├── before_cell(ctx)   ← 单元格 [0,0]
    │   │   └── after_cell(ctx)
    │   ├── before_cell(ctx)   ← 单元格 [0,1]
    │   │   └── after_cell(ctx)
    │   └── after_table(ctx)
    │
    └── after_document(ctx)    ← 文档结束
```

### 4.3 原子写入流程

```
DocWriteExecutor::execute()
    │
    ▼
docx_rs::Docx::build()
    │
    ▼
docx_rs::Docx::pack() → Vec<u8>
    │
    ▼
AtomicFile::create(target_path)
    │
    ├── write_all(bytes)
    ├── flush()
    ├── sync_all()
    └── persist()  ← 原子替换
         │
         ├── 成功 → target 被替换
         └── 失败 → target 保持不变
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | Writer 用 docx-rs 作为后端 | 成熟的 OOXML 生成库，支持样式/图片/表格 | 依赖外部库，上游 bug 需等待修复 |
| 2 | `DocBuilder` 与 `TableWriteBuilder` 分离 | 纯表格场景更简洁，避免混合构建的复杂度 | 两套 API 增加学习成本 |
| 3 | `content_renderer` 桥接 core model | 使 `load() → modify → write_content()` 闭环可行 | 桥接层有额外的类型转换开销 |
| 4 | `AtomicFile` 在同一目录创建临时文件 | 确保 rename 是原子操作（同一文件系统） | 需要目标目录可写 |
| 5 | width 注解用字符串而非结构化类型 | 用户体验更自然（"2cm" vs `Width::Cm(2.0)`） | 字符串解析需处理各种格式和边界 |
| 6 | derive 宏同时生成 `to_row` 和 `from_row` | 对称性：同一 struct 可读可写 | 宏代码量大，调试困难 |

### 5.1 Writer 统一到 core model 的迁移计划

当前 Writer 有自建的 `Paragraph`、`Run`、`Table`、`DocImage` 类型。Phase 4 计划：

```
当前：
  DocBuilder → 自建 Paragraph/Run/Table → DocWriteExecutor → docx_rs

目标：
  DocBuilder → DocumentBlock (core model) → content_renderer → docx_rs
```

具体步骤：
1. 让 `DocBuilder::add_paragraph()` 内部构建 `DocumentBlock::Paragraph(Vec<DocumentTextRun>)`。
2. `DocWriteExecutor` 改为消费 `DocumentContent`。
3. 移除 `easydoc-writer/src/lib.rs` 中的 `Paragraph`、`Run`、`Table`、`DocImage` 类型。
4. `EasyDoc::document()` API 保持不变（fluent builder 语法不变）。

## 6. 测试与验收

### 6.1 现有测试

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_write_table_basic` | `Vec<User>` → 有效 DOCX ZIP | `writer_test.rs` |
| `test_write_document_mixed` | heading + paragraph + table + image | `writer_test.rs` |
| `test_roundtrip_write_read` | write → read_text 内容一致 | `writer_test.rs` |
| `test_table_roundtrip` | write_table → read_tables 数据一致 | `writer_test.rs` |
| `test_derive_annotations_width` | width 注解生成正确 `<w:tcW>` | `writer_test.rs` |
| `test_derive_annotations_format` | format 注解生成正确 `<w:numFmt>` | `writer_test.rs` |
| `test_write_handler_hooks` | 钩子按正确顺序调用 | `writer_test.rs` |
| `test_atomic_write_failure` | 写入失败时原文件不变 | `package_rewriter_test.rs` |
| `test_write_content_roundtrip` | DocumentContent → write → read → 验证 | `writer_test.rs` |

### 6.2 待补充测试

- `width` 注解全部格式（cm / px / pct / auto / pt）的 OOXML 输出验证。
- `DocEditor::replace_text()` 跨 `<w:r>` 节点的替换正确性。
- `content_renderer` 对 `DocumentBlock::List` / `DocumentBlock::Footnote` 的渲染。
- `TableWriteBuilder` 的 `banded_rows` / `header_style` / `auto_width` 组合。
- Writer 统一到 core model 后的回归测试。

## 7. 引用

- 架构文档：`docs/easydoc-rust-Architecture.zh_CN.md` 第 5.1 节「写入流程」、第 9 节「easydoc-writer 设计」
- 使用指南：`docs/usage-guide.md` 第 3 节「写入文档」、第 9.1 节「derive 宏」、第 9.3 节「写入钩子」
- Roadmap：`docs/roadmap.md` Phase 1（基础写入）、Phase 3（事件链与 Handler）
- 源码：`crates/easydoc-writer/src/`、`crates/easydoc-derive/src/`
