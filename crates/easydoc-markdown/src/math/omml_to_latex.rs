//! OMML (Office Math Markup Language) to LaTeX converter.
//!
//! Recursive descent over `<m:oMath>` XML, producing a LaTeX string suitable
//! for inclusion in Markdown `$...$` or `$$...$$`.
//!
//! Ported from the Python `markitdown` project (`omml.py`), which itself was
//! adapted from [dwml](https://github.com/xiilei/dwml).

use std::collections::HashMap;
use std::io::BufRead;

use quick_xml::Reader;
use quick_xml::events::BytesStart;
use quick_xml::events::Event;

use super::latex_dict::{
    self, ACCENT_DEFAULT, ALN, ARRAY_TEMPLATE, BAR_POS_DEFAULT, BRK, CHARS, DELIMITER_DEFAULT_LEFT,
    DELIMITER_DEFAULT_RIGHT, DELIMITER_NULL, DELIMITER_TEMPLATE, FRACTION_DEFAULT, FUNC_PLACE,
    GROUP_CHR_DEFAULT, LIM_ARROW_FROM, LIM_ARROW_TO, LIM_UPPER_TEMPLATE, MATRIX_TEMPLATE,
    RADICAL_DEFAULT_TEMPLATE, RADICAL_DEG_TEMPLATE, SUB_TEMPLATE, SUP_TEMPLATE,
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
    text_symbols: HashMap<&'static str, &'static str>,
    big_operators: HashMap<&'static str, &'static str>,
    accents: HashMap<&'static str, &'static str>,
    func_names: HashMap<&'static str, &'static str>,
    fraction_styles: HashMap<&'static str, &'static str>,
    limit_functions: HashMap<&'static str, &'static str>,
    bar_positions: HashMap<&'static str, &'static str>,
}

/// Parsed properties from an `<m:xxxPr>` element.
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
}

impl PrProps {
    fn empty() -> Self {
        Self {
            chr: None,
            pos: None,
            beg_chr: None,
            end_chr: None,
            typ: None,
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
            "oMath" | "e" | "num" | "den" | "deg" | "box" | "sSub" | "sSup" | "sSubSup" => {
                let latex = self.process_children_to_string(reader)?;
                Ok(Some(latex))
            }
            "spre" => {
                let latex = self.process_spre(reader)?;
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

    // -----------------------------------------------------------------------
    // OMML element handlers
    // -----------------------------------------------------------------------

    /// `<m:r>` -- text run. Maps each character through the symbol table and
    /// escapes LaTeX special characters.
    ///
    /// Consumes through the closing `</m:r>` tag (depth-tracked).
    fn process_run<R: BufRead>(&mut self, reader: &mut Reader<R>) -> easydoc_core::Result<String> {
        let mut text_parts = Vec::new();
        let mut buf = Vec::new();
        let mut in_text = false;
        let mut depth = 1_u32;
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    let tag = local_name(&e);
                    if tag == "t" {
                        in_text = true;
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
                            None => mapped.push_str(s),
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
        Ok(escape_latex(&text_parts.join("")))
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
                            self.fill_pr(reader, &mut pr)?;
                        }
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
                            self.fill_pr(reader, &mut pr)?;
                        }
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
        let left = pr.beg_chr.as_deref().map_or(DELIMITER_DEFAULT_LEFT, |v| {
            self.text_symbols.get(v).map_or(v, |s| *s)
        });
        let right = pr.end_chr.as_deref().map_or(DELIMITER_DEFAULT_RIGHT, |v| {
            self.text_symbols.get(v).map_or(v, |s| *s)
        });
        let left = if left.is_empty() {
            DELIMITER_NULL
        } else {
            &escape_latex(left)
        };
        let right = if right.is_empty() {
            DELIMITER_NULL
        } else {
            &escape_latex(right)
        };
        let text = texts.join("");
        let result = DELIMITER_TEMPLATE
            .replace("{left}", left)
            .replace("{text}", &text)
            .replace("{right}", right);
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
                            self.fill_pr(reader, &mut pr)?;
                        }
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
                            self.fill_pr(reader, &mut pr)?;
                        }
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
        let template = pr
            .pos
            .as_deref()
            .and_then(|p| self.bar_positions.get(p))
            .map_or(BAR_POS_DEFAULT, |s| *s);
        Ok(template.replace("{0}", &inner))
    }

    /// `<m:nary>` -- n-ary operator (sum, integral, product, etc.).
    fn process_nary<R: BufRead>(&mut self, reader: &mut Reader<R>) -> easydoc_core::Result<String> {
        let mut parts = Vec::new();
        let mut bo = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "naryPr" => {
                            let mut pr = PrProps::empty();
                            self.fill_pr(reader, &mut pr)?;
                            if let Some(chr) = &pr.chr {
                                bo = self
                                    .big_operators
                                    .get(chr.as_str())
                                    .map_or_else(|| chr.clone(), |s| (*s).to_owned());
                            }
                        }
                        _ => {
                            parts.push(self.process_children_to_string(reader)?);
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
        Ok(format!("{bo}{}", parts.join("")))
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
                            self.fill_pr(reader, &mut pr)?;
                        }
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

    /// Fill a `PrProps` struct from the children of an `xxxPr` element.
    /// Consumes through the matching end-tag.
    fn fill_pr<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
        pr: &mut PrProps,
    ) -> easydoc_core::Result<()> {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "chr" | "pos" | "begChr" | "endChr" | "type" => {
                            if let Some(val) = attr_val(&e, "val") {
                                match tag.as_str() {
                                    "chr" => pr.chr = Some(val),
                                    "pos" => pr.pos = Some(val),
                                    "begChr" => pr.beg_chr = Some(val),
                                    "endChr" => pr.end_chr = Some(val),
                                    "type" => pr.typ = Some(val),
                                    _ => {}
                                }
                            }
                            let _ = self.process_children_to_string(reader)?;
                        }
                        _ => {
                            let _ = self.process_children_to_string(reader)?;
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let tag = local_name(&e);
                    match tag.as_str() {
                        "chr" | "pos" | "begChr" | "endChr" | "type" => {
                            if let Some(val) = attr_val(&e, "val") {
                                match tag.as_str() {
                                    "chr" => pr.chr = Some(val),
                                    "pos" => pr.pos = Some(val),
                                    "begChr" => pr.beg_chr = Some(val),
                                    "endChr" => pr.end_chr = Some(val),
                                    "type" => pr.typ = Some(val),
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
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
}
