# Roadmap

## 0.1 — 基础 DOCX 生成 ✅ 已完成

- [x] 独立文档中间模型
- [x] `EasyDoc` Builder 与 `DocxWriter`
- [x] 标题、段落、富文本 Run 和分页
- [x] 字体、字号、颜色、加粗、斜体、下划线和对齐
- [x] A4 页面、页边距、强类型单位和中文字体槽位
- [x] 表格、嵌套块、表头样式和横向合并
- [x] 块级与行内图片
- [x] 显式 `finish()` 与类型化错误
- [x] DOCX ZIP/XML 集成测试

## 0.2 — 安全基础设施 ✅ 已完成

- [x] `easydoc-ooxml`：原子文件写入（临时文件 + persist）
- [x] `PackageLimits`：ZIP 条目数、单条目大小、总大小、压缩比限制
- [x] `PackageRewriter`：安全 ZIP 重写，未修改条目逐字节保留
- [x] 模板 XML 特殊字符转义（`&`, `<`, `>`, `"`, `'`）
- [x] 跨 `<w:t>` 节点的标量占位符替换
- [x] H1–H6 标题写入 Word 标题样式 + outline level

## 0.3 — 语义模型与 Markdown 🔧 进行中

- [x] `DocumentContent` / `DocumentBlock` 后端无关语义模型
- [x] `read_document()` 将 DOC/DOCX 转为 `DocumentContent`
- [x] `easydoc-markdown`：DOC/DOCX → Markdown 转换
  - [x] 标题、富文本、超链接
  - [x] GFM 表格（管道转义、自动列宽）
  - [x] 合并单元格 → HTML `<table>` + 降级警告
  - [x] 有序/无序嵌套列表
  - [x] 代码块、脚注、尾注
  - [x] 图片提取（可配置目录和引用前缀）
  - [x] YAML front matter
  - [x] 原子文件输出
- [ ] 整合/废弃旧 `model.rs`
- [ ] Writer 使用 `easydoc-core` 语义模型

## 0.4 — 页面与样式完善

- [ ] 行距、段前段后、边框和背景色
- [ ] 表格列宽、单元格边距、纵向合并和重复表头
- [ ] 页眉、页脚、页码、编号和项目符号
- [ ] 命名样式输出到 DOCX styles.xml

## 0.5 — Event 链与高级读取

- [ ] `DocumentEvent` 枚举
- [ ] `DocumentEventSink` trait（流式读取）
- [ ] `DocumentReader` trait（`read_model()` + `read_events()`）
- [ ] Writer 重构为 `DocxRenderer` + core model
- [ ] `#[derive(DocxRow)]` 派生宏增强

## 0.6 — 高级模板与公式

- [ ] 条件模板引擎
- [ ] 图片模板引擎
- [ ] 列表、条件块和图片变量
- [ ] 公式支持（OMML → LaTeX）
- [ ] 批注（Comments）和修订追踪（Revisions）
- [ ] Markdown source map

## 0.7 — 生态与工具链

- [ ] `easydoc-cli` 命令行工具
- [ ] `easydoc-mcp` MCP 集成
- [ ] `easydoc-web` Web 响应适配
- [ ] benchmarks、golden tests、fuzz tests
- [ ] `tests/fixtures/` 真实文档集

## 非目标

- 旧式 `.doc` 二进制格式写入（只读支持通过 `office_oxide`）
- 完整 Word 排版/渲染引擎
- DOCX 到 PDF 的像素级转换
- 公式重新计算、协同编辑和完整修订跟踪
- OCR/LLM 图片描述（通过 Trait 注入，非默认依赖）
