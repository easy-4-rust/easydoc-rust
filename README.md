# easydoc-rs

[![CI](https://github.com/hiwepy/easydoc-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/hiwepy/easydoc-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

`easydoc-rs` 是一个面向业务开发者的 Rust DOCX 生成库。它提供类似 Hutool
`Word07Writer` 的简单调用体验，同时借鉴 EasyExcel 的 Builder、独立上下文模型和显式
资源收尾设计，底层使用 [`docx-rs`](https://github.com/bokuweb/docx-rs) 生成
Office Open XML 文档。

当前版本专注于“可靠生成简单 DOCX”，不承诺完整的 Word 编辑器能力。

## 已支持

- 新建 DOCX、A4 页面和页边距
- 标题、段落、多个富文本 Run
- 字体、字号、颜色、粗体、斜体和下划线
- 中文字体的 `ascii`、`eastAsia`、`hAnsi`、`cs` 四槽位映射
- 段落对齐、缩进、命名样式和分页
- 表格、表头加粗、嵌套块和横向单元格合并
- PNG/JPEG 等 `docx-rs` 可识别图片
- 文件和 `Write + Seek` 输出
- 显式、可返回错误的 `finish()` 生命周期

## 快速开始

```toml
[dependencies]
easydoc = "0.1"
```

```rust
use easydoc::{
    Alignment, Cell, EasyDoc, Paragraph, ParagraphStyle, Pt, Row, Table, TextRun,
    TextStyle,
};

fn main() -> Result<(), easydoc::Error> {
    let mut writer = EasyDoc::write("report.docx")
        .default_font("宋体")
        .default_font_size(Pt(12.0))
        .build()?;

    writer.add_heading("年度经营报告", 1);
    writer.add_paragraph(
        Paragraph::new()
            .format(ParagraphStyle::default().align(Alignment::Center))
            .push(TextRun::new("2026 年度").format(TextStyle::default().bold())),
    );
    writer.add_table(
        Table::new()
            .push_row(Row::new([
                Cell::text("项目"),
                Cell::text("数量"),
                Cell::text("金额"),
            ]))
            .push_row(Row::new([
                Cell::text("订单"),
                Cell::text("120"),
                Cell::text("￥32,000"),
            ]))
            .first_row_as_header(),
    );

    writer.finish()?;
    Ok(())
}
```

`finish()` 是必需的。库不会在 `Drop` 中偷偷写文件，因为析构过程无法可靠地把 I/O
错误返回给调用者。

## Workspace

| Crate | 职责 |
|---|---|
| `easydoc` | 面向用户的 `EasyDoc` Builder 和 `DocxWriter` 门面 |
| `easydoc-core` | 与后端无关的文档树、样式、单位和错误类型 |
| `easydoc-docx` | 将中间模型转换为 `docx-rs` 并打包 DOCX |

详细设计见 [架构文档](docs/architecture.md)，版本边界见
[路线图](docs/roadmap.md)。

## 明确边界

- 不支持旧式二进制 `.doc`。
- 暂不承诺对任意已有 DOCX 进行无损修改。
- 模板替换不会通过“读出后重建整个文档”的方式实现；未来会直接保留未知 OOXML 部件。
- DOCX 转 PDF、Word 排版渲染和公式重算不属于核心库。

## 开发

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

## License

Apache License 2.0.
Ergonomic DOCX generation and templating for Rust
