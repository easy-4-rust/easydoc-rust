//! OMML (Office Math Markup Language) to LaTeX converter.
//!
//! Recursive descent over `<m:oMath>` XML, producing a LaTeX string suitable
//! for inclusion in Markdown `$...$` or `$$...$$`.
//!
//! Ported from the Python `markitdown` project (`omml.py`), which itself was
//! adapted from [dwml](https://github.com/xiilei/dwml)。结构语义（phantom/
//! borderBox/run 样式）参考 [litchi](https://crates.io/crates/litchi)
//! （Apache-2.0）。来源与版权详见仓库根 `THIRD_PARTY.md`。

use std::collections::HashMap;
use std::io::BufRead;

use quick_xml::Reader;
use quick_xml::events::BytesStart;
use quick_xml::events::Event;

use super::latex_dict::{
    self, ACCENT_DEFAULT, ALN, ARRAY_TEMPLATE, BAR_POS_DEFAULT, BRK, CHARS, DELIMITER_DEFAULT_LEFT,
    DELIMITER_DEFAULT_RIGHT, DELIMITER_NULL, DELIMITER_STACK_TEMPLATE, DELIMITER_TEMPLATE,
    FRACTION_DEFAULT, FUNC_PLACE, GROUP_CHR_DEFAULT, LIM_ARROW_FROM, LIM_ARROW_TO,
    LIM_UPPER_TEMPLATE, MATRIX_TEMPLATE, RADICAL_DEFAULT_TEMPLATE, RADICAL_DEG_TEMPLATE,
    SUB_TEMPLATE, SUP_TEMPLATE, run_style_command,
};

/// The OMML XML namespace prefix as it appears in prefixed documents (`m:`).
const OMML_NS_PREFIX: &str = "m:";

/// Top-level entry point: convert an OMML XML string to a LaTeX string.
///
/// The input should be the `<m:oMath>...</m:oMath>` element (with or without
/// a wrapping root element and namespace declarations).
///
/// # Errors
///
/// Returns an error if the XML is malformed.
pub fn convert(omml: &str) -> easydoc_core::Result<String> {
    let mut converter = OmmlConverter::new();
    converter.convert_str(omml)
}

// ---------------------------------------------------------------------------
// Internal converter
// ---------------------------------------------------------------------------

/// Stateful converter holding lazily-built symbol tables.
struct OmmlConverter {
    text_symbols: HashMap<String, String>,
    big_operators: HashMap<&'static str, &'static str>,
    accents: HashMap<&'static str, &'static str>,
    func_names: HashMap<&'static str, &'static str>,
    fraction_styles: HashMap<&'static str, &'static str>,
    limit_functions: HashMap<&'static str, &'static str>,
    bar_positions: HashMap<&'static str, &'static str>,
}

/// Parsed properties from an `<m:xxxPr>` element.
///
/// 属性既可能出现在 Pr 元素的**自身属性**上（`<m:dPr m:begChr="["/>`，
/// 真实 Word 的常见形式），也可能出现在**子元素带 `m:val`** 上
/// （`<m:dPr><m:begChr m:val="["/></m:dPr>`，本项目自产 OMML 的形式）；
/// [`OmmlConverter::fill_pr`] 两种形式都读取。
struct PrProps {
    /// Raw `chr` attribute value.
    chr: Option<String>,
    /// Raw `pos` attribute value.
    pos: Option<String>,
    /// Raw `begChr` attribute value.
    beg_chr: Option<String>,
    /// Raw `endChr` attribute value.
    end_chr: Option<String>,
    /// Raw `type` attribute value.
    typ: Option<String>,
    /// Raw `sepChr` attribute value（分隔符内堆叠元素之间的分隔符）。
    sep_chr: Option<String>,
    /// Raw `limLoc` attribute value（n-ary 上下限位置：`undOvr`/`subSup`）。
    lim_loc: Option<String>,
    /// Raw `subHide` attribute value（n-ary 下标隐藏）。
    sub_hide: Option<String>,
    /// Raw `supHide` attribute value（n-ary 上标隐藏）。
    sup_hide: Option<String>,
    /// Raw `grow` attribute value（n-ary 是否拉伸）。
    grow: Option<String>,
    /// Raw `opEmu` attribute value（box 运算符模拟器）。
    op_emu: Option<String>,
    /// Raw `noBreak` attribute value（box 禁止换行）。
    no_break: Option<String>,
    /// Raw `diff` attribute value（box 微分算子）。
    diff: Option<String>,
}

impl PrProps {
    fn empty() -> Self {
        Self {
            chr: None,
            pos: None,
            beg_chr: None,
            end_chr: None,
            typ: None,
            sep_chr: None,
            lim_loc: None,
            sub_hide: None,
            sup_hide: None,
            grow: None,
            op_emu: None,
            no_break: None,
            diff: None,
        }
    }
}

impl OmmlConverter {
    fn new() -> Self {
        Self {
            text_symbols: latex_dict::build_text_symbols(),
            big_operators: latex_dict::build_big_operators(),
            accents: latex_dict::build_accents(),
            func_names: latex_dict::build_func_names(),
            fraction_styles: latex_dict::build_fraction_styles(),
            limit_functions: latex_dict::build_limit_functions(),
            bar_positions: latex_dict::build_bar_positions(),
        }
    }

