//! OMML-to-LaTeX symbol and structure mapping tables.
//!
//! Ported from the Python `markitdown` project (`latex_dict.py`), which itself was
//! adapted from [dwml](https://github.com/xiilei/dwml).

use std::collections::HashMap;

/// Characters that must be backslash-escaped in LaTeX math mode.
pub(crate) const CHARS: &[char] = &['{', '}', '_', '^', '#', '&', '$', '%', '~'];

/// The column-alignment separator used inside matrix environments.
pub(crate) const ALN: &str = "&";

/// The row-break command for multi-row math environments.
pub(crate) const BRK: &str = "\\\\";

/// Placeholder inserted by function-name processing and later replaced with the argument.
pub(crate) const FUNC_PLACE: &str = "{fe}";

/// Builds the accent / combining-character mapping table (top and bottom accents).
///
/// Keys are Unicode combining characters; values are LaTeX templates with `{0}` as the
/// placeholder for the accented base character.
pub(crate) fn build_accents() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // --- Top accents ---
    m.insert("\u{0300}", "\\grave{{0}}");
    m.insert("\u{0301}", "\\acute{{0}}");
    m.insert("\u{0302}", "\\hat{{0}}");
    m.insert("\u{0303}", "\\tilde{{0}}");
    m.insert("\u{0304}", "\\bar{{0}}");
    m.insert("\u{0305}", "\\overbar{{0}}");
    m.insert("\u{0306}", "\\breve{{0}}");
    m.insert("\u{0307}", "\\dot{{0}}");
    m.insert("\u{0308}", "\\ddot{{0}}");
    m.insert("\u{0309}", "\\ovhook{{0}}");
    m.insert("\u{030a}", "\\ocirc{{0}}");
    m.insert("\u{030c}", "\\check{{0}}");
    m.insert("\u{0310}", "\\candra{{0}}");
    m.insert("\u{0312}", "\\oturnedcomma{{0}}");
    m.insert("\u{0315}", "\\ocommatopright{{0}}");
    m.insert("\u{031a}", "\\droang{{0}}");
    m.insert("\u{0338}", "\\not{{0}}");
    m.insert("\u{20d0}", "\\leftharpoonaccent{{0}}");
    m.insert("\u{20d1}", "\\rightharpoonaccent{{0}}");
    m.insert("\u{20d2}", "\\vertoverlay{{0}}");
    m.insert("\u{20d6}", "\\overleftarrow{{0}}");
    m.insert("\u{20d7}", "\\vec{{0}}");
    m.insert("\u{20db}", "\\dddot{{0}}");
    m.insert("\u{20dc}", "\\ddddot{{0}}");
    m.insert("\u{20e1}", "\\overleftrightarrow{{0}}");
    m.insert("\u{20e7}", "\\annuity{{0}}");
    m.insert("\u{20e9}", "\\widebridgeabove{{0}}");
    m.insert("\u{20f0}", "\\asteraccent{{0}}");
    // --- Bottom accents ---
    m.insert("\u{0330}", "\\wideutilde{{0}}");
    m.insert("\u{0331}", "\\underbar{{0}}");
    m.insert("\u{20e8}", "\\threeunderdot{{0}}");
    m.insert("\u{20ec}", "\\underrightharpoondown{{0}}");
    m.insert("\u{20ed}", "\\underleftharpoondown{{0}}");
    m.insert("\u{20ee}", "\\underleftarrow{{0}}");
    m.insert("\u{20ef}", "\\underrightarrow{{0}}");
    // --- Over grouping ---
    m.insert("\u{23b4}", "\\overbracket{{0}}");
    m.insert("\u{23dc}", "\\overparen{{0}}");
    m.insert("\u{23de}", "\\overbrace{{0}}");
    // --- Under grouping ---
    m.insert("\u{23b5}", "\\underbracket{{0}}");
    m.insert("\u{23dd}", "\\underparen{{0}}");
    m.insert("\u{23df}", "\\underbrace{{0}}");
    m
}

