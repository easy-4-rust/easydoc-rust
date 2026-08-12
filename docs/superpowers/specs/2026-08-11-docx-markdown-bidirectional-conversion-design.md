# DOCX ↔ Markdown 双向转换设计

- **日期**：2026-08-11
- **作者**：ZCode Agent（协同设计）
- **状态**：DOCX → Markdown 已实现，Markdown → DOCX 为设计目标
- **依赖**：easydoc-markdown、easydoc-core（DocumentContent）、easydoc-reader（read_document）、easydoc-ooxml（AtomicFile）

## 1. 目标与范围

为 easydoc-rust 提供 DOCX 与 Markdown 之间的双向转换能力，覆盖文档格式化元素、表格、列表、图片、公式等全部内容类型。

**核心需求**：

1. **DOCX → Markdown**（`[已实现]`）：消费 `DocumentContent`，输出 GFM Markdown。
2. **Markdown → DOCX**（`[设计目标]`）：解析 Markdown AST，构建 `DocumentContent`，写入 DOCX。
3. **OMML → LaTeX**（`[设计目标]`）：将 OOXML 数学公式转换为 LaTeX 表示。
4. **Source Map**（`[设计目标]`）：Markdown 行号 ↔ DOCX 原文位置的双向映射。
5. 图片提取与引用：从 DOCX 的 `word/media/*` 提取图片到指定目录，Markdown 中用相对路径引用。
6. 降级策略：不支持的元素（如合并单元格）降级为 HTML 并输出 warning。
7. YAML front matter：可选提取 title / author / subject / keywords。

**非目标**：

- 不支持 Markdown → DOCX 的像素级还原（Markdown 本身是轻量标记语言）。
- 不做 DOCX → PDF 的转换。
- 不支持自定义 Markdown 方言（仅支持 CommonMark + GFM）。
- 不提供 DOCX 样式到 Markdown CSS 的映射。

## 2. 总体架构

```
                    ┌──────────────────────┐
                    │   DocumentContent    │
                    │   (easydoc-core)     │
                    └──────────┬───────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                                  ▼
   DOCX → Markdown                        Markdown → DOCX
   (已实现)                               (设计目标)
              │                                  │
              ▼                                  ▼
┌──────────────────────────┐      ┌──────────────────────────┐
│    easydoc-markdown      │      │    easydoc-markdown      │
│                          │      │    (新增 parser/)        │
│  MarkdownRenderer        │      │                          │
│    ├─ render_heading()   │      │  MarkdownParser          │
│    ├─ render_paragraph() │      │    ├─ parse_heading()    │
│    ├─ render_table()     │      │    ├─ parse_paragraph()  │
│    ├─ render_list()      │      │    ├─ parse_table()      │
│    ├─ render_image()     │      │    ├─ parse_list()       │
│    ├─ render_footnote()  │      │    ├─ parse_code_block() │
│    └─ render_code_block()│      │    └─ parse_inline()     │
│                          │      │                          │
│  MarkdownBuilder         │      │  MarkdownToDocxBuilder   │
│  MarkdownOptions         │      │                          │
│  MarkdownResult          │      └──────────────────────────┘
│  ConversionWarning       │                   │
│  ExtractedAsset          │                   ▼
└──────────────────────────┘       ┌──────────────────────────┐
              │                    │  DocumentContent         │
              ▼                    │  → write_content()       │
      output.md + assets/         └──────────────────────────┘
                                           │
                                           ▼
                                      output.docx
```

## 3. 模块职责划分

### 3.1 当前模块结构

```
easydoc-markdown/src/
├── lib.rs                      render_document() 入口
├── markdown_builder.rs         MarkdownBuilder — fluent 配置
├── markdown_options.rs         MarkdownOptions { image_directory, image_reference_prefix, include_front_matter }
├── markdown_renderer.rs        MarkdownRenderer — DocumentContent → Markdown 文本
├── markdown_result.rs          MarkdownResult { markdown, assets, warnings }
├── conversion_warning.rs       ConversionWarning — 降级警告
└── extracted_asset.rs          ExtractedAsset — 提取的图片资源
```

### 3.2 待新增模块（Phase 4）