    /// Convert an OMML XML string to LaTeX.
    fn convert_str(&mut self, omml: &str) -> easydoc_core::Result<String> {
        let mut reader = Reader::from_str(omml);
        // We need to find the <m:oMath> element. The input may be a bare
        // <m:oMath> or wrapped in a root element with namespace declarations.
        loop {
            let mut buf = Vec::new();
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) if is_omath(&e) => {
                    return self.process_children_to_string(&mut reader);
                }
                Ok(Event::Empty(e)) if is_omath(&e) => {
                    return Ok(String::new());
                }
                Ok(Event::Eof) => {
                    // No <m:oMath> found; try parsing the whole input as-is.
                    let mut reader2 = Reader::from_str(omml);
                    return self.process_children_to_string(&mut reader2);
                }
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Recursive child processing
    // -----------------------------------------------------------------------

    /// Process all children of the current element, concatenating their LaTeX
    /// output. Consumes events until the matching `End` tag is found.
    fn process_children_to_string<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut parts = Vec::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let stag = local_name(&e);
                    if let Some(latex) = self.dispatch_element(&stag, reader)? {
                        parts.push(latex);
                    }
                }
                Ok(Event::Empty(e)) => {
                    let stag = local_name(&e);
                    if let Some(latex) = dispatch_empty(&stag, &e) {
                        parts.push(latex);
                    }
                }
                Ok(Event::Text(t)) => {
                    let text = String::from_utf8_lossy(&t).into_owned();
                    if !text.is_empty() {
                        parts.push(text);
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        Ok(parts.join(""))
    }

    /// Dispatch a start-tag to the appropriate handler, consuming through its
    /// end-tag. Returns `None` for unrecognized tags (silently skipped).
    fn dispatch_element<R: BufRead>(
        &mut self,
        stag: &str,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<Option<String>> {
        match stag {
            "oMath" | "e" | "num" | "den" | "deg" | "sSub" | "sSup" | "sSubSup" => {
                let latex = self.process_children_to_string(reader)?;
                Ok(Some(latex))
            }
            "box" => {
                let latex = self.process_box(reader)?;
                Ok(Some(latex))
            }
            "spre" => {
                let latex = self.process_spre(reader)?;
                Ok(Some(latex))
            }
            "phant" => {
                let latex = self.process_phantom(reader)?;
                Ok(Some(latex))
            }
            "borderBox" => {
                let latex = self.process_border_box(reader)?;
                Ok(Some(latex))
            }
            "r" => {
                let latex = self.process_run(reader)?;
                Ok(Some(latex))
            }
            "f" => {
                let latex = self.process_fraction(reader)?;
                Ok(Some(latex))
            }
            "rad" => {
                let latex = self.process_radical(reader)?;
                Ok(Some(latex))
            }
            "d" => {
                let latex = self.process_delimiter(reader)?;
                Ok(Some(latex))
            }
            "acc" => {
                let latex = self.process_accent(reader)?;
                Ok(Some(latex))
            }
            "bar" => {
                let latex = self.process_bar(reader)?;
                Ok(Some(latex))
            }
            "nary" => {
                let latex = self.process_nary(reader)?;
                Ok(Some(latex))
            }
            "func" => {
                let latex = self.process_func(reader)?;
                Ok(Some(latex))
            }
            "groupChr" => {
                let latex = self.process_group_chr(reader)?;
                Ok(Some(latex))
            }
            "limLow" => {
                let latex = self.process_lim_low(reader)?;
                Ok(Some(latex))
            }
            "limUpp" => {
                let latex = self.process_lim_upp(reader)?;
                Ok(Some(latex))
            }
            "lim" => {
                let latex = self.process_lim(reader)?;
                Ok(Some(latex))
            }
            "m" => {
                let latex = self.process_matrix(reader)?;
                Ok(Some(latex))
            }
            "mr" => {
                let latex = self.process_matrix_row(reader)?;
                Ok(Some(latex))
            }
            "eqArr" => {
                let latex = self.process_eq_arr(reader)?;
                Ok(Some(latex))
            }
            "sub" => {
                let inner = self.process_children_to_string(reader)?;
                Ok(Some(SUB_TEMPLATE.replace("{0}", &inner)))
            }
            "sup" => {
                let inner = self.process_children_to_string(reader)?;
                Ok(Some(SUP_TEMPLATE.replace("{0}", &inner)))
            }
            tag if tag.ends_with("Pr") => {
                let _ = self.process_children_to_string(reader)?;
                Ok(None)
            }
            _ => {
                let _ = self.process_children_to_string(reader)?;
                Ok(None)
            }
        }
    }

    /// `<m:phant>` -- phantom：内容不可见但占据空间（常用于对齐排版）。
    ///
    /// 参考 litchi 的 `MathNode::Phantom` 处理，输出 `\phantom{...}`。
    fn process_phantom<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let inner = self.process_children_to_string(reader)?;
        Ok(format!("\\phantom{{{inner}}}"))
    }

    /// `<m:borderBox>` -- 边框盒子：内容外加方框。
    ///
    /// 参考 litchi 的 `MathNode::BorderBox` 处理，输出 `\boxed{...}`。
    fn process_border_box<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let inner = self.process_children_to_string(reader)?;
        Ok(format!("\\boxed{{{inner}}}"))
    }

    /// `<m:box>` -- 盒装表达式。
    ///
    /// 解析 `boxPr`：`opEmu`（运算符模拟器，内容按大算子行为）→ `\mathop{...}`；
    /// `noBreak`（禁止换行）/`diff`（微分算子）对 LaTeX 文本输出无影响，保持内容。
    fn process_box<R: BufRead>(&mut self, reader: &mut Reader<R>) -> easydoc_core::Result<String> {
        let mut inner = String::new();
        let mut pr = PrProps::empty();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "e" => inner = self.process_children_to_string(reader)?,
                        "boxPr" => {
                            self.fill_pr(&e, reader, &mut pr)?;
                        }
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    if tag == "boxPr" {
                        apply_pr_attrs(&e, &mut pr);
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        if pr.op_emu.as_deref() == Some("1") {
            Ok(format!("\\mathop{{{inner}}}"))
        } else {
            Ok(inner)
        }
    }

    // -----------------------------------------------------------------------
    // OMML element handlers
    // -----------------------------------------------------------------------

    /// `<m:r>` -- text run. Maps each character through the symbol table and
    /// escapes LaTeX special characters.
    ///
    /// Consumes through the closing `</m:r>` tag (depth-tracked). Honors the
    /// run style from `<m:rPr>`: `<m:sty>` (`p`/`b`/`i`/`bi`) and `<m:scr>`
    /// (double-struck, script, ...) wrap the run text in the corresponding
    /// LaTeX style command (`\mathrm{}`, `\mathbf{}`, ...).
    fn process_run<R: BufRead>(&mut self, reader: &mut Reader<R>) -> easydoc_core::Result<String> {
        let mut text_parts = Vec::new();
        let mut buf = Vec::new();
        let mut in_text = false;
        let mut depth = 1_u32;
        let mut sty_cmd: Option<&'static str> = None;
        let mut scr_cmd: Option<&'static str> = None;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    let tag = local_name(&e);
                    if tag == "t" {
                        in_text = true;
                    } else if tag == "sty" || tag == "scr" {
                        capture_run_style(&e, &tag, &mut sty_cmd, &mut scr_cmd);
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    if tag == "sty" || tag == "scr" {
                        capture_run_style(&e, &tag, &mut sty_cmd, &mut scr_cmd);
                    }
                }
                Ok(Event::Text(t)) if in_text => {
                    let raw = String::from_utf8_lossy(&t);
                    let mut mapped = String::with_capacity(raw.len());
                    for c in raw.chars() {
                        let mut char_buf = [0u8; 4];
                        let s = c.encode_utf8(&mut char_buf);
                        match self.text_symbols.get(s) {
                            Some(replacement) => mapped.push_str(replacement),
                            None => mapped.push_str(&escape_char(c)),
                        }
                    }
                    text_parts.push(mapped);
                }
                Ok(Event::End(_)) => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    in_text = false;
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        let content = text_parts.join("");
        if content.is_empty() {
            return Ok(content);
        }
        // `<m:scr>` 语义更丰富，优先级高于 `<m:sty>`。
        let cmd = scr_cmd.or(sty_cmd);
        Ok(match cmd {
            Some(cmd) => format!("{cmd}{{{content}}}"),
            None => content,
        })
    }

    /// `<m:f>` -- fraction. Extracts `<m:num>` and `<m:den>` children and
    /// applies the fraction style from `<m:fPr>`.
    fn process_fraction<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut num = String::new();
        let mut den = String::new();
        let mut pr = PrProps::empty();
        let pr_text = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "num" => num = self.process_children_to_string(reader)?,
                        "den" => den = self.process_children_to_string(reader)?,
                        "fPr" => {
                            self.fill_pr(&e, reader, &mut pr)?;
                        }
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    if tag == "fPr" {
                        apply_pr_attrs(&e, &mut pr);
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        let template = pr
            .typ
            .as_deref()
            .and_then(|t| self.fraction_styles.get(t))
            .map_or(FRACTION_DEFAULT, |s| *s);
        let result = template.replace("{num}", &num).replace("{den}", &den);
        Ok(format!("{pr_text}{result}"))
    }

    /// `<m:rad>` -- radical (square root or n-th root).
    fn process_radical<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut text = String::new();
        let mut deg = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "e" => text = self.process_children_to_string(reader)?,
                        "deg" => deg = self.process_children_to_string(reader)?,
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        if deg.is_empty() {
            Ok(RADICAL_DEFAULT_TEMPLATE.replace("{text}", &text))
        } else {
            Ok(RADICAL_DEG_TEMPLATE
                .replace("{deg}", &deg)
                .replace("{text}", &text))
        }
    }

