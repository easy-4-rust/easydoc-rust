# easydoc-core 数据模型与扩展 Trait 设计

- **日期**：2026-08-11
- **作者**：ZCode Agent（协同设计）
- **状态**：已部分实现，本文档为补全设计
- **依赖**：easydoc-core 现有 `document/`、`traits.rs`、`error.rs`、`types.rs`、`converter/`

## 1. 目标与范围

为 easydoc-rust 定义**唯一语义模型**与**扩展 trait 体系**，使所有下游 crate（reader、writer、template、markdown、mcp）统一消费 `easydoc-core` 提供的类型与接口，不再各自建立平行数据结构。

**核心需求**：

1. `DocumentContent` + `DocumentBlock` 作为读/写/渲染的唯一中间表示（IR）。
2. 所有 `DocumentBlock` 变体实现 `Clone + Debug + PartialEq`，支持序列化（serde 可选）。
3. `DocxRow` trait 统一表格行的序列化/反序列化，并通过 `derive` 宏自动生成。
4. `DocConverter<T>` trait 支持自定义值转换，`ConverterRegistry` 做运行时分发。
5. `DocReadListener<T>` / `DocWriteHandler` 为读/写提供生命周期钩子。
6. `EventSink` trait 为 SAX 流式读取提供统一回调接口。
7. 错误模型 `DocError` 覆盖 IO、ZIP、格式、模板、转换、不支持等全部场景。

**非目标**：

- 不提供运行时 schema 校验（如 JSON Schema 验证 `DocumentContent`）。
- 不引入泛型参数到 `DocumentContent`（保持零泛型、易序列化）。
- 不在 core 层引入任何 IO 或 ZIP 操作（这些由 ooxml/reader/writer 承担）。
- 不支持 .doc 二进制格式的写入（读取由 reader 通过 office_oxide 完成）。

## 2. 总体架构

```
┌─────────────────────────────────────────────────────────────┐
│                     easydoc (facade)                        │
│  EasyDoc::document() / read_text() / to_markdown() / ...    │
└──────────────────────┬──────────────────────────────────────┘
                       │ 消费
        ┌──────────────┼──────────────┬──────────────┐
        ▼              ▼              ▼              ▼
   easydoc-reader  easydoc-writer  easydoc-template  easydoc-markdown
        │              │              │              │
        └──────────────┴──────────────┴──────────────┘
                       │ 依赖
                       ▼
              ┌─────────────────┐
              │  easydoc-core   │
              │                 │
              │  DocumentContent│◄── 唯一 IR
              │  DocumentBlock  │
              │  DocxRow        │◄── 表格行 trait
              │  DocConverter   │◄── 值转换 trait
              │  EventSink      │◄── SAX 回调 trait
              │  DocWriteHandler│◄── 写入钩子
              │  DocReadListener│◄── 读取钩子
              │  DocError       │◄── 统一错误
              │  types / style  │◄── 基础类型
              └─────────────────┘
                       ▲
                       │ 仅类型依赖
              ┌─────────────────┐
              │ easydoc-derive  │
              │ #[derive(DocxRow)]
              └─────────────────┘
```

## 3. 模块职责划分

### 3.1 `document/` — 语义模型

| 类型 | 职责 | 当前状态 |
|---|---|---|
| `DocumentContent` | 顶层容器：metadata + blocks | `[已实现]` |
| `DocumentBlock` | 枚举：Heading/Paragraph/Table/List/Image/Footnote/Endnote/CodeBlock/ThematicBreak/PageBreak/ColumnBreak/TextBox/Section | `[部分实现]`，Section 待补全 |
| `DocumentTextRun` | 行内文本：text + bold/italic/strikethrough/hyperlink | `[已实现]` |
| `DocumentTable` / `DocumentTableRow` / `DocumentTableCell` | 表格三层结构 | `[已实现]` |
| `DocumentList` / `DocumentListItem` | 列表（有序/无序，支持嵌套） | `[已实现]` |
| `DocumentImage` | 图片：alt_text + binary data + extension | `[已实现]` |
| `DocumentMeta` | 元数据：title/author/subject/keywords/created | `[已实现]` |

**待补全**：

- `DocumentBlock::Section`：段落分节（`<w:sectPr>`），携带页面尺寸、边距、方向信息。
- `DocumentBlock::Equation`：OMML 公式节点（Phase 4 目标）。
- `DocumentBlock::Comment` / `DocumentBlock::Revision`：批注与修订（Phase 4 目标）。

### 3.2 `traits.rs` — 扩展 Trait

| Trait | 签名概要 | 职责 |
|---|---|---|
| `DocxRow` | `schema()`, `to_row()`, `from_row()`, `to_row_with_converters()`, `from_row_with_converters()` | 表格行的序列化/反序列化 |
| `DocConverter<T>` | `support_type()`, `to_doc_value()`, `from_doc_value()` | 自定义值转换 |
| `DocReadListener<T>` | `invoke(data, ctx)`, `on_complete(ctx)`, `on_error(err, ctx)`, `has_next(ctx)` | 读取生命周期钩子 |
| `DocWriteHandler` | `before_document/after_document`, `before_paragraph/after_paragraph`, `before_table/after_table`, `before_cell/after_cell` | 写入生命周期钩子 |
| `EventSink` | `on_event(&mut self, event: &DocumentEvent) -> Result<()>` | SAX 流式读取回调 |
| `DocumentReader` | `read_model(path) -> Result<DocumentContent>`, `read_events(path, sink) -> Result<()>` | 统一读取入口 |
| `DocumentRenderer` | `render(content: &DocumentContent) -> Result<Vec<u8>>` | 统一渲染入口（设计目标） |

