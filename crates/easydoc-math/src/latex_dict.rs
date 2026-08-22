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
///
/// Keys are `String` because the mathematical alphabets (bold, double-struck,
/// script, fraktur) are generated programmatically over contiguous Unicode
/// ranges. Values may contain LaTeX commands with braces (e.g. `\mathbf{A}`),
/// which are inserted verbatim and not re-escaped.
pub(crate) fn build_text_symbols() -> HashMap<String, String> {
    // 常量表统一声明在函数顶部，避免"语句后声明条目"的歧义。
    // BMP 希腊字母：部分 Word 文档直接使用 BMP 区段码点而非数学斜体扩展区段；
    // 无 LaTeX 命令的大写希腊字母映射为对应拉丁字母（与大写罗马体一致）。
    const BMP_GREEK: &[(&str, &str)] = &[
        ("\u{0391}", "A"),
        ("\u{0392}", "B"),
        ("\u{0393}", "\\Gamma "),
        ("\u{0394}", "\\Delta "),
        ("\u{0395}", "E"),
        ("\u{0396}", "Z"),
        ("\u{0397}", "H"),
        ("\u{0398}", "\\Theta "),
        ("\u{0399}", "I"),
        ("\u{039a}", "K"),
        ("\u{039b}", "\\Lambda "),
        ("\u{039c}", "M"),
        ("\u{039d}", "N"),
        ("\u{039e}", "\\Xi "),
        ("\u{039f}", "O"),
        ("\u{03a0}", "\\Pi "),
        ("\u{03a1}", "P"),
        ("\u{03a3}", "\\Sigma "),
        ("\u{03a4}", "T"),
        ("\u{03a5}", "\\Upsilon "),
        ("\u{03a6}", "\\Phi "),
        ("\u{03a7}", "X"),
        ("\u{03a8}", "\\Psi "),
        ("\u{03a9}", "\\Omega "),
        ("\u{03b1}", "\\alpha "),
        ("\u{03b2}", "\\beta "),
        ("\u{03b3}", "\\gamma "),
        ("\u{03b4}", "\\delta "),
        ("\u{03b5}", "\\epsilon "),
        ("\u{03b6}", "\\zeta "),
        ("\u{03b7}", "\\eta "),
        ("\u{03b8}", "\\theta "),
        ("\u{03b9}", "\\iota "),
        ("\u{03ba}", "\\kappa "),
        ("\u{03bb}", "\\lambda "),
        ("\u{03bc}", "\\mu "),
        ("\u{03bd}", "\\nu "),
        ("\u{03be}", "\\xi "),
        ("\u{03bf}", "o"),
        ("\u{03c0}", "\\pi "),
        ("\u{03c1}", "\\rho "),
        ("\u{03c2}", "\\varsigma "),
        ("\u{03c3}", "\\sigma "),
        ("\u{03c4}", "\\tau "),
        ("\u{03c5}", "\\upsilon "),
        ("\u{03c6}", "\\phi "),
        ("\u{03c7}", "\\chi "),
        ("\u{03c8}", "\\psi "),
        ("\u{03c9}", "\\omega "),
    ];
    // BMP 运算符与常用符号（对照 litchi 的 UNICODE_TO_LATEX 表）。
    const BMP_SYMBOLS: &[(&str, &str)] = &[
        ("\u{2211}", "\\sum "),
        ("\u{220f}", "\\prod "),
        ("\u{222b}", "\\int "),
        ("\u{222e}", "\\oint "),
        ("\u{221a}", "\\surd "),
        ("\u{2202}", "\\partial "),
        ("\u{2207}", "\\nabla "),
        ("\u{2205}", "\\emptyset "),
        ("\u{2200}", "\\forall "),
        ("\u{2203}", "\\exists "),
        ("\u{2204}", "\\nexists "),
        ("\u{2234}", "\\therefore "),
        ("\u{2235}", "\\because "),
        ("\u{2282}", "\\subset "),
        ("\u{2283}", "\\supset "),
        ("\u{2286}", "\\subseteq "),
        ("\u{2287}", "\\supseteq "),
        ("\u{2229}", "\\cap "),
        ("\u{222a}", "\\cup "),
        ("\u{2248}", "\\approx "),
        ("\u{223c}", "\\sim "),
        ("\u{221d}", "\\propto "),
        ("\u{2261}", "\\equiv "),
        ("\u{21d2}", "\\Rightarrow "),
        ("\u{21d0}", "\\Leftarrow "),
        ("\u{21d4}", "\\Leftrightarrow "),
        ("\u{21d1}", "\\Uparrow "),
        ("\u{21d3}", "\\Downarrow "),
        ("\u{00d7}", "\\times "),
        ("\u{00f7}", "\\div "),
        ("\u{22c5}", "\\cdot "),
        ("\u{2217}", "\\ast "),
        ("\u{2218}", "\\circ "),
        ("\u{2227}", "\\wedge "),
        ("\u{2228}", "\\vee "),
        ("\u{2295}", "\\oplus "),
        ("\u{2297}", "\\otimes "),
        ("\u{2299}", "\\odot "),
        ("\u{25b3}", "\\triangle "),
        ("\u{25a1}", "\\square "),
        ("\u{25c7}", "\\diamond "),
        ("\u{2020}", "\\dagger "),
        ("\u{2021}", "\\ddagger "),
        ("\u{203e}", "\\bar "),
        ("\u{02c6}", "\\hat "),
        ("\u{02dc}", "\\tilde "),
        ("\u{02c7}", "\\check "),
        ("\u{00b4}", "\\acute "),
        ("\u{02d9}", "\\dot "),
        ("\u{00a8}", "\\ddot "),
        ("\u{02d8}", "\\breve "),
        ("\u{00b0}", "\\degree "),
        ("\u{2032}", "'"),
        ("\u{2033}", "''"),
        ("\u{2034}", "'''"),
        ("\u{2135}", "\\aleph "),
        ("\u{2136}", "\\beth "),
        ("\u{2137}", "\\gimel "),
        ("\u{2138}", "\\daleth "),
        ("\u{210f}", "\\hbar "),
        ("\u{2113}", "\\ell "),
        ("\u{2118}", "\\wp "),
        ("\u{211c}", "\\Re "),
        ("\u{2111}", "\\Im "),
        ("\u{2115}", "\\mathbb{N}"),
        ("\u{2124}", "\\mathbb{Z}"),
        ("\u{211a}", "\\mathbb{Q}"),
        ("\u{211d}", "\\mathbb{R}"),
        ("\u{2102}", "\\mathbb{C}"),
        ("\u{210d}", "\\mathbb{H}"),
        ("\u{2119}", "\\mathbb{P}"),
    ];
    // 粗体希腊（U+1D6A8..U+1D6C1 大写、U+1D6C2..U+1D6DB 小写，各 26 项）。
    // 大写区段含 theta 符号与 nabla，小写区段以 partial 结尾，均按码点顺序一一对应。
    const BOLD_GREEK_UPPER: [&str; 26] = [
        "A",
        "B",
        "\\Gamma",
        "\\Delta",
        "E",
        "Z",
        "H",
        "\\Theta",
        "I",
        "K",
        "\\Lambda",
        "M",
        "N",
        "\\Xi",
        "O",
        "\\Pi",
        "P",
        "\\vartheta",
        "\\Sigma",
        "T",
        "\\Upsilon",
        "\\Phi",
        "X",
        "\\Psi",
        "\\Omega",
        "\\nabla",
    ];
    const BOLD_GREEK_LOWER: [&str; 26] = [
        "\\alpha",
        "\\beta",
        "\\gamma",
        "\\delta",
        "\\epsilon",
        "\\zeta",
        "\\eta",
        "\\theta",
        "\\iota",
        "\\kappa",
        "\\lambda",
        "\\mu",
        "\\nu",
        "\\xi",
        "o",
        "\\pi",
        "\\rho",
        "\\varsigma",
        "\\sigma",
        "\\tau",
        "\\upsilon",
        "\\phi",
        "\\chi",
        "\\psi",
        "\\omega",
        "\\partial",
    ];

    let mut m: HashMap<String, String> = HashMap::new();
    let mut put = |k: &str, v: &str| {
        m.insert(k.to_owned(), v.to_owned());
    };

    // ===== Greek letters (Mathematical Italic, U+1D6FC..U+1D71B) =====
    // The OMML font uses these codepoints for Greek in math mode.
    put("\u{1d6fc}", "\\alpha ");
    put("\u{1d6fd}", "\\beta ");
    put("\u{1d6fe}", "\\gamma ");
    put("\u{1d6ff}", "\\delta ");
    put("\u{1d700}", "\\epsilon ");
    put("\u{1d701}", "\\zeta ");
    put("\u{1d702}", "\\eta ");
    put("\u{1d703}", "\\theta ");
    put("\u{1d704}", "\\iota ");
    put("\u{1d705}", "\\kappa ");
    put("\u{1d706}", "\\lambda ");
    put("\u{1d707}", "\\mu ");
    put("\u{1d708}", "\\nu ");
    put("\u{1d709}", "\\xi ");
    put("\u{1d70a}", "o");
    put("\u{1d70b}", "\\pi ");
    put("\u{1d70c}", "\\rho ");
    put("\u{1d70d}", "\\varsigma ");
    put("\u{1d70e}", "\\sigma ");
    put("\u{1d70f}", "\\tau ");
    put("\u{1d710}", "\\upsilon ");
    put("\u{1d711}", "\\phi ");
    put("\u{1d712}", "\\chi ");
    put("\u{1d713}", "\\psi ");
    put("\u{1d714}", "\\omega ");
    put("\u{1d715}", "\\partial ");
    put("\u{1d716}", "\\varepsilon ");
    put("\u{1d717}", "\\vartheta ");
    put("\u{1d718}", "\\varkappa ");
    put("\u{1d719}", "\\varphi ");
    put("\u{1d71a}", "\\varrho ");
    put("\u{1d71b}", "\\varpi ");

    // ===== BMP 希腊字母 =====
    for (key, value) in BMP_GREEK {
        put(key, value);
    }

    // ===== BMP 运算符与常用符号 =====
    for (key, value) in BMP_SYMBOLS {
        put(key, value);
    }

    // ===== Relation symbols =====
    put("\u{2190}", "\\leftarrow ");
    put("\u{2191}", "\\uparrow ");
    put("\u{2192}", "\\rightarrow ");
    put("\u{2193}", "\\downarrow ");
    put("\u{2194}", "\\leftrightarrow ");
    put("\u{2195}", "\\updownarrow ");
    put("\u{2196}", "\\nwarrow ");
    put("\u{2197}", "\\nearrow ");
    put("\u{2198}", "\\searrow ");
    put("\u{2199}", "\\swarrow ");
    put("\u{22ee}", "\\vdots ");
    put("\u{22ef}", "\\cdots ");
    put("\u{22f0}", "\\adots ");
    put("\u{22f1}", "\\ddots ");
    put("\u{2260}", "\\ne ");
    put("\u{2264}", "\\leq ");
    put("\u{2265}", "\\geq ");
    put("\u{2266}", "\\leqq ");
    put("\u{2267}", "\\geqq ");
    put("\u{2268}", "\\lneqq ");
    put("\u{2269}", "\\gneqq ");
    put("\u{226a}", "\\ll ");
    put("\u{226b}", "\\gg ");
    put("\u{2208}", "\\in ");
    put("\u{2209}", "\\notin ");
    put("\u{220b}", "\\ni ");
    put("\u{220c}", "\\nni ");

    // ===== Ordinary symbols =====
    put("\u{221e}", "\\infty ");

    // ===== Binary relations =====
    put("\u{00b1}", "\\pm ");
    put("\u{2213}", "\\mp ");

    // ===== 数学粗体拉丁字母（U+1D400..U+1D433）=====
    // Word 以粗体渲染的变量使用该区段码点。
    for cp in 0x1d400_u32..0x1d41a {
        let key = char::from_u32(cp).expect("valid bold Latin uppercase");
        let letter = char::from_u32(cp - 0x1d400 + 'A' as u32).expect("valid ASCII letter");
        put(&key.to_string(), &format!("\\mathbf{{{letter}}}"));
    }
    for cp in 0x1d41a_u32..0x1d434 {
        let key = char::from_u32(cp).expect("valid bold Latin lowercase");
        let letter = char::from_u32(cp - 0x1d41a + 'a' as u32).expect("valid ASCII letter");
        put(&key.to_string(), &format!("\\mathbf{{{letter}}}"));
    }

    // ===== 数学粗体希腊字母（U+1D6A8..U+1D6DB）=====
    for (offset, latex) in BOLD_GREEK_UPPER.iter().enumerate() {
        let key = char::from_u32(0x1d6a8_u32 + offset as u32).expect("valid bold Greek uppercase");
        put(&key.to_string(), &format!("\\boldsymbol{{{latex}}}"));
    }
    for (offset, latex) in BOLD_GREEK_LOWER.iter().enumerate() {
        let key = char::from_u32(0x1d6c2_u32 + offset as u32).expect("valid bold Greek lowercase");
        put(&key.to_string(), &format!("\\boldsymbol{{{latex}}}"));
    }

    // ===== 数学双空/手写/哥特小写字母（U+1D552..U+1D56B 等，连续区段）=====
    for cp in 0x1d552_u32..0x1d56c {
        let key = char::from_u32(cp).expect("valid double-struck lowercase");
        let letter = char::from_u32(cp - 0x1d552 + 'a' as u32).expect("valid ASCII letter");
        put(&key.to_string(), &format!("\\mathbb{{{letter}}}"));
    }
    for cp in 0x1d4b6_u32..0x1d4d0 {
        let key = char::from_u32(cp).expect("valid script lowercase");
        let letter = char::from_u32(cp - 0x1d4b6 + 'a' as u32).expect("valid ASCII letter");
        put(&key.to_string(), &format!("\\mathcal{{{letter}}}"));
    }
    for cp in 0x1d51e_u32..0x1d538 {
        let key = char::from_u32(cp).expect("valid fraktur lowercase");
        let letter = char::from_u32(cp - 0x1d51e + 'a' as u32).expect("valid ASCII letter");
        put(&key.to_string(), &format!("\\mathfrak{{{letter}}}"));
    }

    // ===== Italic Latin uppercase (U+1D434..U+1D44D) =====
    put("\u{1d434}", "A");
    put("\u{1d435}", "B");
    put("\u{1d436}", "C");
    put("\u{1d437}", "D");
    put("\u{1d438}", "E");
    put("\u{1d439}", "F");
    put("\u{1d43a}", "G");
    put("\u{1d43b}", "H");
    put("\u{1d43c}", "I");
    put("\u{1d43d}", "J");
    put("\u{1d43e}", "K");
    put("\u{1d43f}", "L");
    put("\u{1d440}", "M");
    put("\u{1d441}", "N");
    put("\u{1d442}", "O");
    put("\u{1d443}", "P");
    put("\u{1d444}", "Q");
    put("\u{1d445}", "R");
    put("\u{1d446}", "S");
    put("\u{1d447}", "T");
    put("\u{1d448}", "U");
    put("\u{1d449}", "V");
    put("\u{1d44a}", "W");
    put("\u{1d44b}", "X");
    put("\u{1d44c}", "Y");
    put("\u{1d44d}", "Z");

    // ===== Italic Latin lowercase (U+1D44E..U+1D467) =====
    put("\u{1d44e}", "a");
    put("\u{1d44f}", "b");
    put("\u{1d450}", "c");
    put("\u{1d451}", "d");
    put("\u{1d452}", "e");
    put("\u{1d453}", "f");
    put("\u{1d454}", "g");
    put("\u{1d456}", "i");
    put("\u{1d457}", "j");
    put("\u{1d458}", "k");
    put("\u{1d459}", "l");
    put("\u{1d45a}", "m");
    put("\u{1d45b}", "n");
    put("\u{1d45c}", "o");
    put("\u{1d45d}", "p");
    put("\u{1d45e}", "q");
    put("\u{1d45f}", "r");
    put("\u{1d460}", "s");
    put("\u{1d461}", "t");
    put("\u{1d462}", "u");
    put("\u{1d463}", "v");
    put("\u{1d464}", "w");
    put("\u{1d465}", "x");
    put("\u{1d466}", "y");
    put("\u{1d467}", "z");

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

/// Map an OMML run style value (`<m:sty>` or `<m:scr>` `val` attribute) to a
/// LaTeX style command prefix.
///
/// `m:sty` 取值：`p`（正体）、`b`（粗体）、`i`（斜体）、`bi`（粗斜体）；
/// `m:scr` 取值：`nor`/`b`/`i`/`bi`/`ds`（双空）/`sc`（手写）/`fr`（哥特）等。
/// 两表的取值有重叠，统一映射到 LaTeX 命令，转换器在 run 级别包裹内容。
pub(crate) fn run_style_command(val: &str) -> Option<&'static str> {
    match val {
        "p" | "nor" | "normal" | "roman" => Some("\\mathrm"),
        "b" | "bold" => Some("\\mathbf"),
        "i" | "italic" => Some("\\mathit"),
        "bi" | "bold-italic" => Some("\\boldsymbol"),
        "ds" | "double-struck" => Some("\\mathbb"),
        // 粗体手写/哥特没有独立的 LaTeX 命令，退化为普通手写/哥特
        "sc" | "script" | "bsc" | "bold-script" => Some("\\mathcal"),
        "fr" | "fraktur" | "bfr" | "bold-fraktur" => Some("\\mathfrak"),
        // 无衬线各变体共用 \mathsf
        "ss"
        | "sans-serif"
        | "ssb"
        | "sans-serif-bold"
        | "ssi"
        | "sans-serif-italic"
        | "ssbi"
        | "sans-serif-bold-italic" => Some("\\mathsf"),
        "m" | "monospace" => Some("\\mathtt"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_non_empty_and_keyed_by_combining_chars() {
        let accents = build_accents();
        assert!(!accents.is_empty());
        // 每个键都是 Unicode 组合字符（非 ASCII 字母），值为 LaTeX 模板
        for (key, value) in &accents {
            assert!(!key.is_empty());
            assert!(
                value.contains("{0}"),
                "accent template should contain {{0}}: {value}"
            );
        }
    }

    #[test]
    fn accents_known_mappings() {
        let accents = build_accents();
        assert_eq!(accents.get("\u{0301}").copied(), Some("\\acute{{0}}"));
        assert_eq!(accents.get("\u{0307}").copied(), Some("\\dot{{0}}"));
        assert_eq!(accents.get("\u{20d7}").copied(), Some("\\vec{{0}}"));
    }

    #[test]
    fn big_operators_known() {
        let ops = build_big_operators();
        assert!(!ops.is_empty());
    }

    #[test]
    fn text_symbols_non_empty() {
        let symbols = build_text_symbols();
        assert!(!symbols.is_empty());
    }

    #[test]
    fn text_symbols_values_non_empty() {
        let symbols = build_text_symbols();
        for value in symbols.values() {
            assert!(!value.is_empty(), "symbol value should not be empty");
        }
    }

    #[test]
    fn func_names_known_math_functions() {
        let funcs = build_func_names();
        assert!(!funcs.is_empty());
        // 常见数学函数应存在
        assert!(funcs.contains_key("sin") || funcs.values().any(|v| v.contains("sin")));
        assert!(funcs.contains_key("cos") || funcs.values().any(|v| v.contains("cos")));
    }

    #[test]
    fn fraction_styles_known() {
        let styles = build_fraction_styles();
        assert!(!styles.is_empty());
    }

    #[test]
    fn limit_functions_non_empty() {
        let limits = build_limit_functions();
        assert!(!limits.is_empty());
    }

    #[test]
    fn bar_positions_known() {
        let bars = build_bar_positions();
        assert!(!bars.is_empty());
        assert!(bars.contains_key("top"));
        assert!(bars.contains_key("bot"));
    }

    #[test]
    fn constants_defined() {
        assert!(CHARS.contains(&'{'));
        assert!(CHARS.contains(&'%'));
        assert!(!CHARS.contains(&'a')); // 普通字符不需要转义
        assert_eq!(ALN, "&");
        assert_eq!(BRK, "\\\\");
        assert_eq!(FUNC_PLACE, "{fe}");
    }

    #[test]
    fn all_dicts_keys_unique() {
        // 各字典内部键不重复（HashMap 天然保证，此处验证构建不 panic 且规模合理）
        let accents = build_accents();
        let ops = build_big_operators();
        let symbols = build_text_symbols();
        let funcs = build_func_names();
        assert!(accents.len() > 20);
        assert!(!ops.is_empty());
        assert!(symbols.len() > 50);
        assert!(funcs.len() > 10);
    }

    #[test]
    fn text_symbols_bmp_greek() {
        let symbols = build_text_symbols();
        assert_eq!(
            symbols.get("\u{03b1}").map(String::as_str),
            Some("\\alpha ")
        );
        assert_eq!(
            symbols.get("\u{03b3}").map(String::as_str),
            Some("\\gamma ")
        );
        assert_eq!(
            symbols.get("\u{03a9}").map(String::as_str),
            Some("\\Omega ")
        );
        assert_eq!(
            symbols.get("\u{03c3}").map(String::as_str),
            Some("\\sigma ")
        );
    }

    #[test]
    fn text_symbols_bmp_operators() {
        let symbols = build_text_symbols();
        assert_eq!(symbols.get("\u{2211}").map(String::as_str), Some("\\sum "));
        assert_eq!(symbols.get("\u{222b}").map(String::as_str), Some("\\int "));
        assert_eq!(
            symbols.get("\u{2248}").map(String::as_str),
            Some("\\approx ")
        );
        assert_eq!(
            symbols.get("\u{211d}").map(String::as_str),
            Some("\\mathbb{R}")
        );
    }

    #[test]
    fn text_symbols_bold_alphabets() {
        let symbols = build_text_symbols();
        // 数学粗体 A（U+1D400）与粗体 α（U+1D6C2）
        assert_eq!(
            symbols.get("\u{1d400}").map(String::as_str),
            Some("\\mathbf{A}")
        );
        assert_eq!(
            symbols.get("\u{1d41a}").map(String::as_str),
            Some("\\mathbf{a}")
        );
        assert_eq!(
            symbols.get("\u{1d6c2}").map(String::as_str),
            Some("\\boldsymbol{\\alpha}")
        );
        // 双空/手写/哥特小写首尾
        assert_eq!(
            symbols.get("\u{1d552}").map(String::as_str),
            Some("\\mathbb{a}")
        );
        assert_eq!(
            symbols.get("\u{1d56b}").map(String::as_str),
            Some("\\mathbb{z}")
        );
        assert_eq!(
            symbols.get("\u{1d4b6}").map(String::as_str),
            Some("\\mathcal{a}")
        );
        assert_eq!(
            symbols.get("\u{1d51e}").map(String::as_str),
            Some("\\mathfrak{a}")
        );
    }

    #[test]
    fn omicron_maps_to_plain_letter() {
        // \omicron 不是标准 LaTeX 命令，应映射为普通字母 o
        let symbols = build_text_symbols();
        assert_eq!(symbols.get("\u{1d70a}").map(String::as_str), Some("o"));
        assert_eq!(symbols.get("\u{03bf}").map(String::as_str), Some("o"));
    }

    #[test]
    fn run_style_known_commands() {
        assert_eq!(run_style_command("b"), Some("\\mathbf"));
        assert_eq!(run_style_command("i"), Some("\\mathit"));
        assert_eq!(run_style_command("bi"), Some("\\boldsymbol"));
        assert_eq!(run_style_command("ds"), Some("\\mathbb"));
        assert_eq!(run_style_command("sc"), Some("\\mathcal"));
        assert_eq!(run_style_command("p"), Some("\\mathrm"));
        assert_eq!(run_style_command("unknown"), None);
    }
}