/// Builds the big-operator mapping table.
///
/// Keys are Unicode characters for operators like summation, product, integral, etc.
pub(crate) fn build_big_operators() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("\u{2140}", "\\Bbbsum");
    m.insert("\u{220f}", "\\prod");
    m.insert("\u{2210}", "\\coprod");
    m.insert("\u{2211}", "\\sum");
    m.insert("\u{222b}", "\\int");
    m.insert("\u{22c0}", "\\bigwedge");
    m.insert("\u{22c1}", "\\bigvee");
    m.insert("\u{22c2}", "\\bigcap");
    m.insert("\u{22c3}", "\\bigcup");
    m.insert("\u{2a00}", "\\bigodot");
    m.insert("\u{2a01}", "\\bigoplus");
    m.insert("\u{2a02}", "\\bigotimes");
    m
}

/// Builds the main text-symbol mapping table.
///
/// Covers Greek letters (mathematical italic Unicode block), relation symbols,
/// arrows, ordinary symbols, binary operators, and Latin letters (italic math).
pub(crate) fn build_text_symbols() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();

    // ===== Greek letters (Mathematical Italic, U+1D6FC..U+1D71B) =====
    // The OMML font uses these codepoints for Greek in math mode.
    m.insert("\u{1d6fc}", "\\alpha ");
    m.insert("\u{1d6fd}", "\\beta ");
    m.insert("\u{1d6fe}", "\\gamma ");
    m.insert("\u{1d6ff}", "\\delta ");
    m.insert("\u{1d700}", "\\epsilon ");
    m.insert("\u{1d701}", "\\zeta ");
    m.insert("\u{1d702}", "\\eta ");
    m.insert("\u{1d703}", "\\theta ");
    m.insert("\u{1d704}", "\\iota ");
    m.insert("\u{1d705}", "\\kappa ");
    m.insert("\u{1d706}", "\\lambda ");
    m.insert("\u{1d707}", "\\mu ");
    m.insert("\u{1d708}", "\\nu ");
    m.insert("\u{1d709}", "\\xi ");
    m.insert("\u{1d70a}", "\\omicron ");
    m.insert("\u{1d70b}", "\\pi ");
    m.insert("\u{1d70c}", "\\rho ");
    m.insert("\u{1d70d}", "\\varsigma ");
    m.insert("\u{1d70e}", "\\sigma ");
    m.insert("\u{1d70f}", "\\tau ");
    m.insert("\u{1d710}", "\\upsilon ");
    m.insert("\u{1d711}", "\\phi ");
    m.insert("\u{1d712}", "\\chi ");
    m.insert("\u{1d713}", "\\psi ");
    m.insert("\u{1d714}", "\\omega ");
    m.insert("\u{1d715}", "\\partial ");
    m.insert("\u{1d716}", "\\varepsilon ");
    m.insert("\u{1d717}", "\\vartheta ");
    m.insert("\u{1d718}", "\\varkappa ");
    m.insert("\u{1d719}", "\\varphi ");
    m.insert("\u{1d71a}", "\\varrho ");
    m.insert("\u{1d71b}", "\\varpi ");

    // ===== Relation symbols =====
    m.insert("\u{2190}", "\\leftarrow ");
    m.insert("\u{2191}", "\\uparrow ");
    m.insert("\u{2192}", "\\rightarrow ");
    m.insert("\u{2193}", "\\downarrow ");
    m.insert("\u{2194}", "\\leftrightarrow ");
    m.insert("\u{2195}", "\\updownarrow ");
    m.insert("\u{2196}", "\\nwarrow ");
    m.insert("\u{2197}", "\\nearrow ");
    m.insert("\u{2198}", "\\searrow ");
    m.insert("\u{2199}", "\\swarrow ");
    m.insert("\u{22ee}", "\\vdots ");
    m.insert("\u{22ef}", "\\cdots ");
    m.insert("\u{22f0}", "\\adots ");
    m.insert("\u{22f1}", "\\ddots ");
    m.insert("\u{2260}", "\\ne ");
    m.insert("\u{2264}", "\\leq ");
    m.insert("\u{2265}", "\\geq ");
    m.insert("\u{2266}", "\\leqq ");
    m.insert("\u{2267}", "\\geqq ");
    m.insert("\u{2268}", "\\lneqq ");
    m.insert("\u{2269}", "\\gneqq ");
    m.insert("\u{226a}", "\\ll ");
    m.insert("\u{226b}", "\\gg ");
    m.insert("\u{2208}", "\\in ");
    m.insert("\u{2209}", "\\notin ");
    m.insert("\u{220b}", "\\ni ");
    m.insert("\u{220c}", "\\nni ");

    // ===== Ordinary symbols =====
    m.insert("\u{221e}", "\\infty ");

    // ===== Binary relations =====
    m.insert("\u{00b1}", "\\pm ");
    m.insert("\u{2213}", "\\mp ");

    // ===== Italic Latin uppercase (U+1D434..U+1D44D) =====
    m.insert("\u{1d434}", "A");
    m.insert("\u{1d435}", "B");
    m.insert("\u{1d436}", "C");
    m.insert("\u{1d437}", "D");
    m.insert("\u{1d438}", "E");
    m.insert("\u{1d439}", "F");
    m.insert("\u{1d43a}", "G");
    m.insert("\u{1d43b}", "H");
    m.insert("\u{1d43c}", "I");
    m.insert("\u{1d43d}", "J");
    m.insert("\u{1d43e}", "K");
    m.insert("\u{1d43f}", "L");
    m.insert("\u{1d440}", "M");
    m.insert("\u{1d441}", "N");
    m.insert("\u{1d442}", "O");
    m.insert("\u{1d443}", "P");
    m.insert("\u{1d444}", "Q");
    m.insert("\u{1d445}", "R");
    m.insert("\u{1d446}", "S");
    m.insert("\u{1d447}", "T");
    m.insert("\u{1d448}", "U");
    m.insert("\u{1d449}", "V");
    m.insert("\u{1d44a}", "W");
    m.insert("\u{1d44b}", "X");
    m.insert("\u{1d44c}", "Y");
    m.insert("\u{1d44d}", "Z");

    // ===== Italic Latin lowercase (U+1D44E..U+1D467) =====
    m.insert("\u{1d44e}", "a");
    m.insert("\u{1d44f}", "b");
    m.insert("\u{1d450}", "c");
    m.insert("\u{1d451}", "d");
    m.insert("\u{1d452}", "e");
    m.insert("\u{1d453}", "f");
    m.insert("\u{1d454}", "g");
    m.insert("\u{1d456}", "i");
    m.insert("\u{1d457}", "j");
    m.insert("\u{1d458}", "k");
    m.insert("\u{1d459}", "l");
    m.insert("\u{1d45a}", "m");
    m.insert("\u{1d45b}", "n");
    m.insert("\u{1d45c}", "o");
    m.insert("\u{1d45d}", "p");
    m.insert("\u{1d45e}", "q");
    m.insert("\u{1d45f}", "r");
    m.insert("\u{1d460}", "s");
    m.insert("\u{1d461}", "t");
    m.insert("\u{1d462}", "u");
    m.insert("\u{1d463}", "v");
    m.insert("\u{1d464}", "w");
    m.insert("\u{1d465}", "x");
    m.insert("\u{1d466}", "y");
    m.insert("\u{1d467}", "z");

    // TODO: Additional symbols from markitdown latex_dict.py not yet ported:
    // - Bold math alphabet (U+1D400..U+1D433)
    // - Script / calligraphic alphabet
    // - Double-struck alphabet
    // - Fraktur alphabet
    // - Additional relation and operator symbols

    m
}