```
easydoc-markdown/src/
├── parser/                     [新增] Markdown → DocumentContent
│   ├── mod.rs                  MarkdownParser 入口
│   ├── block.rs                块级元素解析（heading/paragraph/table/list/code_block）
│   ├── inline.rs               行内元素解析（bold/italic/link/code/image）
│   └── table.rs                GFM 表格解析
├── omml/                       [新增] OMML → LaTeX
│   ├── mod.rs                  转换入口
│   ├── node.rs                 OMML 节点类型定义
│   └── latex.rs                LaTeX 输出
└── source_map.rs               [新增] Markdown ↔ 源位置映射
```

### 3.3 各组件职责

| 组件 | 方向 | 职责 |
|---|---|---|
| `MarkdownRenderer` | DOCX → MD | 遍历 `DocumentContent` blocks，输出 GFM 文本 |
| `MarkdownBuilder` | DOCX → MD | fluent 配置（图片目录、前缀、front matter） |
| `MarkdownParser`（待建） | MD → DOCX | 解析 Markdown 文本为 `DocumentContent` |
| `MarkdownToDocxBuilder`（待建） | MD → DOCX | fluent 配置（输出路径、字体、页面） |
| OMML→LaTeX（待建） | 公式 | `<m:oMath>` XML → LaTeX 字符串 |
| SourceMap（待建） | 双向 | 行号 ↔ block index 映射 |

## 4. 关键数据流

### 4.1 DOCX → Markdown 转换

```
DocumentContent
    │
    ▼
MarkdownRenderer::render()
    │
    ├── metadata → YAML front matter
    │   "---\ntitle: '...'\nauthor: '...'\n---\n\n"
    │
    ├── Heading { level, runs }
    │   → "## **text**\n\n"
    │
    ├── Paragraph(runs)
    │   → "**bold** *italic* [link](url)\n\n"
    │
    ├── Table(DocumentTable)
    │   ├── 正常表格 → GFM pipe table
    │   │   "| col1 | col2 |\n|---|---|\n| a | b |\n\n"
    │   └── 合并单元格 → HTML <table> + ConversionWarning
    │
    ├── List(DocumentList)
    │   → "1. item\n2. item\n   - nested\n\n"
    │
    ├── Image(DocumentImage)
    │   → 提取到 image_directory + "![alt](assets/image1.png)\n\n"
    │
    ├── CodeBlock { language, code }
    │   → "```rust\ncode\n```\n\n"
    │
    ├── Footnote { id, blocks }
    │   → "[^id]: footnote text\n"
    │
    └── ThematicBreak / PageBreak / ColumnBreak
        → "---\n" / "<!-- page-break -->\n" / "<!-- column-break -->\n"
    │
    ▼
MarkdownResult {
    markdown: String,
    assets: Vec<ExtractedAsset>,
    warnings: Vec<ConversionWarning>,
}
```

### 4.2 Markdown → DOCX 转换（设计目标）

```
Markdown 文本
    │
    ▼
MarkdownParser::parse()
    │
    ├── # Heading → DocumentBlock::Heading { level: 1, runs: [...] }
    ├── paragraph → DocumentBlock::Paragraph(runs)
    ├── | table | → DocumentBlock::Table(DocumentTable)
    ├── - list → DocumentBlock::List(DocumentList)
    ├── ```code``` → DocumentBlock::CodeBlock { language, code }
    ├── ![img](path) → DocumentBlock::Image(DocumentImage)
    ├── [link](url) → DocumentTextRun { hyperlink: Some(url) }
    └── --- → DocumentBlock::ThematicBreak
    │
    ▼
DocumentContent { metadata, blocks }
    │
    ▼
EasyDoc::write_content(&content, "output.docx")
```

### 4.3 OMML → LaTeX（设计目标）

```
<m:oMath>
  <m:r><m:t>x</m:t></m:r>
  <m:sSup>
    <m:e><m:r><m:t>2</m:t></m:r></m:e>
  </m:sSup>
  <m:r><m:t>+1</m:t></m:r>
</m:oMath>
    │
    ▼
OMML → LaTeX 转换器
    │
    ▼
"x^{2}+1"
```

支持的 OMML 元素映射：

| OMML 元素 | LaTeX | 说明 |
|---|---|---|
| `<m:r>` | 文本 | 行内文本 |
| `<m:f>` | `\frac{...}{...}` | 分数 |
| `<m:sSup>` | `^{...}` | 上标 |
| `<m:sSub>` | `_{...}` | 下标 |
| `<m:rad>` | `\sqrt{...}` | 根号 |
| `<m:nary>` | `\int/sum/prod` | 积分/求和/求积 |
| `<m:d>` | `(...)` / `[...]` | 定界符 |
| `<m:bar>` | `\overline{...}` | 上划线 |
| `<m:acc>` | `\hat{...}` / `\vec{...}` | 重音 |
| `<m:eqArr>` | `\begin{aligned}...\end{aligned}` | 方程组 |