### 3.3 `types.rs` / `style/` / `converter/` — 基础设施

| 模块 | 内容 |
|---|---|
| `types.rs` | `DocValue`、`CellData`、`RowData`、`HeadingLevel`、`HorizontalAlignment`、`FillDirection` 等 |
| `style/` | `Color`、`FontConfig`、`ParagraphStyle`、`TableStyle`、`AutoWidthStrategy` |
| `converter/` | `ConverterRegistry`（HashMap<TypeId, Box<dyn Any>>） |
| `error.rs` | `DocError`（7 个变体：Io/Zip/Format/Template/Conversion/Unsupported/Document） |
| `metadata/` | `TableColumn`、`DocumentMeta` |

## 4. 关键数据流

### 4.1 表格写入：DocxRow → OOXML

```
Vec<User>                          User: impl DocxRow / #[derive(DocxRow)]
    │
    ▼
DocxRow::to_row(&self)            → Vec<CellData>
    │
    ▼
TableWriteExecutor::execute()     → docx_rs::Table
    │                                (应用 TableColumn 的 width/format/align/wrap)
    ▼
DocWriteExecutor::execute()       → docx_rs::Docx
    │
    ▼
AtomicFile::write()               → output.docx
```

### 4.2 表格读取：OOXML → DocxRow

```
input.docx
    │
    ▼
office_oxide::Document            → IR (rows of cells)
    │
    ▼
extract_tables::<T: DocxRow>()    → RowData
    │
    ▼
DocxRow::from_row(&RowData)       → T
    │                                (ConverterRegistry 做值转换)
    ▼
Vec<Vec<T>>                       → 返回给调用方
```

### 4.3 语义模型往返

```
input.docx → read_document() → DocumentContent → [用户修改] → write_content() → output.docx
```

关键点：`DocumentContent` 不携带任何 ZIP 结构信息，只包含语义块。写入时由 `content_renderer` 重新构建 OOXML。

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | `DocumentBlock` 用 enum 而非 trait object | 模式匹配简洁、无堆分配、序列化友好 | 新增变体需修改 enum，破坏兼容性 |
| 2 | `DocxRow` trait 方法同时接受 `RowData` 和 `ConverterRegistry` | 支持有/无自定义转换两种场景 | trait 方法较多（5 个），derive 宏需生成全部 |
| 3 | `DocError` 用 thiserror 派生 | 减少样板代码 | 变体不可跨 crate 精细化捕获 |
| 4 | `DocumentContent` 不带泛型参数 | 简化序列化、传递、存储 | 无法在类型层面约束 block 类型 |
| 5 | `EventSink` 用 `&mut self` 回调 | 允许 sink 内部维护状态（计数、过滤） | 调用方需持有可变引用 |
| 6 | Writer 仍用自建 `Paragraph`/`Run`/`Table`（非 core model） | 历史原因，Phase 3 已通过 `content_renderer` 桥接 | 两套类型并存增加认知负担 |

### 5.1 Writer 统一到 core model 的路径

当前 Writer 有自建的 `Paragraph`、`Run`、`Table`、`DocImage` 类型，与 `DocumentBlock` / `DocumentTextRun` 平行。Phase 4 计划：

1. 让 `DocBuilder` 内部直接构建 `DocumentBlock`。
2. `DocWriteExecutor` 消费 `DocumentContent`（而非自建类型）。
3. 移除 `easydoc-writer/src/lib.rs` 中的重复类型定义。

这将使 `EasyDoc::load()` → 修改 → `EasyDoc::write_content()` 的闭环与 `EasyDoc::document()` → `.add_heading()` → `.save()` 的闭环共用同一套类型。

## 6. 测试与验收

### 6.1 现有测试覆盖

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_docx_row_derive_basic` | derive 生成的 `schema()`/`to_row()`/`from_row()` 正确 | `writer_test.rs` |
| `test_docx_row_derive_with_annotations` | `width`/`format`/`align`/`wrap`/`converter` 注解生效 | `writer_test.rs` |
| `test_template_scalar_fill` | 标量占位符替换 | `writer_test.rs` |
| `test_converter_registry` | `ConverterRegistry` 注册与分发 | `writer_test.rs` |
| `test_read_document_to_content` | `read_document()` 返回正确 `DocumentContent` | `writer_test.rs` |
| `test_write_content_roundtrip` | 写入 → 读取 → 验证 blocks 一致 | `writer_test.rs` |

### 6.2 待补充测试

- `DocumentBlock::Section` 的序列化/反序列化往返。
- `DocumentBlock::Equation` 的创建与渲染（Phase 4）。
- `DocReadListener` 的 `has_next()` 提前终止。
- `DocWriteHandler` 各钩子的调用顺序与参数正确性。
- `EventSink` 在 OOM 边界下的行为（大文档不停机）。

## 7. 引用

- 架构文档：`docs/easydoc-rust-Architecture.zh_CN.md` 第 6 节「easydoc-core 模型设计」
- 使用指南：`docs/usage-guide.md` 第 7 节「语义文档模型」、第 9 节「高级特性」
- Roadmap：`docs/roadmap.md` Phase 2（语义模型）、Phase 3（事件链）
- 源码：`crates/easydoc-core/src/document/`、`crates/easydoc-core/src/traits.rs`