    /// `<m:d>` -- delimiter (parentheses, brackets, etc.).
    fn process_delimiter<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut texts = Vec::new();
        let mut pr = PrProps::empty();
        let pr_text = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "e" => texts.push(self.process_children_to_string(reader)?),
                        "dPr" => {
                            self.fill_pr(&e, reader, &mut pr)?;
                        }
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    if tag == "dPr" {
                        apply_pr_attrs(&e, &mut pr);
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        let left = match pr.beg_chr.as_deref() {
            Some(v) => match self.text_symbols.get(v) {
                Some(s) => s.clone(),
                None => escape_latex(v),
            },
            None => DELIMITER_DEFAULT_LEFT.to_owned(),
        };
        let right = match pr.end_chr.as_deref() {
            Some(v) => match self.text_symbols.get(v) {
                Some(s) => s.clone(),
                None => escape_latex(v),
            },
            None => DELIMITER_DEFAULT_RIGHT.to_owned(),
        };
        let left = if left.is_empty() {
            DELIMITER_NULL.to_owned()
        } else {
            left
        };
        let right = if right.is_empty() {
            DELIMITER_NULL.to_owned()
        } else {
            right
        };
        // 多个 `<m:e>` 表示上下堆叠（如 \left(\begin{matrix}..\end{matrix}\right)）；
        // `sepChr`（分隔符内堆叠元素的分隔字符）在 LaTeX 中由矩阵行换行承担，不单独输出。
        let text = texts.join(BRK);
        let result = if texts.len() > 1 {
            DELIMITER_STACK_TEMPLATE
                .replace("{left}", &left)
                .replace("{text}", &text)
                .replace("{right}", &right)
        } else {
            DELIMITER_TEMPLATE
                .replace("{left}", &left)
                .replace("{text}", &text)
                .replace("{right}", &right)
        };
        Ok(format!("{pr_text}{result}"))
    }

    /// `<m:acc>` -- accent (hat, tilde, bar, etc.).
    fn process_accent<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut inner = String::new();
        let mut pr = PrProps::empty();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "e" => inner = self.process_children_to_string(reader)?,
                        "accPr" => {
                            self.fill_pr(&e, reader, &mut pr)?;
                        }
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    if tag == "accPr" {
                        apply_pr_attrs(&e, &mut pr);
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        let template = pr
            .chr
            .as_deref()
            .and_then(|c| self.accents.get(c))
            .map_or(ACCENT_DEFAULT, |s| *s);
        Ok(template.replace("{0}", &inner))
    }

    /// `<m:bar>` -- overbar / underline.
    fn process_bar<R: BufRead>(&mut self, reader: &mut Reader<R>) -> easydoc_core::Result<String> {
        let mut inner = String::new();
        let mut pr = PrProps::empty();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "e" => inner = self.process_children_to_string(reader)?,
                        "barPr" => {
                            self.fill_pr(&e, reader, &mut pr)?;
                        }
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    if tag == "barPr" {
                        apply_pr_attrs(&e, &mut pr);
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        let template = pr
            .pos
            .as_deref()
            .and_then(|p| self.bar_positions.get(p))
            .map_or(BAR_POS_DEFAULT, |s| *s);
        Ok(template.replace("{0}", &inner))
    }

    /// `<m:nary>` -- n-ary operator (sum, integral, product, etc.).
    ///
    /// 结构：`<m:nary><m:naryPr><m:chr m:val="∑"/></m:naryPr>
    /// <m:sub>lower</m:sub><m:sup>upper</m:sup><m:e>base</m:e></m:nary>`
    /// LaTeX 输出：`\sum_{lower}^{upper}base`，并按 `limLoc` 在非默认布局时
    /// 显式输出 `\limits`/`\nolimits`，按 `subHide`/`supHide` 省略上下标。
    fn process_nary<R: BufRead>(&mut self, reader: &mut Reader<R>) -> easydoc_core::Result<String> {
        let mut bo = String::new();
        let mut op_chr = String::new();
        let mut sub = String::new();
        let mut sup = String::new();
        let mut base = String::new();
        let mut pr = PrProps::empty();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "naryPr" => {
                            self.fill_pr(&e, reader, &mut pr)?;
                            if let Some(chr) = &pr.chr {
                                op_chr.clone_from(chr);
                                bo = self
                                    .big_operators
                                    .get(chr.as_str())
                                    .map_or_else(|| chr.clone(), |s| (*s).to_owned());
                            }
                        }
                        "sub" => sub = self.process_children_to_string(reader)?,
                        "sup" => sup = self.process_children_to_string(reader)?,
                        "e" => base = self.process_children_to_string(reader)?,
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    if tag == "naryPr" {
                        apply_pr_attrs(&e, &mut pr);
                        if let Some(chr) = &pr.chr {
                            op_chr.clone_from(chr);
                            bo = self
                                .big_operators
                                .get(chr.as_str())
                                .map_or_else(|| chr.clone(), |s| (*s).to_owned());
                        }
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        // subHide/supHide=1 时省略对应上下标（即使子元素存在内容）
        if pr.sub_hide.as_deref() == Some("1") {
            sub.clear();
        }
        if pr.sup_hide.as_deref() == Some("1") {
            sup.clear();
        }
        // 组装：\sum\limits_{sub}^{sup}base（无上下标时省略）
        let mut result = bo.clone();
        if !sub.is_empty() || !sup.is_empty() {
            // limLoc 仅在偏离该算子 LaTeX 默认布局时显式写出：
            // 求和类默认 \limits，积分类默认 \nolimits；bo 须为已知命令。
            let wants_limits = pr.lim_loc.as_deref() == Some("undOvr");
            if bo.starts_with('\\') && wants_limits == is_integral_operator(&op_chr) {
                if wants_limits {
                    result.push_str("\\limits");
                } else {
                    result.push_str("\\nolimits");
                }
            }
            result.push_str("_{");
            result.push_str(&sub);
            result.push_str("}^{");
            result.push_str(&sup);
            result.push('}');
        } else if bo.starts_with('\\') && !base.is_empty() {
            // 命令后无上下标直接接基式会拼成 \sumx 之类的非法命令，补空格分隔
            result.push(' ');
        }
        result.push_str(&base);
        Ok(result)
    }

    /// `<m:func>` -- function application (sin, cos, etc.).
    fn process_func<R: BufRead>(&mut self, reader: &mut Reader<R>) -> easydoc_core::Result<String> {
        let mut func_name = String::new();
        let mut arg = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "fName" => func_name = self.process_func_name(reader)?,
                        "e" => arg = self.process_children_to_string(reader)?,
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        if func_name.contains(FUNC_PLACE) {
            Ok(func_name.replace(FUNC_PLACE, &arg))
        } else {
            Ok(format!("{func_name}{arg}"))
        }
    }

    /// Process the `<m:fName>` child of `<m:func>`.
    fn process_func_name<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut parts = Vec::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    if tag == "r" {
                        let text = self.process_run(reader)?;
                        if let Some(template) = self.func_names.get(text.as_str()) {
                            parts.push((*template).to_owned());
                        } else {
                            parts.push(text);
                        }
                    } else {
                        let latex = self.process_children_to_string(reader)?;
                        parts.push(latex);
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        let joined = parts.join("");
        if joined.contains(FUNC_PLACE) {
            Ok(joined)
        } else {
            Ok(format!("{joined}{FUNC_PLACE}"))
        }
    }

    /// `<m:groupChr>` -- group character (underbrace, overbrace, etc.).
    fn process_group_chr<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut inner = String::new();
        let mut pr = PrProps::empty();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "e" => inner = self.process_children_to_string(reader)?,
                        "groupChrPr" => {
                            self.fill_pr(&e, reader, &mut pr)?;
                        }
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    if tag == "groupChrPr" {
                        apply_pr_attrs(&e, &mut pr);
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        let template = pr
            .chr
            .as_deref()
            .and_then(|c| self.accents.get(c))
            .map_or(GROUP_CHR_DEFAULT, |s| *s);
        Ok(template.replace("{0}", &inner))
    }

    /// `<m:limLow>` -- lower-limit object.
    fn process_lim_low<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut base = String::new();
        let mut lim = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "e" => base = self.process_children_to_string(reader)?,
                        "lim" => lim = self.process_lim(reader)?,
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        if let Some(template) = self.limit_functions.get(base.as_str()) {
            Ok(template.replace("{lim}", &lim))
        } else {
            Ok(format!("{base}_{{{lim}}}"))
        }
    }

    /// `<m:limUpp>` -- upper-limit object.
    fn process_lim_upp<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut base = String::new();
        let mut lim = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "e" => base = self.process_children_to_string(reader)?,
                        "lim" => lim = self.process_lim(reader)?,
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        Ok(LIM_UPPER_TEMPLATE
            .replace("{lim}", &lim)
            .replace("{text}", &base))
    }