/// Builds the trigonometric / transcendental function name mapping table.
///
/// Keys are plain function names as they appear in OMML `<m:t>`; values are LaTeX
/// command templates containing the `{fe}` placeholder for the function argument.
pub(crate) fn build_func_names() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("sin", "\\sin({fe})");
    m.insert("cos", "\\cos({fe})");
    m.insert("tan", "\\tan({fe})");
    m.insert("arcsin", "\\arcsin({fe})");
    m.insert("arccos", "\\arccos({fe})");
    m.insert("arctan", "\\arctan({fe})");
    m.insert("arccot", "\\arccot({fe})");
    m.insert("sinh", "\\sinh({fe})");
    m.insert("cosh", "\\cosh({fe})");
    m.insert("tanh", "\\tanh({fe})");
    m.insert("coth", "\\coth({fe})");
    m.insert("sec", "\\sec({fe})");
    m.insert("csc", "\\csc({fe})");
    m
}

/// Fraction type variants.
///
/// The key is the OMML `m:type` attribute value on a `<m:fPr>` element.
/// The value is a LaTeX template with `{num}` and `{den}` placeholders.
pub(crate) fn build_fraction_styles() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("bar", "\\frac{{num}}{{den}}");
    m.insert("skw", "^{{num}}/_{{den}}");
    m.insert("noBar", "\\genfrac{{}}{{0pt}}{{}}{{num}}{{den}}");
    m.insert("lin", "{{num}}/{{den}}");
    m
}

