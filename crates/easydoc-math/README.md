<a id="readme-top"></a>

<div align="center">

# easydoc-math

**LaTeX ↔ OMML (Office Math Markup Language) bidirectional conversion for the easydoc-rust workspace.**

[![Crates.io](https://img.shields.io/crates/v/easydoc-math)](https://crates.io/crates/easydoc-math)
[![docs.rs](https://img.shields.io/docsrs/easydoc-math)](https://docs.rs/easydoc-math)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](#rust-baseline)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

[Positioning](#positioning) · [Usage](#usage) · [Supported constructs](#supported-constructs) ·
[Error handling](#error-handling) · [Attribution](#attribution) · [License](#license)

</div>

---

> **Status**: stable (0.1.x, API frozen within the 0.x line)
> **MSRV**: Rust `1.88`
> **Edition**: `2024`
> **Dependencies**: `easydoc-core`, `quick-xml` only

## Positioning

`easydoc-math` 是 easydoc-rust 工作区中专司**公式转换**的 crate，提供两个独立方向的转换模块：

| 模块 | 方向 | 用途 |
|---|---|---|
| [`omml_to_latex`](https://docs.rs/easydoc-math/latest/easydoc_math/omml_to_latex/) | OMML → LaTeX | DOCX 读回：把 Word 原生公式还原为 Markdown `$...$` / `$$...$$` |
| [`latex_to_omml`](https://docs.rs/easydoc-math/latest/easydoc_math/latex_to_omml/) | LaTeX → OMML | DOCX 写出：把 Markdown 公式转成 Word 可编辑的原生公式 |

两个模块都是自研实现，架构与符号映射吸收了 tex2word-math（MIT）、litchi（Apache-2.0）、markitdown/dwml（MIT）的优秀设计，来源与版权见仓库根 [`THIRD_PARTY.md`](https://github.com/easy-4-rust/easydoc-rust/blob/main/THIRD_PARTY.md)。

## Usage

```toml
[dependencies]
easydoc-math = "0.1"
```

### OMML → LaTeX（读方向）

```rust
use easydoc_math::omml_to_latex;

let omml = r#"<m:oMath><m:f><m:num><m:r><m:t>1</m:t></m:r></m:num>
<m:den><m:r><m:t>2</m:t></m:r></m:den></m:f></m:oMath>"#;
let latex = omml_to_latex::convert(omml)?; // "\\frac{1}{2}"
```

### LaTeX → OMML（写方向）

```rust
use easydoc_math::latex_to_omml;

let latex = r"\frac{a}{b} + \sum_{i=1}^{n} x_i";
let omml = latex_to_omml::convert(latex)?; // "<m:oMath>...</m:oMath>"
```

## Supported constructs

### OMML → LaTeX

文本 run（含全量数学字母表：粗体/粗斜体/无衬线/等宽/双空/手写/哥特/数字）、
分数（`bar`/`skw`/`noBar`/`lin`）、根式（含 n 次根）、上下标与前标、n-ary 算子
（`limLoc`/`subHide`/`supHide`）、定界符（含多元素堆叠与 `sepChr`）、重音、
上下线、函数（`\sin` 等）、花括号（`groupChr`）、上下限（`limLow`/`limUpp`）、
矩阵（含 `mcJc` 列对齐、`mcs` 列序列）、等式数组（`eqArrPr baseJc`）、
`box`（`opEmu`）、`phant`（`\phantom`）、`borderBox`（`\boxed`）、run 样式
（`m:sty`/`m:scr`）。兼容真实 Word 产出的**属性形式** Pr（`<m:dPr m:begChr="["/>`）。

### LaTeX → OMML

分数/根式/上下标/`\left...\right` 定界符/n-ary（求和与积分上下限布局）/
重音与上下线/`\mathbf` 等样式/`\binom`/矩阵环境（`pmatrix`/`bmatrix`/`cases`
等）/`aligned` 等式数组/159+ 符号与 35 函数名，以及扩展命令：

`\underbrace` `\overbrace` `\overset` `\underset` `\stackrel` `\boxed`
`\operatorname*` `\lim_{x\to 0}`（极限布局）`\not`（U+0338）
`\limits`/`\nolimits` 与分级间距（`\,` `\:` `\;` `\quad` `\!`）。

## Error handling

`latex_to_omml::convert` 采用**严格错误通道**：未知命令、括号不配对、嵌套过深、
无法无损表达的构造一律返回 `Err`——绝不静默丢弃输入字符。调用方（如
easydoc-writer）在转换失败时回退保留 `$latex$` 原文，保证内容零丢失。

## Rust baseline

- **MSRV**: Rust `1.88`（CI 6-matrix 持续验证）
- **Edition**: `2024`，`unsafe_code = "forbid"`

## License

Apache-2.0（详见仓库根 [`LICENSE`](https://github.com/easy-4-rust/easydoc-rust/blob/main/LICENSE)）。
第三方吸收内容（tex2word-math / litchi / markitdown / dwml）的版权与许可证见
[`THIRD_PARTY.md`](https://github.com/easy-4-rust/easydoc-rust/blob/main/THIRD_PARTY.md)。