    /// `<m:lim>` -- limit text (used inside limLow / limUpp).
    /// Replaces `\rightarrow` with `\to`.
    fn process_lim<R: BufRead>(&mut self, reader: &mut Reader<R>) -> easydoc_core::Result<String> {
        let inner = self.process_children_to_string(reader)?;
        Ok(inner.replace(LIM_ARROW_FROM, LIM_ARROW_TO))
    }

    /// `<m:m>` -- matrix.
    fn process_matrix<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut rows = Vec::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "mr" => rows.push(self.process_matrix_row(reader)?),
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        let text = rows.join(BRK);
        Ok(MATRIX_TEMPLATE.replace("{text}", &text))
    }

    /// `<m:mr>` -- a single row of a matrix.
    fn process_matrix_row<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut cells = Vec::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    if tag == "e" {
                        cells.push(self.process_children_to_string(reader)?);
                    } else {
                        let _ = self.process_children_to_string(reader)?;
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        Ok(cells.join(ALN))
    }

    /// `<m:eqArr>` -- equation array.
    fn process_eq_arr<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
    ) -> easydoc_core::Result<String> {
        let mut rows = Vec::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    if tag == "e" {
                        rows.push(self.process_children_to_string(reader)?);
                    } else {
                        let _ = self.process_children_to_string(reader)?;
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        let text = rows.join(BRK);
        Ok(ARRAY_TEMPLATE.replace("{text}", &text))
    }

    /// `<m:spre>` -- pre-sub-superscript.
    ///
    /// Structure: `<m:spre><m:e>base</m:e><m:sup>top</m:sup><m:sub>bot</m:sub></m:spre>`
    ///
    /// LaTeX output: `${}^{top}_{bot}base`
    fn process_spre<R: BufRead>(&mut self, reader: &mut Reader<R>) -> easydoc_core::Result<String> {
        let mut base = String::new();
        let mut upper_script = String::new();
        let mut lower_script = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "e" => base = self.process_children_to_string(reader)?,
                        "sup" => upper_script = self.process_children_to_string(reader)?,
                        "sub" => lower_script = self.process_children_to_string(reader)?,
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        // Pre-sub-superscript: ${}^{sup}_{sub}base
        let mut result = String::new();
        if !upper_script.is_empty() || !lower_script.is_empty() {
            result.push_str("{}");
            if !upper_script.is_empty() {
                result.push('^');
                result.push('{');
                result.push_str(&upper_script);
                result.push('}');
            }
            if !lower_script.is_empty() {
                result.push('_');
                result.push('{');
                result.push_str(&lower_script);
                result.push('}');
            }
        }
        result.push_str(&base);
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Property parsing
    // -----------------------------------------------------------------------

    /// Fill a `PrProps` struct from an `xxxPr` element.
    ///
    /// 同时读取两种属性形式：
    /// 1. Pr 元素**自身属性**（`<m:dPr m:begChr="["/>`，真实 Word 的常见形式）；
    /// 2. **子元素带 `m:val`**（`<m:dPr><m:begChr m:val="["/></m:dPr>`，
    ///    本项目自产 OMML 的形式）。
    ///
    /// 对 Start 形式的 Pr，消费到匹配的 end-tag。
    fn fill_pr<R: BufRead>(
        &mut self,
        e: &BytesStart,
        reader: &mut Reader<R>,
        pr: &mut PrProps,
    ) -> easydoc_core::Result<()> {
        apply_pr_attrs(e, pr);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    if PR_FIELDS.contains(&tag.as_str())
                        && let Some(val) = attr_val(&e, "val")
                    {
                        apply_pr_field(pr, &tag, val);
                    }
                    let _ = self.process_children_to_string(reader)?;
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    if PR_FIELDS.contains(&tag.as_str())
                        && let Some(val) = attr_val(&e, "val")
                    {
                        apply_pr_field(pr, &tag, val);
                    }
                }
                Ok(Event::End(_) | Event::Eof) => break,
                Ok(_) => {}
                Err(e) => {
                    return Err(easydoc_core::DocError::Format(format!(
                        "OMML XML parse error: {e}"
                    )));
                }
            }
            buf.clear();
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Free helpers (no `&self` needed)
// ---------------------------------------------------------------------------

/// Check if a start-event is the `<m:oMath>` element.
fn is_omath(e: &BytesStart) -> bool {
    local_name(e) == "oMath"
}

/// Get the local name (stripping `m:` prefix) from a start event.
fn local_name(e: &BytesStart) -> String {
    let qname = e.name();
    let raw = qname.as_ref();
    let s = String::from_utf8_lossy(raw);
    match s.strip_prefix(OMML_NS_PREFIX) {
        Some(stripped) => stripped.to_owned(),
        None => s.into_owned(),
    }
}

/// Handle self-closing (empty) elements.
fn dispatch_empty(stag: &str, e: &BytesStart) -> Option<String> {
    match stag {
        "brk" => Some(BRK.to_owned()),
        _ => attr_val(e, "val"),
    }
}

/// Try to read an attribute value (e.g. `m:val`) from a start event.
fn attr_val(e: &BytesStart, name: &str) -> Option<String> {
    let prefixed = format!("{OMML_NS_PREFIX}{name}");
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref());
        if key == prefixed || key == name {
            return Some(String::from_utf8_lossy(&attr.value).into_owned());
        }
    }
    None
}

