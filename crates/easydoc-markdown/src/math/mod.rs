//! OMML (Office Math Markup Language) to LaTeX conversion.
//!
//! This module converts Office Math XML fragments (`<m:oMath>`) into LaTeX strings
//! suitable for embedding in Markdown as `$...$` (inline) or `$$...$$` (display).
//!
//! # Supported OMML structures
//!
//! | OMML element | LaTeX output |
//! |---|---|
//! | `<m:r>` (text run) | Text with symbol mapping and LaTeX escaping |
//! | `<m:f>` (fraction) | `\frac{num}{den}` |
//! | `<m:rad>` (radical) | `\sqrt{text}` / `\sqrt[n]{text}` |
//! | `<m:sSub>` (subscript) | `base_{sub}` |
//! | `<m:sSup>` (superscript) | `base^{sup}` |
//! | `<m:sSubSup>` (sub-superscript) | `base_{sub}^{sup}` |
//! | `<m:nary>` (n-ary operator) | `\sum`, `\int`, etc. with limits |
//! | `<m:d>` (delimiter) | `\left( ... \right)` |
//! | `<m:acc>` (accent) | `\hat{}`, `\vec{}`, etc. |
//! | `<m:bar>` (bar) | `\overline{}`, `\underline{}` |
//! | `<m:m>` (matrix) | `\begin{matrix}...\end{matrix}` |
//! | `<m:func>` (function) | `\sin()`, `\cos()`, etc. |
//! | `<m:groupChr>` (group character) | `\underbrace{}`, `\overbrace{}` |
//! | `<m:limLow>` (lower limit) | `\lim_{...}` |
//! | `<m:limUpp>` (upper limit) | `\overset{...}{...}` |
//! | `<m:eqArr>` (equation array) | `\begin{array}{c}...\end{array}` |

pub mod latex_dict;
pub mod omml_to_latex;
