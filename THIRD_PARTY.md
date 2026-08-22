# Third-party notices

本仓库的数学转换模块（`crates/easydoc-math`）为自研实现，但设计、符号映射与
部分代码路径参考了以下开源项目。各来源的版权与许可证信息如下，吸收内容均在
对应模块头注释中标注。

## tex2word-math — MIT

- 项目：https://github.com/yfyang86/rstex2word （作者 Yifan Yang）
- 许可证：MIT（crate `tex2word-math` 1.0.6）
- 用途：`easydoc-math/src/latex_to_omml.rs` 的设计参考——
  LaTeX 递归下降解析 → 数学 AST → OMML 渲染的架构、符号/函数/n-ary/字母表
  映射表。自研实现在此基础上扩展了严格错误通道与缺失命令。

## litchi — Apache-2.0

- 项目：https://crates.io/crates/litchi （0.0.1）
- 许可证：Apache-2.0
- 用途：`easydoc-math/src/omml_to_latex.rs` 的设计参考——
  `m:phant` → `\phantom{}`、`m:borderBox` → `\boxed{}`、run 样式
  （`m:sty`/`m:scr`）的处理方式。

## markitdown — MIT

- 项目：https://github.com/microsoft/markitdown
- 许可证：MIT
- 用途：`easydoc-math/src/omml_to_latex.rs` 与 `latex_dict.rs` 移植自
  `omml.py` / `latex_dict.py`（OMML → LaTeX 主逻辑与符号表），
  后者又源自 [dwml](https://github.com/xiilei/dwml)。

## dwml — MIT

- 项目：https://github.com/xiilei/dwml
- 许可证：MIT
- 用途：markitdown 的 OMML → LaTeX 逻辑原始出处。

---

本仓库许可证：Apache-2.0（见仓库根 LICENSE）。
