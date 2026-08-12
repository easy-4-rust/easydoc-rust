# SAX 流式 DOCX 读取架构设计

- **日期**：2026-08-11
- **作者**：ZCode Agent（协同设计）
- **状态**：已部分实现，本文档为补全设计
- **依赖**：easydoc-reader（`extractor/`）、easydoc-core（`EventSink`、`DocumentEvent`、`DocumentReader`）、office_oxide

## 1. 目标与范围

为 easydoc-rust 提供**O(1) 内存**的 SAX 流式文档读取能力，使用户能以事件驱动方式处理超大 DOCX 文件，而无需将整个文档加载到内存。

**核心需求**：

1. `EventSink` trait 接收逐事件回调：Heading、Paragraph、Table、Image、Formula、List、PageBreak 等。
2. `DocumentEvent` 枚举覆盖所有已支持的 DOCX 内容类型。
3. `DocumentReader` trait 统一 `read_model()`（返回 `DocumentContent`）和 `read_events()`（SAX 流式）两种读取模式。
4. SAX 路径下内存消耗为 O(1)（不累积整个文档的 blocks）。
5. 覆盖 OMML 公式（`<m:oMath>` / `<m:oMathPara>`）、嵌套表格、合并单元格（gridSpan + vMerge）、超链接（rId 解析）、图片（word/media/* 二进制提取）。
6. `ViewMode`（Plain / Annotated / Outline / Stats）基于 `DocumentContent` 渲染，而非独立解析路径。

**非目标**：

- 不提供异步流式读取（当前为同步阻塞模型）。
- 不支持增量读取（如"只读第 N 页"）。
- 不在 SAX 路径下保留格式信息（如字体、颜色）——这些仅在 `read_model()` 路径下可用。
- 不解析 OLE2（.doc）的 SAX 流——.doc 仅支持 `read_model()` 和 `read_text()`。

## 2. 总体架构

```
┌─────────────────────────────────────────────────────────┐
│                    easydoc (facade)                      │
│  EasyDoc::read_events("large.docx", &mut MySink)?       │
│  EasyDoc::view_as("doc.docx", &ViewMode::Annotated)?   │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                 easydoc-reader                           │
│                                                         │
│  DocReadBuilder ──► DocumentReader::read_events()       │
│                            │                            │
│                   ┌────────┴────────┐                   │
│                   ▼                 ▼                   │
│            SAX 解析路径       Model 构建路径             │
│            (O(1) 内存)        (完整 DocumentContent)     │
│                   │                 │                   │
│                   ▼                 ▼                   │
│            EventSink          DocumentContent           │
│            on_event()         blocks: Vec<...>          │
└────────────────┬───────────────────┬────────────────────┘
                 │                   │
                 ▼                   ▼
┌─────────────────────────────────────────────────────────┐
│                     easydoc-core                         │
│  DocumentEvent    EventSink trait    DocumentContent     │
│  DocumentReader   ViewMode           DocumentBlock       │
└─────────────────────────────────────────────────────────┘
                 ▲
                 │
┌─────────────────────────────────────────────────────────┐
│                    office_oxide                          │
│  OLE2/ZIP 解析 → IR → 事件流                            │
└─────────────────────────────────────────────────────────┘
```

## 3. 模块职责划分

### 3.1 `easydoc-core` — 事件定义与 Trait

| 类型 | 职责 |
|---|---|
| `DocumentEvent` | 枚举：Heading / Paragraph / Table / Image / Formula / List / PageBreak / ThematicBreak / ColumnBreak / SectionBreak |
| `EventSink` | trait：`on_event(&mut self, event: &DocumentEvent) -> Result<()>` |
| `DocumentReader` | trait：`read_model()` + `read_events()` |
| `ViewMode` | 枚举：Plain / Annotated / Outline / Stats |

### 3.2 `easydoc-reader` — 解析实现

| 模块 | 职责 |
|---|---|
| `extractor/semantic.rs` | `extract_document()` → `DocumentContent`（Model 路径） |
| `extractor/sax.rs`（待建） | SAX 流式解析器：遍历 office_oxide IR，逐节点发射 `DocumentEvent` |
| `builder/read_builder.rs` | `DocReadBuilder`：fluent 配置入口 |
| `listener/collect.rs` | `CollectListener<T>`：内置收集型 listener |

### 3.3 SAX 解析器内部状态机

```
        ┌───────────┐
        │  Idle     │◄──────────────────────────────┐
        └─────┬─────┘                               │
              │ 开始解析 document.xml               │
              ▼                                     │
        ┌───────────┐                               │
        │ InBody    │◄────────┐                     │
        └─────┬─────┘        │                     │
              │               │                     │
    ┌─────────┼─────────┐    │                     │
    ▼         ▼         ▼    │                     │
 Heading  Paragraph   Table  │                     │
    │         │         │    │                     │
    │         │         ├──► InTable               │
    │         │         │    │  ├─ InRow            │
    │         │         │    │  │  └─ InCell        │
    │         │         │    │  └─ (嵌套 InTable)   │
    │         │         │    │                     │
    ▼         ▼         ▼    │                     │
 on_event  on_event  on_event │                     │
    │         │         │    │                     │
    └─────────┴─────────┴────┘                     │
              │                                     │
              │ 遇到 </w:body>                      │
              └─────────────────────────────────────┘
```

每个状态转换时，收集到的文本 runs / table rows 等组装为对应的 `DocumentEvent`，通过 `EventSink::on_event()` 发射给调用方，然后**丢弃**（不累积），从而保持 O(1) 内存。

## 4. 关键数据流

### 4.1 SAX 流式读取

```
input.docx
    │
    ▼
office_oxide::Document::open()
    │
    ▼
遍历 word/document.xml 节点流
    │
    ├── <w:p> + <w:pPr><w:pStyle w:val="Heading1"/>
    │   └── 组装 DocumentEvent::Heading { level: 1, runs: [...] }
    │       └── sink.on_event(&event)  ← 发射后丢弃 runs
    │
    ├── <w:tbl>
    │   └── 遍历 <w:tr> / <w:tc>
    │       └── 处理 gridSpan / vMerge / 嵌套表格
    │           └── 组装 DocumentEvent::Table { rows: [...] }
    │               └── sink.on_event(&event)
    │
    ├── <w:drawing> / <w:pict>
    │   └── 解析 rId → word/media/image1.png
    │       └── 提取二进制数据
    │           └── 组装 DocumentEvent::Image { data, alt_text, extension }
    │               └── sink.on_event(&event)
    │
    ├── <m:oMath> / <m:oMathPara>
    │   └── 提取 XML 片段
    │       └── 组装 DocumentEvent::Formula { xml }
    │           └── sink.on_event(&event)
    │
    └── <w:numPr>
        └── 解析 numbering.xml 关联
            └── 组装 DocumentEvent::List { ordered, items }
                └── sink.on_event(&event)
```

### 4.2 ViewMode 渲染

```
input.docx
    │
    ▼
read_document() → DocumentContent     (Model 路径，完整加载)
    │
    ▼
ViewMode::Annotated 渲染：
    for block in content.blocks:
        match block {
            Heading  → "[Heading{level}] {text}\n"
            Paragraph → "[Paragraph {n}] {text}\n"
            Table    → "[Table {n}: {rows}x{cols}] ...\n"
            Image    → "[Image {n}: {alt}, {size}B]\n"
            ...
        }
    │
    ▼
String (结构化标注文本)
```

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | SAX 与 Model 共用 office_oxide IR | 避免两套解析器维护 | office_oxide IR 不暴露 OMML 原始 XML 时需额外提取 |
| 2 | `EventSink` 用 `&mut self` | 允许 sink 维护状态（计数、过滤、聚合） | 调用方需持有可变引用 |
| 3 | SAX 路径不保留格式信息 | O(1) 内存需要丢弃中间数据 | 用户如需格式信息只能走 Model 路径 |
| 4 | `ViewMode` 基于 `DocumentContent` 而非独立解析 | 复用 Model 构建路径，减少维护成本 | 无法处理超大文件的 ViewMode（需完整加载） |
| 5 | 合并单元格降级为 HTML table + warning | GFM Markdown 不支持合并单元格 | 用户收到 warning 需自行处理 |
| 6 | 图片二进制数据在 SAX 路径下按需提取 | 避免大文档内存峰值 | 需要 rId → media 路径的映射表 |

### 5.1 未决问题

1. **SAX 路径下的超大表格**：如果单个表格有 10 万行，`DocumentEvent::Table` 会一次性携带所有行。是否需要拆分为 `TableStart` / `TableRow` / `TableEnd` 三个事件？当前设计选择一次性发射，因为表格在 DOCX 中是天然分块的。
2. **OMML 公式 XML 的提取**：office_oxide IR 暂不暴露 `<m:oMath>` 原始 XML，需在 reader 层做额外遍历。Phase 4 计划与 office_oxide 上游协调。

## 6. 测试与验收

### 6.1 现有测试

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_read_events_basic` | Heading/Paragraph/Table/Image 事件正确发射 | `writer_test.rs` |
| `test_read_events_formula` | OMML 公式事件正确发射 | `writer_test.rs` |
| `test_read_events_list` | 有序/无序列表事件正确 | `writer_test.rs` |
| `test_read_events_merged_cells` | 合并单元格事件正确 | `writer_test.rs` |
| `test_view_mode_annotated` | Annotated 输出包含结构标记 | `writer_test.rs` |
| `test_view_mode_outline` | Outline 输出仅含标题 | `writer_test.rs` |

### 6.2 待补充测试

- **内存压力测试**：100MB DOCX 文件的 SAX 读取，内存峰值不超过 50MB。
- **事件顺序测试**：Heading → Paragraph → Table 的顺序与文档顺序一致。
- **嵌套表格测试**：表格内嵌表格的事件正确嵌套。
- **提前终止测试**：`EventSink` 在第 N 个事件后返回错误，解析器正确停止。
- **ViewMode::Stats 精度**：Paragraphs / Tables / Images / Words 计数与手动计数一致。

## 7. 迁移路径

当前 `read_text()` 和 `read_tables()` 直接使用 office_oxide IR，不经过 `DocumentContent`。迁移计划：

1. Phase 4：让 `read_text()` 内部走 `read_document()` → 遍历 blocks → 拼接文本。
2. 保持 `read_tables()` 的 `DocxRow` 反序列化路径不变（性能敏感）。
3. 最终移除 `extractor/text.rs` 中的直接 office_oxide 调用。

## 8. 引用

- 架构文档：`docs/easydoc-rust-Architecture.zh_CN.md` 第 5.2 节「读取流程」、第 10 节「easydoc-reader 设计」
- 使用指南：`docs/usage-guide.md` 第 4.3 节「SAX 流式读取」、第 4.4 节「ViewMode 渲染」
- Roadmap：`docs/roadmap.md` Phase 3（事件链与高级读取）
- 源码：`crates/easydoc-reader/src/extractor/`、`crates/easydoc-core/src/traits.rs`
