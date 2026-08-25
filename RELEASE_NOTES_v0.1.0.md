# Release v0.1.0

首个稳定发布。公共 API 冻结，Markdown↔DOCX 双向转换完整，数学公式双向转换
生产就绪。**1134 个测试全绿**，clippy `-D warnings` 0 警告，rustfmt 100% 合规。

- **API 稳定性**：0.1.0 起公共 API 冻结（0.x 线向后兼容），
  `#[non_exhaustive]` 覆盖全部公共枚举，`cargo-semver-checks` CI 门禁
- **MSRV**：Rust 1.88.0（CI 6-matrix 持续验证）
- **License**：Apache-2.0（含 `THIRD_PARTY.md` 第三方协议标注）

## 新增能力（相对 alpha.2）

### 数学公式双向转换（新 crate `easydoc-math`）

- **OMML → LaTeX**（读方向）：
  - 兼容真实 Word 产出的属性形式 Pr（`<m:dPr m:begChr="["/>`）
  - `m:box`/boxPr、`m:d` 多元素堆叠、`m:naryPr` limLoc/subHide/supHide
  - 全量数学字母表：粗体/粗斜体/无衬线/等宽/双空/手写/哥特（含带缺口大写）/数学数字
  - 矩阵/等式数组布局属性（`mcJc`/`baseJc` → `array` 列对齐）
- **LaTeX → OMML**（写方向，自研）：
  - 递归下降解析 → 数学 AST → OMML；覆盖 frac/sqrt/上下标/`\left\right`/
    n-ary/重音/上下线/`\mathbf`/`\binom`/矩阵/cases/159 符号/35 函数
  - 扩展 `\underbrace/\overbrace`、`\overset/\underset/\stackrel`、
    `\lim` 极限布局、`\boxed`、`\operatorname*`、`\not`、间距命令分级
  - **严格错误通道**：未知命令/无法无损表达 → `Err`，调用方回退保留 `$latex$`，
    零静默内容丢失
- **writer 集成**：`xmlns:m` 命名空间注入（Word 识别公式的前提）、
  块级公式 `<m:oMathPara>` 居中、真实失败兜底
- **往返保证**：LaTeX → OMML → LaTeX 精确还原（往返测试 + proptest +
  `fuzz_math_converter` fuzz target）

### 性能回归门禁

- CI `bench-regression` 任务：缓存 `write_throughput` 基准，回归 >10% 即失败
- 写入吞吐 ~95k rows/s（1K 行 10.5ms）

### MCP

- `DirectoryResourceProvider` 根目录可配置（`EASYDOC_MCP_ROOT` /
  `default_config_with_root` / 自定义 provider）

### 测试与质量

- 1134 测试（+49 自 alpha.2）：往返精确断言、真实 Word 风格 OMML 集成、
  proptest 属性测试、golden 快照
- fuzz target 4 个（每日 CI）
- cargo-audit + cargo-deny 每周安全审计

## 破坏性变更

无（0.1.0-alpha.x → 0.1.0 按 semver 0.x 规则，API 自本版本起冻结）。