/// `<m:xxxPr>` 可解析的属性名全集（自身属性形式与子元素 `m:val` 形式共用）。
const PR_FIELDS: &[&str] = &[
    "chr", "pos", "begChr", "endChr", "type", "sepChr", "limLoc", "subHide", "supHide", "grow",
    "opEmu", "noBreak", "diff",
];

/// n-ary 算子是否为积分类（LaTeX 默认上下限置于角标 `\nolimits`）；
/// 求和/乘积类默认置于上下 `\limits`。
fn is_integral_operator(chr: &str) -> bool {
    matches!(
        chr,
        "\u{222b}" | "\u{222c}" | "\u{222d}" | "\u{222e}" | "\u{222f}" | "\u{2230}"
    )
}

/// 把一个属性字段写入 `PrProps`。
fn apply_pr_field(pr: &mut PrProps, tag: &str, val: String) {
    match tag {
        "chr" => pr.chr = Some(val),
        "pos" => pr.pos = Some(val),
        "begChr" => pr.beg_chr = Some(val),
        "endChr" => pr.end_chr = Some(val),
        "type" => pr.typ = Some(val),
        "sepChr" => pr.sep_chr = Some(val),
        "limLoc" => pr.lim_loc = Some(val),
        "subHide" => pr.sub_hide = Some(val),
        "supHide" => pr.sup_hide = Some(val),
        "grow" => pr.grow = Some(val),
        "opEmu" => pr.op_emu = Some(val),
        "noBreak" => pr.no_break = Some(val),
        "diff" => pr.diff = Some(val),
        _ => {}
    }
}

/// 读取 Pr 元素**自身属性**形式的所有字段（`<m:dPr m:begChr="["/>`）。
fn apply_pr_attrs(e: &BytesStart, pr: &mut PrProps) {
    for tag in PR_FIELDS {
        if let Some(val) = attr_val(e, tag) {
            apply_pr_field(pr, tag, val);
        }
    }
}

/// Capture the run style from an `<m:sty>` / `<m:scr>` element into the
/// corresponding slot. `<m:scr>` wins over `<m:sty>` when both are present.
fn capture_run_style(
    e: &BytesStart,
    tag: &str,
    sty_cmd: &mut Option<&'static str>,
    scr_cmd: &mut Option<&'static str>,
) {
    let Some(val) = attr_val(e, "val") else {
        return;
    };
    let Some(cmd) = run_style_command(&val) else {
        return;
    };
    if tag == "scr" {
        *scr_cmd = Some(cmd);
    } else {
        *sty_cmd = Some(cmd);
    }
}

/// Escape LaTeX special characters in a text string.
///
/// Characters `{ } _ ^ # & $ % ~` are backslash-escaped unless already preceded
/// by a backslash. Existing LaTeX commands are preserved.
fn escape_latex(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            // Preserve existing LaTeX commands -- consume until non-alphanumeric.
            output.push(c);
            while let Some(&next) = chars.peek() {
                if next.is_ascii_alphanumeric() {
                    output.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
        } else if CHARS.contains(&c) {
            output.push('\\');
            output.push(c);
        } else {
            output.push(c);
        }
    }
    output
}