## 5. 技术决策与权衡

| # | 决策 | 理由 | 权衡 |
|---|---|---|---|
| 1 | MarkdownRenderer 消费 DocumentContent 而非直接解析 ZIP | 架构一致性：单一 IR，单一渲染路径 | 无法处理 reader 尚未支持的元素 |
| 2 | 合并单元格降级为 HTML table | GFM 不支持合并单元格 | 输出混合 Markdown + HTML，部分解析器不兼容 |
| 3 | 图片提取到独立目录 | 便于管理和引用 | 需要额外的文件系统操作 |
| 4 | Markdown → DOCX 复用 DocumentContent | 避免新建 IR | Markdown 的信息量少于 DOCX，转换会丢失样式 |
| 5 | OMML → LaTeX 作为独立模块 | 可被 MCP、CLI 等多处复用 | 需要维护 OMML 节点类型定义 |
| 6 | Source Map 用行号而非字节偏移 | 对用户更直观 | 行号在跨平台时可能不一致（CRLF vs LF） |

### 5.1 已知限制

1. **DOCX → Markdown 不可逆**：Markdown 丢失了字体、颜色、页面布局等信息，无法还原为原始 DOCX。
2. **OMML → LaTeX 依赖 office_oxide**：当前 office_oxide IR 暂不暴露 `<m:oMath>` 原始 XML，需上游支持或在 reader 层做额外提取。
3. **GFM 表格列宽**：Markdown 不支持指定列宽，渲染结果依赖查看器。

## 6. 测试与验收

### 6.1 现有测试

| 测试 | 断言点 | 文件 |
|---|---|---|
| `test_markdown_headings` | H1-H6 正确渲染为 `## **text**` | `markdown_conversion_test.rs` |
| `test_markdown_table` | GFM pipe table 格式正确 | `markdown_conversion_test.rs` |
| `test_markdown_merged_cells` | 降级为 HTML + warning | `markdown_conversion_test.rs` |
| `test_markdown_lists` | 有序/无序嵌套列表 | `markdown_conversion_test.rs` |
| `test_markdown_images` | 图片提取 + 引用路径 | `markdown_conversion_test.rs` |
| `test_markdown_code_block` | 语言标注 + 内容 | `markdown_conversion_test.rs` |
| `test_markdown_footnotes` | `[^id]: text` 格式 | `markdown_conversion_test.rs` |
| `test_markdown_front_matter` | YAML 头部正确 | `markdown_conversion_test.rs` |
| `test_markdown_end_to_end` | 生成 DOCX → 转换 → 验证 | `markdown_conversion_test.rs` |

### 6.2 待补充测试

- **Markdown → DOCX 往返**：`DOCX → Markdown → DOCX → Markdown` 两次转换结果一致。
- **OMML → LaTeX 精度**：嵌套公式（分数内含上标）的 LaTeX 输出正确。
- **Source Map 精度**：Markdown 第 N 行对应 DocumentContent.blocks[M]。
- **大文档转换**：1000+ 页 DOCX 的 Markdown 转换时间和内存。
- **Unicode 边界**：emoji、CJK 混排、RTL 文本的转换正确性。

## 7. 迁移路径

### Phase 4 实施顺序

1. **OMML → LaTeX**：独立模块，不依赖 Markdown parser，可先实现。
2. **Markdown → DOCX parser**：基于已有的 Markdown 解析库（如 `pulldown-cmark`），构建 `DocumentContent`。
3. **Source Map**：在 MarkdownRenderer 和 MarkdownParser 中嵌入位置信息。
4. **双向闭环验证**：DOCX → Markdown → DOCX 的 golden test。

## 8. 引用

- 架构文档：`docs/easydoc-rust-Architecture.zh_CN.md` 第 5.4 节「Markdown 转换流程」、第 11 节「easydoc-markdown 设计」
- 使用指南：`docs/usage-guide.md` 第 6 节「转换 Markdown」
- Roadmap：`docs/roadmap.md` Phase 2（Markdown）、Phase 4（OMML → LaTeX、Source Map）
- 源码：`crates/easydoc-markdown/src/`