/// Default fraction template (bar style, i.e. standard `\frac`).
pub(crate) const FRACTION_DEFAULT: &str = "\\frac{{num}}{{den}}";

/// Delimiter template with `\left` / `\right`.
pub(crate) const DELIMITER_TEMPLATE: &str = "\\left{left}{text}\\right{right}";

/// Default delimiters: left=`(`, right=`)`, null=`.`.
pub(crate) const DELIMITER_DEFAULT_LEFT: &str = "(";
pub(crate) const DELIMITER_DEFAULT_RIGHT: &str = ")";
pub(crate) const DELIMITER_NULL: &str = ".";

/// Radical template with an explicit degree.
pub(crate) const RADICAL_DEG_TEMPLATE: &str = "\\sqrt[{deg}]{{text}}";

/// Radical template without degree (square root).
pub(crate) const RADICAL_DEFAULT_TEMPLATE: &str = "\\sqrt{{text}}";

/// Array / equation-array template.
pub(crate) const ARRAY_TEMPLATE: &str = "\\begin{array}{c}{text}\\end{array}";

/// Limit-function mapping (lower-limit objects).
pub(crate) fn build_limit_functions() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("lim", "\\lim_{{lim}}");
    m.insert("max", "\\max_{{lim}}");
    m.insert("min", "\\min_{{lim}}");
    m
}

/// The OMML limit arrow that gets replaced with `\to`.
pub(crate) const LIM_ARROW_FROM: &str = "\\rightarrow";
pub(crate) const LIM_ARROW_TO: &str = "\\to";

/// Upper-limit overset template.
pub(crate) const LIM_UPPER_TEMPLATE: &str = "\\overset{{lim}}{{text}}";

/// Matrix environment template.
pub(crate) const MATRIX_TEMPLATE: &str = "\\begin{matrix}{text}\\end{matrix}";

/// Subscript template.
pub(crate) const SUB_TEMPLATE: &str = "_{{0}}";

/// Superscript template.
pub(crate) const SUP_TEMPLATE: &str = "^{{0}}";

/// Default accent value (hat).
pub(crate) const ACCENT_DEFAULT: &str = "\\hat{{0}}";

/// Default group-character value (underbrace).
pub(crate) const GROUP_CHR_DEFAULT: &str = "\\underbrace{{0}}";

/// Default bar position (overline).
pub(crate) const BAR_POS_DEFAULT: &str = "\\overline{{0}}";

/// Position mapping for bar elements.
pub(crate) fn build_bar_positions() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("top", "\\overline{{0}}");
    m.insert("bot", "\\underline{{0}}");
    m
}