/// Escape a single LaTeX special character (`{ } _ ^ # & $ % ~`).
///
/// 与 `escape_latex` 的区别：`process_run` 中已映射的符号（如 `\mathbf{A}`）
/// 原样输出、不再转义，只有未映射的字符走这里逐个转义。
fn escape_char(c: char) -> String {
    if CHARS.contains(&c) {
        let mut out = String::with_capacity(2);
        out.push('\\');
        out.push(c);
        out
    } else {
        c.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: wrap raw inner XML in a proper `<m:oMath>` envelope.
    fn omath(inner: &str) -> String {
        format!(
            "<m:oMath xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\">\
             {inner}</m:oMath>"
        )
    }

    #[test]
    fn simple_text_run() {
        let xml = omath("<m:r><m:t>abc</m:t></m:r>");
        assert_eq!(convert(&xml).unwrap(), "abc");
    }

    #[test]
    fn text_with_special_chars() {
        // Characters like $, %, _, ^ should be escaped
        let xml = omath("<m:r><m:t>$x</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert!(result.contains("\\$"), "got: {result}");
        assert!(result.contains('x'), "got: {result}");
    }

    #[test]
    fn fraction_simple() {
        let xml = omath(
            "<m:f>\
               <m:num><m:r><m:t>1</m:t></m:r></m:num>\
               <m:den><m:r><m:t>2</m:t></m:r></m:den>\
             </m:f>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\frac{1}{2}");
    }

    #[test]
    fn fraction_with_variables() {
        let xml = omath(
            "<m:f>\
               <m:num><m:r><m:t>a+b</m:t></m:r></m:num>\
               <m:den><m:r><m:t>c</m:t></m:r></m:den>\
             </m:f>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\frac{a+b}{c}");
    }

    #[test]
    fn square_root() {
        let xml = omath(
            "<m:rad>\
               <m:deg/>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
             </m:rad>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\sqrt{x}");
    }

    #[test]
    fn nth_root() {
        let xml = omath(
            "<m:rad>\
               <m:deg><m:r><m:t>3</m:t></m:r></m:deg>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
             </m:rad>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\sqrt[3]{x}");
    }

    #[test]
    fn subscript() {
        let xml = omath(
            "<m:sSub>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
               <m:sub><m:r><m:t>1</m:t></m:r></m:sub>\
             </m:sSub>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, "x_{1}");
    }

    #[test]
    fn superscript() {
        let xml = omath(
            "<m:sSup>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
               <m:sup><m:r><m:t>2</m:t></m:r></m:sup>\
             </m:sSup>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, "x^{2}");
    }

    #[test]
    fn sub_superscript() {
        let xml = omath(
            "<m:sSubSup>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
               <m:sub><m:r><m:t>i</m:t></m:r></m:sub>\
               <m:sup><m:r><m:t>2</m:t></m:r></m:sup>\
             </m:sSubSup>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, "x_{i}^{2}");
    }

    #[test]
    fn pre_sub_superscript() {
        let xml = omath(
            "<m:spre>\
               <m:e><m:r><m:t>A</m:t></m:r></m:e>\
               <m:sup><m:r><m:t>i</m:t></m:r></m:sup>\
               <m:sub><m:r><m:t>j</m:t></m:r></m:sub>\
             </m:spre>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, "{}^{i}_{j}A");
    }

    #[test]
    fn pre_sub_superscript_base_first() {
        // base element comes before sup/sub in the XML
        let xml = omath(
            "<m:spre>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
               <m:sup><m:r><m:t>2</m:t></m:r></m:sup>\
               <m:sub><m:r><m:t>n</m:t></m:r></m:sub>\
             </m:spre>",
        );
        let result = convert(&xml).unwrap();
        // base "x" should appear at the end
        assert!(result.ends_with('x'), "got: {result}");
        assert!(result.contains("^{2}"), "got: {result}");
        assert!(result.contains("_{n}"), "got: {result}");
    }

    #[test]
    fn delimiter_parentheses() {
        let xml = omath(
            "<m:d>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
             </m:d>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\left(x\right)");
    }

    #[test]
    fn nary_sum() {
        let xml = omath(
            "<m:nary>\
               <m:naryPr><m:chr m:val=\"\u{2211}\"/></m:naryPr>\
               <m:sub><m:r><m:t>i=0</m:t></m:r></m:sub>\
               <m:sup><m:r><m:t>n</m:t></m:r></m:sup>\
               <m:e><m:r><m:t>x_i</m:t></m:r></m:e>\
             </m:nary>",
        );
        let result = convert(&xml).unwrap();
        assert!(result.contains(r"\sum"), "got: {result}");
        assert!(result.contains("i=0"), "got: {result}");
        assert!(result.contains('n'), "got: {result}");
    }

    #[test]
    fn matrix_2x2() {
        let xml = omath(
            "<m:m>\
               <m:mr>\
                 <m:e><m:r><m:t>a</m:t></m:r></m:e>\
                 <m:e><m:r><m:t>b</m:t></m:r></m:e>\
               </m:mr>\
               <m:mr>\
                 <m:e><m:r><m:t>c</m:t></m:r></m:e>\
                 <m:e><m:r><m:t>d</m:t></m:r></m:e>\
               </m:mr>\
             </m:m>",
        );
        let result = convert(&xml).unwrap();
        assert!(result.contains(r"\begin{matrix}"), "got: {result}");
        assert!(result.contains(r"\end{matrix}"), "got: {result}");
        assert!(result.contains('a'), "got: {result}");
        assert!(result.contains('d'), "got: {result}");
        assert!(result.contains('&'), "got: {result}");
        assert!(result.contains("\\\\"), "got: {result}");
    }

    #[test]
    fn accent_hat() {
        let xml = omath(
            "<m:acc>\
               <m:accPr><m:chr m:val=\"\u{0302}\"/></m:accPr>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
             </m:acc>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\hat{x}");
    }

    #[test]
    fn bar_overline() {
        let xml = omath(
            "<m:bar>\
               <m:barPr><m:pos m:val=\"top\"/></m:barPr>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
             </m:bar>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\overline{x}");
    }

    #[test]
    fn func_sin() {
        let xml = omath(
            "<m:func>\
               <m:fName><m:r><m:t>sin</m:t></m:r></m:fName>\
               <m:e><m:r><m:t>x</m:t></m:r></m:e>\
             </m:func>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\sin(x)");
    }

    #[test]
    fn quadratic_formula() {
        // x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}
        let xml = omath(
            "<m:r><m:t>x=</m:t></m:r>\
             <m:f>\
               <m:num>\
                 <m:r><m:t>-b</m:t></m:r>\
                 <m:sSup>\
                   <m:e><m:rad><m:deg/><m:e><m:r><m:t>b</m:t></m:r></m:e></m:rad></m:e>\
                   <m:sup><m:r><m:t>2</m:t></m:r></m:sup>\
                 </m:sSup>\
               </m:num>\
               <m:den><m:r><m:t>2a</m:t></m:r></m:den>\
             </m:f>",
        );
        let result = convert(&xml).unwrap();
        assert!(result.contains(r"\frac"), "got: {result}");
        assert!(result.contains(r"\sqrt"), "got: {result}");
        assert!(result.contains("2a"), "got: {result}");
    }

    #[test]
    fn empty_omath_returns_empty() {
        let xml =
            "<m:oMath xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"/>";
        assert_eq!(convert(xml).unwrap(), "");
    }

    #[test]
    fn escape_latex_preserves_commands() {
        let result = escape_latex(r"\alpha + \beta");
        assert_eq!(result, r"\alpha + \beta");
    }

    #[test]
    fn escape_latex_escapes_special_chars() {
        let result = escape_latex("$x%y");
        assert!(result.contains("\\$"), "got: {result}");
        assert!(result.contains("\\%"), "got: {result}");
    }

    #[test]
    fn greek_alpha_symbol() {
        // Mathematical italic alpha U+1D6FC
        let xml = omath("<m:r><m:t>\u{1d6fc}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, "\\alpha ");
    }

    #[test]
    fn infinity_symbol() {
        let xml = omath("<m:r><m:t>\u{221e}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, "\\infty ");
    }

    #[test]
    fn lim_low() {
        let xml = omath(
            "<m:limLow>\
               <m:e><m:r><m:t>lim</m:t></m:r></m:e>\
               <m:lim><m:r><m:t>x\\rightarrow 0</m:t></m:r></m:lim>\
             </m:limLow>",
        );
        let result = convert(&xml).unwrap();
        assert!(result.contains(r"\lim"), "got: {result}");
        assert!(result.contains(r"\to"), "got: {result}");
    }

    #[test]
    fn group_chr_underbrace() {
        let xml = omath(
            "<m:groupChr>\
               <m:groupChrPr><m:chr m:val=\"\u{23df}\"/></m:groupChrPr>\
               <m:e><m:r><m:t>x+y</m:t></m:r></m:e>\
             </m:groupChr>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\underbrace{x+y}");
    }
    // === 批量补充：常见 OMML 数学结构（0.1.0 测试扩充） ===

    #[test]
    fn fraction_simple_basic() {
        let xml = omath(
            "<m:f><m:num><m:r><m:t>1</m:t></m:r></m:num><m:den><m:r><m:t>2</m:t></m:r></m:den></m:f>",
        );
        let result = convert(&xml).unwrap();
        assert!(result.contains("frac"), "got: {result}");
        assert!(
            result.contains('1') && result.contains('2'),
            "got: {result}"
        );
    }

    #[test]
    fn subscript_simple() {
        let xml = omath(
            "<m:sSub><m:e><m:r><m:t>x</m:t></m:r></m:e><m:sub><m:r><m:t>i</m:t></m:r></m:sub></m:sSub>",
        );
        let result = convert(&xml).unwrap();
        assert!(
            result.contains('x') && result.contains('i'),
            "got: {result}"
        );
        assert!(
            result.contains('_'),
            "expected subscript underscore, got: {result}"
        );
    }

    #[test]
    fn superscript_simple() {
        let xml = omath(
            "<m:sSup><m:e><m:r><m:t>x</m:t></m:r></m:e><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup>",
        );
        let result = convert(&xml).unwrap();
        assert!(
            result.contains('x') && result.contains('2'),
            "got: {result}"
        );
        assert!(
            result.contains('^'),
            "expected superscript caret, got: {result}"
        );
    }

    #[test]
    fn sub_sup_combined() {
        let xml = omath(
            "<m:sSubSup><m:e><m:r><m:t>x</m:t></m:r></m:e><m:sub><m:r><m:t>a</m:t></m:r></m:sub><m:sup><m:r><m:t>b</m:t></m:r></m:sup></m:sSubSup>",
        );
        let result = convert(&xml).unwrap();
        assert!(result.contains('x'), "got: {result}");
        assert!(
            result.contains('a') && result.contains('b'),
            "got: {result}"
        );
    }

    #[test]
    fn radical_simple_basic() {
        let xml = omath(
            "<m:rad><m:deg><m:r><m:t>3</m:t></m:r></m:deg><m:e><m:r><m:t>x</m:t></m:r></m:e></m:rad>",
        );
        let result = convert(&xml).unwrap();
        assert!(
            result.contains("sqrt") || result.contains("root"),
            "got: {result}"
        );
        assert!(result.contains('x'), "got: {result}");
    }

    #[test]
    fn nary_summation() {
        let xml = omath(
            "<m:nary><m:naryPr><m:chr m:val=\"∑\"/></m:naryPr><m:sub><m:r><m:t>i=1</m:t></m:r></m:sub><m:sup><m:r><m:t>n</m:t></m:r></m:sup><m:e><m:r><m:t>x_i</m:t></m:r></m:e></m:nary>",
        );
        let result = convert(&xml).unwrap();
        assert!(
            result.contains("sum") || result.contains("∑"),
            "got: {result}"
        );
    }

    #[test]
    fn delimiter_paren_with_content_basic() {
        let xml = omath(
            "<m:d><m:dPr><m:begChr m:val=\"(\"/><m:endChr m:val=\")\"/></m:dPr><m:e><m:r><m:t>x</m:t></m:r></m:e></m:d>",
        );
        let result = convert(&xml).unwrap();
        assert!(
            result.contains("left") || result.contains('('),
            "got: {result}"
        );
        assert!(result.contains('x'), "got: {result}");
    }

    #[test]
    fn function_name_application() {
        let xml = omath(
            "<m:func><m:fName><m:r><m:t>sin</m:t></m:r></m:fName><m:e><m:r><m:t>x</m:t></m:r></m:e></m:func>",
        );
        let result = convert(&xml).unwrap();
        assert!(result.contains("sin"), "got: {result}");
        assert!(result.contains('x'), "got: {result}");
    }

    #[test]
    fn matrix_2x2_basic() {
        let xml = omath(
            "<m:m><m:mr><m:e><m:r><m:t>a</m:t></m:r></m:e><m:e><m:r><m:t>b</m:t></m:r></m:e></m:mr><m:mr><m:e><m:r><m:t>c</m:t></m:r></m:e><m:e><m:r><m:t>d</m:t></m:r></m:e></m:mr></m:m>",
        );
        let result = convert(&xml).unwrap();
        assert!(
            result.contains('a') && result.contains('d'),
            "got: {result}"
        );
        assert!(
            result.contains('&') || result.contains("\\\\"),
            "matrix should have separators, got: {result}"
        );
    }

    #[test]
    fn nested_fraction_in_fraction() {
        let xml = omath(
            "<m:f><m:num><m:f><m:num><m:r><m:t>1</m:t></m:r></m:num><m:den><m:r><m:t>2</m:t></m:r></m:den></m:f></m:num><m:den><m:r><m:t>3</m:t></m:r></m:den></m:f>",
        );
        let result = convert(&xml).unwrap();
        assert!(
            result.matches("frac").count() >= 2,
            "expected nested frac, got: {result}"
        );
    }

    #[test]
    fn empty_run_produces_nothing() {
        let xml = omath("<m:r><m:t></m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert!(result.trim().is_empty(), "got: {result}");
    }

    #[test]
    fn mixed_text_and_math() {
        let xml = omath(
            "<m:r><m:t>x</m:t></m:r><m:f><m:num><m:r><m:t>1</m:t></m:r></m:num><m:den><m:r><m:t>y</m:t></m:r></m:den></m:f>",
        );
        let result = convert(&xml).unwrap();
        assert!(result.contains('x'), "got: {result}");
        assert!(result.contains('y'), "got: {result}");
    }

    #[test]
    fn phantom_hides_content() {
        // m:phant 的内容不可见但占据空间 → \phantom{...}（参考 litchi）
        let xml = omath("<m:phant><m:e><m:r><m:t>x</m:t></m:r></m:e></m:phant>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\phantom{x}");
    }

    #[test]
    fn border_box_wraps_content() {
        // m:borderBox → \boxed{...}（参考 litchi）
        let xml = omath("<m:borderBox><m:e><m:r><m:t>x</m:t></m:r></m:e></m:borderBox>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\boxed{x}");
    }

    #[test]
    fn bold_run_style() {
        // m:sty val="b" → \mathbf{...}
        let xml = omath("<m:r><m:rPr><m:sty m:val=\"b\"/></m:rPr><m:t>x</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathbf{x}");
    }

    #[test]
    fn italic_run_style() {
        let xml = omath("<m:r><m:rPr><m:sty m:val=\"i\"/></m:rPr><m:t>f</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathit{f}");
    }

    #[test]
    fn double_struck_scr_style() {
        // m:scr val="ds"（双空）→ \mathbb{...}
        let xml = omath("<m:r><m:rPr><m:scr m:val=\"ds\"/></m:rPr><m:t>x</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathbb{x}");
    }

    #[test]
    fn scr_takes_precedence_over_sty() {
        let xml = omath(
            "<m:r><m:rPr><m:sty m:val=\"b\"/><m:scr m:val=\"ds\"/></m:rPr><m:t>z</m:t></m:r>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathbb{z}");
    }

    #[test]
    fn bold_math_unicode_codepoint() {
        // 数学粗体 A（U+1D400）与粗体 a（U+1D41A）
        let xml = omath("<m:r><m:t>\u{1d400}\u{1d41a}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathbf{A}\mathbf{a}");
    }

    #[test]
    fn bold_greek_codepoint() {
        // 数学粗体 α（U+1D6C2）
        let xml = omath("<m:r><m:t>\u{1d6c2}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\boldsymbol{\alpha}");
    }

    #[test]
    fn bmp_greek_letter() {
        // BMP 区段 α（U+03B1）
        let xml = omath("<m:r><m:t>\u{03b1}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, "\\alpha ");
    }

    #[test]
    fn bmp_double_struck_r() {
        // BMP 双空 ℝ（U+211D）
        let xml = omath("<m:r><m:t>\u{211d}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathbb{R}");
    }

    #[test]
    fn symbol_mapping_survives_special_char_escaping() {
        // 映射值含花括号时不得被二次转义；未映射的 _ 仍按字面转义
        let xml = omath("<m:r><m:t>\u{211d}_x</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathbb{R}\_x");
    }

    #[test]
    fn plain_text_special_chars_still_escaped() {
        // 未映射字符仍需转义
        let xml = omath("<m:r><m:t>a%b</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, "a\\%b");
    }

    // ===== A1: 属性形式 Pr（真实 Word 产出 `<m:dPr m:begChr="["/>`）=====

    #[test]
    fn delimiter_attr_form_pr() {
        // 真实 Word 的空元素属性形式：<m:dPr m:begChr="[" m:endChr="]"/>
        let xml = omath(
            "<m:d><m:dPr m:begChr=\"[\" m:endChr=\"]\"/><m:e><m:r><m:t>x</m:t></m:r></m:e></m:d>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\left[x\right]");
    }

    #[test]
    fn delimiter_attr_form_start_tag() {
        // Start 标签自身带属性（非 Empty 形式）
        let xml = omath(
            "<m:d><m:dPr m:begChr=\"{\" m:endChr=\"}\"></m:dPr><m:e><m:r><m:t>x</m:t></m:r></m:e></m:d>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\left\{x\right\}");
    }

    #[test]
    fn nary_attr_form_pr() {
        // 属性形式的 naryPr：chr/limLoc 直接从属性读取
        let xml = omath(
            "<m:nary><m:naryPr m:chr=\"∫\" m:limLoc=\"subSup\"/>\
             <m:sub><m:r><m:t>0</m:t></m:r></m:sub>\
             <m:sup><m:r><m:t>1</m:t></m:r></m:sup>\
             <m:e><m:r><m:t>x</m:t></m:r></m:e></m:nary>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\int_{0}^{1}x");
    }

    #[test]
    fn fraction_attr_form_pr() {
        // 属性形式的 fPr：type 直接从属性读取
        let xml = omath(
            "<m:f><m:fPr m:type=\"noBar\"/><m:num><m:r><m:t>a</m:t></m:r></m:num>\
             <m:den><m:r><m:t>b</m:t></m:r></m:den></m:f>",
        );
        let result = convert(&xml).unwrap();
        assert!(
            result.contains("genfrac"),
            "noBar 应输出 \\genfrac：{result}"
        );
    }

    // ===== A2: m:box / boxPr =====

    #[test]
    fn box_passthrough_content() {
        let xml = omath("<m:box><m:e><m:r><m:t>x+y</m:t></m:r></m:e></m:box>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, "x+y");
    }

    #[test]
    fn box_op_emu_wraps_mathop() {
        let xml =
            omath("<m:box><m:boxPr m:opEmu=\"1\"/><m:e><m:r><m:t>max</m:t></m:r></m:e></m:box>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathop{max}");
    }

    // ===== A3: m:d 多 e 堆叠 =====

    #[test]
    fn delimiter_stacked_multiple_e() {
        let xml = omath(
            "<m:d><m:dPr m:begChr=\"(\" m:endChr=\")\"/>\
             <m:e><m:r><m:t>a</m:t></m:r></m:e>\
             <m:e><m:r><m:t>b</m:t></m:r></m:e>\
             <m:e><m:r><m:t>c</m:t></m:r></m:e></m:d>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(
            result, r"\left(\begin{matrix}a\\b\\c\end{matrix}\right)",
            "多 e 应上下堆叠：{result}"
        );
    }

    // ===== A4: m:naryPr limLoc / subHide / supHide =====

    #[test]
    fn nary_lim_loc_undovr_int_emits_limits() {
        // 积分默认 \nolimits；显式 undOvr 时输出 \limits
        let xml = omath(
            "<m:nary><m:naryPr m:chr=\"∫\" m:limLoc=\"undOvr\"/>\
             <m:sub><m:r><m:t>0</m:t></m:r></m:sub>\
             <m:sup><m:r><m:t>1</m:t></m:r></m:sup>\
             <m:e><m:r><m:t>x</m:t></m:r></m:e></m:nary>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\int\limits_{0}^{1}x");
    }

    #[test]
    fn nary_sub_sup_hide_omits_scripts() {
        let xml = omath(
            "<m:nary><m:naryPr m:chr=\"∑\" m:subHide=\"1\" m:supHide=\"1\"/>\
             <m:sub><m:r><m:t>i</m:t></m:r></m:sub>\
             <m:sup><m:r><m:t>n</m:t></m:r></m:sup>\
             <m:e><m:r><m:t>x</m:t></m:r></m:e></m:nary>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(
            result, r"\sum x",
            "隐藏上下标后不应输出 _{{...}}^{{...}}：{result}"
        );
    }

    #[test]
    fn nary_sum_default_no_limits_marker() {
        // 求和默认 \limits，不显式输出，保持简洁
        let xml = omath(
            "<m:nary><m:naryPr><m:chr m:val=\"∑\"/><m:limLoc m:val=\"undOvr\"/></m:naryPr>\
             <m:sub><m:r><m:t>i</m:t></m:r></m:sub>\
             <m:sup><m:r><m:t>n</m:t></m:r></m:sup>\
             <m:e><m:r><m:t>x</m:t></m:r></m:e></m:nary>",
        );
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\sum_{i}^{n}x");
    }

    // ===== A5: 字母表补全 =====

    #[test]
    fn bold_italic_alphabet() {
        // 数学粗斜体 A（U+1D468）与 a（U+1D482）
        let xml = omath("<m:r><m:t>\u{1d468}\u{1d482}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\boldsymbol{A}\boldsymbol{a}");
    }

    #[test]
    fn sans_serif_alphabet() {
        // 数学无衬线 A（U+1D5A0）
        let xml = omath("<m:r><m:t>\u{1d5a0}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathsf{A}");
    }

    #[test]
    fn monospace_alphabet() {
        // 数学等宽 a（U+1D68A）
        let xml = omath("<m:r><m:t>\u{1d68a}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathtt{a}");
    }

    #[test]
    fn double_struck_uppercase() {
        // 数学双空 S（U+1D54A）
        let xml = omath("<m:r><m:t>\u{1d54a}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathbb{S}");
    }

    #[test]
    fn math_digits() {
        // 数学粗体数字 0（U+1D7CE）与双空 1（U+1D7D9）
        let xml = omath("<m:r><m:t>\u{1d7ce}\u{1d7d9}</m:t></m:r>");
        let result = convert(&xml).unwrap();
        assert_eq!(result, r"\mathbf{0}\mathbb{1}");
    }
}
