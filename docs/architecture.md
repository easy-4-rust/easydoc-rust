# easydoc-rs 技术架构

## 设计来源

Hutool `Word07Writer` 提供了非常短的业务调用链：创建 Writer、追加文本/表格/图片、
最后刷新或关闭。EasyExcel 则把 Builder、元数据、上下文、转换和输出生命周期分离。
`easydoc-rs` 组合两者的优势，但不复制 Apache POI 类型，也不把 Excel 的 Sheet/Row
流模型套到 DOCX。

## 总体结构

```mermaid
flowchart LR
    A["业务调用 EasyDoc / DocxWriter"] --> B["easydoc-core 文档中间模型"]
    B --> C["样式继承与单位转换"]
    C --> D["easydoc-docx Renderer"]
    D --> E["docx-rs 对象模型"]
    E --> F["WordprocessingML + ZIP"]
    F --> G[".docx 文件或 Write + Seek"]
```

依赖方向始终由门面指向模型和后端：

```mermaid
flowchart TD
    easydoc --> core["easydoc-core"]
    easydoc --> backend["easydoc-docx"]
    backend --> core
    backend --> docxrs["docx-rs"]
    core -. "不依赖格式实现" .-> neutral["纯 Rust 文档类型"]
```

## 文档模型

```mermaid
classDiagram
    class Document {
      DocumentConfig config
      Map styles
      Vec~Block~ blocks
    }
    class Block
    class Paragraph {
      ParagraphStyle style
      Vec~Inline~ children
    }
    class Inline
    class TextRun {
      String text
      TextStyle style
    }
    class Table {
      Vec~Row~ rows
    }
    class Cell {
      Vec~Block~ blocks
      usize colspan
    }
    Document *-- Block
    Block <|-- Paragraph
    Block <|-- Table
    Paragraph *-- Inline
    Inline <|-- TextRun
    Table *-- Cell
    Cell *-- Block
```

段落样式与文字样式分离，因为 WordprocessingML 中 `w:pPr` 和 `w:rPr` 是不同层级。
单元格保存块级内容而不是裸字符串，以便自然扩展到多段落、嵌套表格和图片。

## 样式解析

样式采用三级覆盖，越靠后优先级越高：

```mermaid
flowchart LR
    A["文档默认字体/字号"] --> B["命名样式"]
    B --> C["段落局部样式"]
    C --> D["TextRun 局部样式"]
    D --> E["最终 OOXML Run 属性"]
```

中文字体不会只设置 ASCII 字体。`FontFamily::all("宋体")` 会同时写入 ASCII、East
Asia、High ANSI 和 Complex Script 四个槽位。

## Writer 生命周期

```mermaid
sequenceDiagram
    participant App as 业务代码
    participant Builder as DocxWriterBuilder
    participant Writer as DocxWriter
    participant Renderer as DocxRenderer
    participant File as DOCX 文件
    App->>Builder: EasyDoc::write(path)
    App->>Builder: 配置字体、页面、边距
    Builder->>Writer: build()
    App->>Writer: add_paragraph/table/image
    App->>Writer: finish()
    Writer->>Renderer: render(document)
    Renderer->>File: pack WordprocessingML
    File-->>App: Result
```

`Drop` 仅释放内存，不写文件。所有打包和 I/O 错误都由 `finish()` 返回。

## 后续扩展边界

- `WriteHandler` 只操作 `easydoc-core` 模型，普通扩展不接触 `docx-rs`。
- 模板引擎直接变换 DOCX ZIP 中的相关 XML，保留未知部件、关系、媒体和样式。
- 读取已有文档只承诺映射本项目支持的结构；“无损编辑”需要独立的包级变换能力。
- 当第二种文档后端真正复用同一抽象时，再考虑提取通用 backend trait。
