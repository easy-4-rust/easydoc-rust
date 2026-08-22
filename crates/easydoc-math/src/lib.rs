//! LaTeX ↔ OMML（Office Math Markup Language，Word 原生公式格式）双向转换。
//!
//! 两个独立方向的转换模块：
//! - [`omml_to_latex`]：OMML → LaTeX，用于 DOCX → Markdown 的公式渲染。
//!   设计参考 markitdown（`omml.py`）/ dwml 与 litchi（Apache-2.0）。
//! - [`latex_to_omml`]：LaTeX → OMML，用于 Markdown → DOCX 的公式写回。
//!   设计参考 tex2word-math（MIT, Yifan Yang / rstex2word）。
//! - [`latex_dict`]：两方向共用的符号映射表。
//!
//! 两模块均为自研实现，吸收的开源项目内容已在模块头与
//! 仓库根 `THIRD_PARTY.md` 中标注来源与版权。

pub mod latex_dict;
pub mod latex_to_omml;
pub mod omml_to_latex;
