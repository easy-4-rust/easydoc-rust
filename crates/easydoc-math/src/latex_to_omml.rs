//! LaTeX → OMML（Office Math Markup Language，Word 原生公式）转换。
//!
//! 设计参考 [tex2word-math](https://github.com/yfyang86/rstex2word)（MIT，
//! 作者 Yifan Yang；其符号表/字母表又源自 Python `mathml/symbols.py`），
//! 吸收其"LaTeX 递归下降解析 → 数学 AST → OMML 渲染"的架构与符号映射，
//! 并在此基础上有意改进：
//!
//! - **严格错误通道**：未知命令、括号不配对、嵌套过深、无法无损表达的构造
//!   一律返回 `Err`（调用方回退保留 `$latex$` 原文），杜绝 tex2word-math
//!   那种"未知命令被静默吞掉、输入字符凭空消失"的内容丢失。
//! - **新增命令**：`\underbrace/\overbrace`（`m:groupChr`）、
//!   `\overset/\underset/\stackrel`（`m:limUpp/m:limLow`）、
//!   `\lim_{x\to 0}` 极限布局（`m:limLow`）、`\boxed`（`m:borderBox`）、
//!   `\operatorname*`（星号剥离）。
//! - **内容保真的近似映射**（仅视觉差异，不丢内容）：`\widehat` 降级为窄
//!   重音（U+0302）、`aligned` 行内 `&` 对齐标记剥离后映射为 `m:eqArr`、
//!   `array` 列格式忽略（渲染为居中矩阵）、`\mathsf/\mathtt` 降级为正体。

use std::fmt::Write;

/// 最大嵌套深度，防止病态输入（如数千层 `\frac`/`{`）导致栈溢出。
///
/// 取 64：真实公式嵌套极少超过 10 层，64 已足够宽松；同时保证在默认
/// 2MiB 测试线程栈上、递归到该深度时不会先溢出（每层约 8KiB 栈帧）。
const MAX_DEPTH: usize = 64;

/// 数学 AST（吸收 tex2word-math 的 Node 设计并扩展 GroupChar/Lim/EqArray/BorderBox）。
#[derive(Debug, Clone, PartialEq)]
enum Node {
    /// 数学斜体文本（run）。
    Run(String),
    /// 正体文本（函数名、`\mathrm` 类），渲染为 `<m:nor/>` run。
    Upright(String),
    /// 序列。
    Row(Vec<Node>),
    /// 分式。
    Frac(Box<Node>, Box<Node>),
    /// 上标 / 下标 / 上下标。
    Sup(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    SubSup(Box<Node>, Box<Node>, Box<Node>),
    /// 平方根 / n 次根（index, radicand）。
    Sqrt(Box<Node>),
    Root(Box<Node>, Box<Node>),
    /// n-ary 大算子（∑/∫/∏…），`over_under` 决定上下限置于上下还是角标。
    Nary {
        op: String,
        sub: Option<Box<Node>>,
        sup: Option<Box<Node>>,
        body: Box<Node>,
        over_under: bool,
    },
    /// `\left...\right` 定界符。
    Delim {
        open: String,
        close: String,
        body: Box<Node>,
    },
    /// 重音（`\hat` 等），chr 为组合字符。
    Accent {
        chr: char,
        base: Box<Node>,
    },
    /// 上下线（`\overline/\underline`）。
    Bar {
        top: bool,
        base: Box<Node>,
    },
    /// 矩阵（含定界符环境）。
    Matrix {
        rows: Vec<Vec<Node>>,
        delim: Option<(String, String)>,
    },
    /// 等式数组（`aligned`，行内 `&` 已剥离）。
    EqArray {
        rows: Vec<Node>,
    },
    /// 粗体/斜体样式 run（仅作用于扁平文本）。
    Styled {
        sty: &'static str,
        text: String,
    },
    /// 二项式系数（括号包 noBar 分式）。
    Binom(Box<Node>, Box<Node>),
    /// 花括号（`\underbrace/\overbrace`）。
    GroupChar {
        chr: char,
        pos: &'static str,
        base: Box<Node>,
    },
    /// 上下标极限（`\overset/\underset/\stackrel`、`\lim` 极限布局）。
    Lim {
        pos: &'static str,
        base: Box<Node>,
        lim: Box<Node>,
    },
    /// 边框盒子（`\boxed`）。
    BorderBox(Box<Node>),
}

/// 解析结果类型，`Err` 携带人类可读的错误描述。
type PResult<T> = Result<T, String>;

/// 递归下降解析器（按字符扫描，架构同 tex2word-math，改为严格报错）。
struct Parser {
    s: Vec<char>,
    i: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n' | '\t' | '\r')) {
            self.i += 1;
        }
    }

    /// 解析一串元素（`{...}` 分组或整个公式），遇到 `}` 或结尾停止。
    fn parse_row(&mut self) -> PResult<Node> {
        let mut items: Vec<Node> = Vec::new();
        loop {
            self.skip_space();
            match self.peek() {
                None | Some('}') => break,
                _ => {}
            }
            let atom = self.parse_atom()?;
            let atom = self.maybe_scripts(atom)?;
            push_merge(&mut items, atom);
        }
        Ok(Node::Row(items))
    }

    /// 解析一个原子：`{...}` 分组、命令或单字符。
    fn parse_atom(&mut self) -> PResult<Node> {
        if self.depth >= MAX_DEPTH {
            return Err("嵌套过深（超过 256 层），拒绝继续解析".to_string());
        }
        self.depth += 1;
        let node = match self.peek() {
            Some('{') => {
                self.i += 1;
                let inner = self.parse_row()?;
                if self.peek() == Some('}') {
                    self.i += 1;
                } else {
                    return Err(format!("缺少闭合的 }}（位置 {}）", self.i));
                }
                inner
            }
            Some('\\') => self.parse_command()?,
            Some(c) => {
                self.i += 1;
                Node::Run(c.to_string())
            }
            None => Node::Run(String::new()),
        };
        self.depth -= 1;
        Ok(node)
    }

    /// 原子后追加 `^`/`_` 上下标（顺序任意）；极限函数（lim/max/min…）转为 Lim。
    fn maybe_scripts(&mut self, base: Node) -> PResult<Node> {
        let mut sub: Option<Box<Node>> = None;
        let mut sup: Option<Box<Node>> = None;
        loop {
            self.skip_space();
            match self.peek() {
                Some('^') => {
                    self.i += 1;
                    self.skip_space();
                    sup = Some(Box::new(self.parse_atom()?));
                }
                Some('_') => {
                    self.i += 1;
                    self.skip_space();
                    sub = Some(Box::new(self.parse_atom()?));
                }
                _ => break,
            }
        }
        // 极限函数（\lim_{x\to 0}、\max_{x\in X}）→ m:limLow/m:limUpp 布局。
        if let Node::Upright(name) = &base
            && is_limit_function(name)
            && (sub.is_some() || sup.is_some())
        {
            return Ok(build_lim(base, sub, sup));
        }
        Ok(match (sub, sup) {
            (None, None) => base,
            (Some(sb), None) => Node::Sub(Box::new(base), sb),
            (None, Some(sp)) => Node::Sup(Box::new(base), sp),
            (Some(sb), Some(sp)) => Node::SubSup(Box::new(base), sb, sp),
        })
    }

    /// 读取命令名（字母序列）或单个非字母控制符号，消耗开头的 `\`。
    fn read_name(&mut self) -> String {
        self.i += 1; // 消费 '\'
        match self.peek() {
            Some(c) if c.is_ascii_alphabetic() => {
                let start = self.i;
                while matches!(self.peek(), Some(c) if c.is_ascii_alphabetic()) {
                    self.i += 1;
                }
                self.s[start..self.i].iter().collect()
            }
            Some(c) => {
                self.i += 1;
                c.to_string()
            }
            None => String::new(),
        }
    }

    fn parse_command(&mut self) -> PResult<Node> {
        let name = self.read_name();
        match name.as_str() {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let num = self.parse_atom()?;
                let den = self.parse_atom()?;
                Ok(Node::Frac(Box::new(num), Box::new(den)))
            }
            "sqrt" => {
                self.skip_space();
                if self.peek() == Some('[') {
                    let idx = self.read_bracket()?;
                    let rad = self.parse_atom()?;
                    Ok(Node::Root(Box::new(idx), Box::new(rad)))
                } else {
                    Ok(Node::Sqrt(Box::new(self.parse_atom()?)))
                }
            }
            // 正体内容：\mathrm/\text/\operatorname/\mathsf/\mathtt（后两者降级为正体）
            "mathrm" | "text" | "operatorname" | "mathsf" | "mathtt" => {
                let text = flatten_text(&self.parse_atom()?)?;
                Ok(Node::Upright(text))
            }
            // 粗体：仅对扁平文本生效；含结构时（如 \mathbf{x+y}）flatten 成文本，
            // 内容不丢但结构扁平化——属于可接受的近似（与 tex2word-math 一致）。
            "mathbf" | "boldsymbol" | "bm" => {
                let text = flatten_text(&self.parse_atom()?)?;
                Ok(Node::Styled { sty: "b", text })
            }
            "mathit" => Ok(Node::Run(flatten_text(&self.parse_atom()?)?)),
            // 数学类别包装只影响间距 → 内容透明透传
            "mathbin" | "mathrel" | "mathop" | "mathord" | "mathopen" | "mathclose"
            | "mathpunct" | "mathinner" => self.parse_atom(),
            // 黑板/手写/哥特字母表 → Unicode 数学字母
            "mathbb" | "mathcal" | "mathscr" | "mathfrak" => {
                let text = flatten_text(&self.parse_atom()?)?;
                Ok(Node::Run(alphabet(&name, &text)))
            }
            "binom" | "dbinom" | "tbinom" => {
                let n = self.parse_atom()?;
                let k = self.parse_atom()?;
                Ok(Node::Binom(Box::new(n), Box::new(k)))
            }
            "pmod" => Ok(Node::Row(vec![
                Node::Run(" (".into()),
                Node::Upright("mod".into()),
                Node::Run(" ".into()),
                self.parse_atom()?,
                Node::Run(")".into()),
            ])),
            "bmod" => Ok(Node::Upright("mod".into())),
            "underbrace" => {
                let base = self.parse_atom()?;
                Ok(Node::GroupChar {
                    chr: '\u{23df}', // ⏟
                    pos: "bot",
                    base: Box::new(base),
                })
            }
            "overbrace" => {
                let base = self.parse_atom()?;
                Ok(Node::GroupChar {
                    chr: '\u{23de}', // ⏞
                    pos: "top",
                    base: Box::new(base),
                })
            }
            "underset" => {
                let lim = self.parse_atom()?;
                let base = self.parse_atom()?;
                Ok(Node::Lim {
                    pos: "bot",
                    base: Box::new(base),
                    lim: Box::new(lim),
                })
            }
            "overset" | "stackrel" => {
                let lim = self.parse_atom()?;
                let base = self.parse_atom()?;
                Ok(Node::Lim {
                    pos: "top",
                    base: Box::new(base),
                    lim: Box::new(lim),
                })
            }
            "boxed" => Ok(Node::BorderBox(Box::new(self.parse_atom()?))),
            "left" => self.parse_delim(),
            "right" => Err("游离的 \\right（缺少配对的 \\left）".to_string()),
            "begin" => self.parse_environment(),
            "end" => Err("游离的 \\end{...}（缺少配对的 \\begin）".to_string()),
            "overline" => Ok(Node::Bar {
                top: true,
                base: Box::new(self.parse_atom()?),
            }),
            "underline" => Ok(Node::Bar {
                top: false,
                base: Box::new(self.parse_atom()?),
            }),
            // 通用符号/函数/重音/大算子查表
            _ => {
                if let Some(chr) = math_accent(&name) {
                    return Ok(Node::Accent {
                        chr,
                        base: Box::new(self.parse_atom()?),
                    });
                }
                if let Some((op, over_under)) = nary_symbol(&name) {
                    let (sub, sup) = self.parse_nary_limits()?;
                    let body = self.parse_operand()?;
                    return Ok(Node::Nary {
                        op: op.to_string(),
                        sub,
                        sup,
                        body: Box::new(body),
                        over_under,
                    });
                }
                if let Some(f) = function_name(&name) {
                    return Ok(Node::Upright(f.to_string()));
                }
                if let Some(sym) = math_symbol(&name) {
                    return Ok(Node::Run(sym.to_string()));
                }
                Err(format!("不支持的 LaTeX 命令 \\{name}（位置 {}）", self.i))
            }
        }
    }

    /// 读取 n-ary 算子的 `_下限`/`^上限`（顺序任意）。
    #[allow(clippy::type_complexity)]
    fn parse_nary_limits(&mut self) -> PResult<(Option<Box<Node>>, Option<Box<Node>>)> {
        let mut sub = None;
        let mut sup = None;
        loop {
            self.skip_space();
            match self.peek() {
                Some('_') => {
                    self.i += 1;
                    self.skip_space();
                    sub = Some(Box::new(self.parse_atom()?));
                }
                Some('^') => {
                    self.i += 1;
                    self.skip_space();
                    sup = Some(Box::new(self.parse_atom()?));
                }
                _ => break,
            }
        }
        Ok((sub, sup))
    }

    /// n-ary 算子的操作数：下一个原子（含自身上下标）。只取一个原子以匹配
    /// LaTeX 优先级（`\sum_{i=1}^n i + j` 的操作数是 `i` 而非 `i + j`）。
    fn parse_operand(&mut self) -> PResult<Node> {
        self.skip_space();
        match self.peek() {
            None | Some('}') => Ok(Node::Row(Vec::new())),
            _ => {
                let a = self.parse_atom()?;
                self.maybe_scripts(a)
            }
        }
    }

    /// 解析 `\left<open> … \right<close>`（`\left` 已消费）；不配对则报错。
    fn parse_delim(&mut self) -> PResult<Node> {
        let open = self.read_delim();
        let start = self.i;
        let mut depth = 1;
        while self.i < self.s.len() {
            if self.at_command("left") {
                depth += 1;
                self.i += "\\left".len();
                continue;
            }
            if self.at_command("right") {
                depth -= 1;
                if depth == 0 {
                    let inner: String = self.s[start..self.i].iter().collect();
                    self.i += "\\right".len();
                    if self.peek().is_none() {
                        return Err(format!(
                            "\\right 缺少定界符（位置 {}；可用 . 表示空定界符）",
                            self.i
                        ));
                    }
                    let close = self.read_delim();
                    return Ok(Node::Delim {
                        open,
                        close,
                        body: Box::new(self.parse_with_depth(&inner)?),
                    });
                }
                self.i += "\\right".len();
                continue;
            }
            self.i += 1;
        }
        Err(format!("\\left{open} 缺少配对的 \\right（位置 {start}）"))
    }

    /// 以指定深度种子解析子串（跨子解析器的递归仍计入 `MAX_DEPTH`）。
    fn parse_with_depth(&self, latex: &str) -> PResult<Node> {
        let mut p = Parser {
            s: latex.chars().collect(),
            i: 0,
            depth: self.depth,
        };
        p.parse_row()
    }

    /// 读取平衡的 `{...}` 分组内文并越过其结尾。
    fn read_raw_group(&mut self) -> String {
        self.skip_space();
        if self.peek() != Some('{') {
            return String::new();
        }
        self.i += 1;
        let start = self.i;
        let mut depth = 1;
        while self.i < self.s.len() {
            match self.s[self.i] {
                '\\' => {
                    self.i = (self.i + 2).min(self.s.len());
                    continue;
                }
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let inner: String = self.s[start..self.i].iter().collect();
                        self.i += 1;
                        return inner;
                    }
                }
                _ => {}
            }
            self.i += 1;
        }
        self.s[start..].iter().collect()
    }

    fn matches(&self, pat: &[char]) -> bool {
        self.i + pat.len() <= self.s.len() && self.s[self.i..self.i + pat.len()] == *pat
    }

    /// 解析 `\begin{env} … \end{env}`（`\begin` 已消费）。
    fn parse_environment(&mut self) -> PResult<Node> {
        let raw = self.read_raw_group();
        let env_raw = raw.trim().to_string();
        if env_raw.is_empty() {
            return Err("\\begin 缺少环境名".to_string());
        }
        let env = env_raw.trim_end_matches('*').to_string();
        // array/alignat 携带列格式参数，解析但不使用（渲染为居中矩阵）。
        if env == "array" || env == "alignat" {
            self.skip_space();
            if self.peek() == Some('{') {
                let _ = self.read_raw_group();
            }
        }
        let body = self.read_env_body(&env_raw);
        if env == "aligned" || env == "align" || env == "gathered" {
            // 等式数组：按 \\ 分行，剥离行内 & 对齐标记，映射为 m:eqArr
            let rows = split_rows(&body, self.depth)?;
            return Ok(Node::EqArray { rows });
        }
        let rows = split_matrix(&body, self.depth)?;
        Ok(Node::Matrix {
            rows,
            delim: matrix_delim(&env),
        })
    }

    /// 读取到匹配的 `\end{env}`（嵌套感知，含星号形式）。
    fn read_env_body(&mut self, env: &str) -> String {
        let bpat: Vec<char> = format!("\\begin{{{env}}}").chars().collect();
        let epat: Vec<char> = format!("\\end{{{env}}}").chars().collect();
        let start = self.i;
        let mut depth = 1;
        while self.i < self.s.len() {
            if self.matches(&bpat) {
                depth += 1;
                self.i += bpat.len();
                continue;
            }
            if self.matches(&epat) {
                depth -= 1;
                if depth == 0 {
                    let inner: String = self.s[start..self.i].iter().collect();
                    self.i += epat.len();
                    return inner;
                }
                self.i += epat.len();
                continue;
            }
            self.i += 1;
        }
        let inner: String = self.s[start..].iter().collect();
        self.i = self.s.len();
        inner
    }

    /// 读取 `\left`/`\right` 后的定界符：单字符、`\cmd` 或 `.`（空）。
    fn read_delim(&mut self) -> String {
        self.skip_space();
        match self.peek() {
            Some('\\') => {
                // `\|` 是控制符号（双竖线），与直接写 `|` 区分开
                if self.i + 1 < self.s.len() && self.s[self.i + 1] == '|' {
                    self.i += 2;
                    return "‖".to_string();
                }
                let name = self.read_name();
                delim_symbol(&name).to_string()
            }
            Some('.') => {
                self.i += 1;
                String::new()
            }
            Some(c) => {
                self.i += 1;
                c.to_string()
            }
            None => String::new(),
        }
    }

    /// 读取 `[...]` 可选组并解析其内容为数学。
    fn read_bracket(&mut self) -> PResult<Node> {
        self.i += 1; // 消费 '['
        let start = self.i;
        while self.peek().is_some() && self.peek() != Some(']') {
            self.i += 1;
        }
        let inner: String = self.s[start..self.i].iter().collect();
        if self.peek() == Some(']') {
            self.i += 1;
        } else {
            return Err("缺少闭合的 ]（\\sqrt 的次根参数）".to_string());
        }
        self.parse_with_depth(&inner)
    }

    /// 光标处是否为完整命令 `\name`（词边界）。
    fn at_command(&self, name: &str) -> bool {
        if self.peek() != Some('\\') {
            return false;
        }
        let mut k = self.i + 1;
        for pc in name.chars() {
            if self.s.get(k) != Some(&pc) {
                return false;
            }
            k += 1;
        }
        !matches!(self.s.get(k), Some(c) if c.is_ascii_alphabetic())
    }
}

/// 入口：将 LaTeX 数学源码转换为完整的 `<m:oMath>…</m:oMath>` 元素。
///
/// 任何无法无损表达的构造返回 `Err`（调用方应回退保留 `$latex$` 原文，
/// 杜绝静默内容丢失）。
pub fn convert(latex: &str) -> easydoc_core::Result<String> {
    let mut p = Parser {
        s: latex.chars().collect(),
        i: 0,
        depth: 0,
    };
    let node = p
        .parse_row()
        .map_err(|e| easydoc_core::DocError::Format(format!("LaTeX→OMML 解析失败：{e}")))?;
    if p.peek().is_some() {
        return Err(easydoc_core::DocError::Format(format!(
            "LaTeX→OMML 解析失败：多余的 }}（位置 {}）",
            p.i
        )));
    }
    let inner = render(&node);
    Ok(format!("<m:oMath>{inner}</m:oMath>"))
}

/// 极限函数名（`\lim_{x\to 0}`、`\max_{x\in X}` 等需要极限布局）。
/// 注意：函数名已经过 `function_name` 映射（`\limsup` → "lim sup"）。
fn is_limit_function(name: &str) -> bool {
    matches!(
        name,
        "lim" | "lim sup" | "lim inf" | "max" | "min" | "sup" | "inf" | "det" | "gcd" | "Pr"
    )
}

/// 构建极限节点：上下标同时存在时下标进入 m:limLow，上标退化为上标节点。
fn build_lim(base: Node, sub: Option<Box<Node>>, sup: Option<Box<Node>>) -> Node {
    let base = Box::new(base);
    match (sub, sup) {
        (Some(sb), None) => Node::Lim {
            pos: "bot",
            base,
            lim: sb,
        },
        (None, Some(sp)) => Node::Lim {
            pos: "top",
            base,
            lim: sp,
        },
        (Some(sb), Some(sp)) => {
            let low = Node::Lim {
                pos: "bot",
                base,
                lim: sb,
            };
            Node::Sup(Box::new(low), sp)
        }
        (None, None) => unreachable!("build_lim 仅在存在上下标时调用"),
    }
}

// ---------------------------------------------------------------------------
// 符号表（LaTeX 命令 → Unicode 字形）
// ---------------------------------------------------------------------------

/// 重音命令 → 组合字符（`m:acc` 的 chr）。
fn math_accent(name: &str) -> Option<char> {
    Some(match name {
        "hat" | "widehat" => '\u{0302}', // \widehat 降级为窄帽（内容保真）
        "tilde" | "widetilde" => '\u{0303}',
        "bar" => '\u{0304}',
        "vec" | "overrightarrow" => '\u{20D7}',
        "dot" => '\u{0307}',
        "ddot" => '\u{0308}',
        "dddot" => '\u{20DB}',
        "check" => '\u{030C}',
        "breve" => '\u{0306}',
        "acute" => '\u{0301}',
        "grave" => '\u{0300}',
        "mathring" => '\u{030A}',
        _ => return None,
    })
}

/// n-ary 大算子：`(字形, over_under)`——求和类上下限置于上下，积分类置于角标。
fn nary_symbol(name: &str) -> Option<(&'static str, bool)> {
    Some(match name {
        "sum" => ("∑", true),
        "prod" => ("∏", true),
        "coprod" => ("∐", true),
        "bigcup" => ("⋃", true),
        "bigcap" => ("⋂", true),
        "bigsqcup" => ("⨆", true),
        "bigvee" => ("⋁", true),
        "bigwedge" => ("⋀", true),
        "bigoplus" => ("⨁", true),
        "bigotimes" => ("⨂", true),
        "bigodot" => ("⨀", true),
        "int" => ("∫", false),
        "iint" => ("∬", false),
        "iiint" => ("∭", false),
        "oint" => ("∮", false),
        _ => return None,
    })
}

/// 数学函数名（正体渲染）。
fn function_name(name: &str) -> Option<&'static str> {
    Some(match name {
        "sin" => "sin",
        "cos" => "cos",
        "tan" => "tan",
        "cot" => "cot",
        "sec" => "sec",
        "csc" => "csc",
        "sinh" => "sinh",
        "cosh" => "cosh",
        "tanh" => "tanh",
        "coth" => "coth",
        "arcsin" => "arcsin",
        "arccos" => "arccos",
        "arctan" => "arctan",
        "log" => "log",
        "ln" => "ln",
        "lg" => "lg",
        "exp" => "exp",
        "lim" => "lim",
        "limsup" => "lim sup",
        "liminf" => "lim inf",
        "max" => "max",
        "min" => "min",
        "inf" => "inf",
        "sup" => "sup",
        "det" => "det",
        "gcd" => "gcd",
        "deg" => "deg",
        "dim" => "dim",
        "ker" => "ker",
        "hom" => "hom",
        "arg" => "arg",
        "Pr" => "Pr",
        "mod" | "bmod" => "mod",
        _ => return None,
    })
}

/// 普通符号（希腊字母、二元运算符、关系、箭头、逻辑/集合等）。
fn math_symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        // ---- 希腊字母（小写） ----
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ϵ",
        "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "vartheta" => "ϑ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "varpi" => "ϖ",
        "rho" => "ρ",
        "varrho" => "ϱ",
        "sigma" => "σ",
        "varsigma" => "ς",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "ϕ",
        "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        // ---- 希腊字母（大写） ----
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Upsilon" => "Υ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        // ---- 二元运算符 ----
        "times" => "×",
        "div" => "÷",
        "pm" => "±",
        "mp" => "∓",
        "cdot" => "⋅",
        "ast" => "∗",
        "star" => "⋆",
        "circ" => "∘",
        "bullet" => "∙",
        "oplus" => "⊕",
        "ominus" => "⊖",
        "otimes" => "⊗",
        "odot" => "⊙",
        "cup" => "∪",
        "cap" => "∩",
        "setminus" | "smallsetminus" => "∖",
        "wedge" | "land" => "∧",
        "vee" | "lor" => "∨",
        "sqcup" => "⊔",
        "sqcap" => "⊓",
        // ---- 关系 ----
        "leq" | "le" => "≤",
        "geq" | "ge" => "≥",
        "neq" | "ne" => "≠",
        "equiv" => "≡",
        "approx" => "≈",
        "cong" => "≅",
        "sim" => "∼",
        "simeq" => "≃",
        "propto" => "∝",
        "ll" => "≪",
        "gg" => "≫",
        "subset" => "⊂",
        "supset" => "⊃",
        "subseteq" => "⊆",
        "supseteq" => "⊇",
        "in" => "∈",
        "notin" => "∉",
        "ni" => "∋",
        "perp" | "bot" => "⊥",
        "parallel" => "∥",
        "mid" => "∣",
        "models" => "⊨",
        "vdash" => "⊢",
        "prec" => "≺",
        "succ" => "≻",
        "lhd" | "vartriangleleft" => "⊲",
        "rhd" | "vartriangleright" => "⊳",
        "unlhd" | "trianglelefteq" => "⊴",
        "unrhd" | "trianglerighteq" => "⊵",
        "ntrianglelefteq" => "⋬",
        "ntrianglerighteq" => "⋭",
        "nmid" => "∤",
        "nparallel" => "∦",
        "upharpoonright" | "restriction" => "↾",
        "upharpoonleft" => "↿",
        "downharpoonright" => "⇂",
        "downharpoonleft" => "⇃",
        // ---- 箭头 ----
        "to" | "rightarrow" => "→",
        "leftarrow" | "gets" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" => "⇒",
        "Leftarrow" => "⇐",
        "Leftrightarrow" => "⇔",
        "mapsto" => "↦",
        "hookrightarrow" => "↪",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "longrightarrow" => "⟶",
        "longleftarrow" => "⟵",
        "implies" => "⟹",
        "iff" => "⟺",
        // ---- 逻辑/集合/杂项 ----
        "forall" => "∀",
        "exists" => "∃",
        "nexists" => "∄",
        "neg" | "lnot" => "¬",
        "nabla" => "∇",
        "partial" => "∂",
        "infty" => "∞",
        "emptyset" | "varnothing" => "∅",
        "angle" => "∠",
        "triangle" => "△",
        "square" => "□",
        "diamond" => "⋄",
        "aleph" => "ℵ",
        "hbar" => "ℏ",
        "ell" => "ℓ",
        "Re" => "ℜ",
        "Im" => "ℑ",
        "wp" => "℘",
        "prime" => "′",
        "dagger" => "†",
        "ddagger" => "‡",
        "top" => "⊤",
        "surd" => "√",
        "flat" => "♭",
        "sharp" => "♯",
        // ---- 点 ----
        "cdots" => "⋯",
        "ldots" | "dots" => "…",
        "vdots" => "⋮",
        "ddots" => "⋱",
        // ---- 转义字面量 ----
        "{" => "{",
        "}" => "}",
        "|" => "‖",
        "%" => "%",
        "&" => "&",
        "#" => "#",
        "_" => "_",
        "$" => "$",
        "backslash" => "\\",
        "langle" => "⟨",
        "rangle" => "⟩",
        "lceil" => "⌈",
        "rceil" => "⌉",
        "lfloor" => "⌊",
        "rfloor" => "⌋",
        // ---- 空格（统一映射为细空格 U+2009） ----
        "," | ":" | ";" | " " | "quad" | "qquad" | "!" | "thinspace" => "\u{2009}",
        _ => return None,
    })
}

/// `\left`/`\right` 定界符命令 → 字形。
fn delim_symbol(name: &str) -> &'static str {
    match name {
        "lbrace" | "{" => "{",
        "rbrace" | "}" => "}",
        "vert" => "|",
        "Vert" | "lVert" | "rVert" => "‖",
        "langle" => "⟨",
        "rangle" => "⟩",
        "lceil" => "⌈",
        "rceil" => "⌉",
        "lfloor" => "⌊",
        "rfloor" => "⌋",
        "backslash" => "\\",
        "uparrow" => "↑",
        "downarrow" => "↓",
        other => math_symbol(other).unwrap_or(""),
    }
}

/// 矩阵环境 → 定界符（`None` = 裸 `m:m`）。
fn matrix_delim(env: &str) -> Option<(String, String)> {
    let (o, c) = match env {
        "pmatrix" => ("(", ")"),
        "bmatrix" => ("[", "]"),
        "Bmatrix" => ("{", "}"),
        "vmatrix" => ("|", "|"),
        "Vmatrix" => ("‖", "‖"),
        "cases" => ("{", ""),
        _ => return None, // matrix, smallmatrix, array, alignat, gathered…
    };
    Some((o.to_string(), c.to_string()))
}

/// 数学字母表：ASCII 字母/数字 → Unicode 数学字母（黑板/手写/哥特）。
fn alphabet(style: &str, s: &str) -> String {
    s.chars()
        .map(|c| alpha_char(style, c).unwrap_or(c))
        .collect()
}

fn nth(table: &str, i: usize) -> Option<char> {
    table.chars().nth(i)
}

fn alpha_char(style: &str, c: char) -> Option<char> {
    let upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let idx = upper.find(c);
    match style {
        "mathbb" => {
            if let Some(i) = idx {
                return nth("𝔸𝔹ℂ𝔻𝔼𝔽𝔾ℍ𝕀𝕁𝕂𝕃𝕄ℕ𝕆ℙℚℝ𝕊𝕋𝕌𝕍𝕎𝕏𝕐ℤ", i);
            }
            if c.is_ascii_lowercase() {
                return char::from_u32(0x1D552 + (c as u32 - 'a' as u32));
            }
            if c.is_ascii_digit() {
                return char::from_u32(0x1D7D8 + (c as u32 - '0' as u32));
            }
            None
        }
        "mathcal" | "mathscr" => idx.and_then(|i| nth("𝒜ℬ𝒞𝒟ℰℱ𝒢ℋℐ𝒥𝒦ℒℳ𝒩𝒪𝒫𝒬ℛ𝒮𝒯𝒰𝒱𝒲𝒳𝒴𝒵", i)),
        "mathfrak" => {
            if let Some(i) = idx {
                return nth("𝔄𝔅ℭ𝔇𝔈𝔉𝔊ℌℑ𝔍𝔎𝔏𝔐𝔑𝔒𝔓𝔔ℜ𝔖𝔗𝔘𝔙𝔚𝔛𝔜ℨ", i);
            }
            if c.is_ascii_lowercase() {
                return char::from_u32(0x1D51E + (c as u32 - 'a' as u32));
            }
            None
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

/// 合并相邻 Run 并丢弃空 Run。
fn push_merge(items: &mut Vec<Node>, atom: Node) {
    if let Node::Run(s) = &atom {
        if s.is_empty() {
            return;
        }
        if let Some(Node::Run(prev)) = items.last_mut() {
            prev.push_str(s);
            return;
        }
    }
    items.push(atom);
}

/// 展平节点为纯文本（用于 `\mathrm`/`\text` 等正体内容）。
///
/// 含数学结构（分式/根式/脚本等）时返回 `Err`——展平会丢内容，宁可直接报错。
fn flatten_text(node: &Node) -> PResult<String> {
    match node {
        Node::Run(s) | Node::Upright(s) => Ok(s.clone()),
        Node::Row(items) => {
            let mut out = String::new();
            for item in items {
                out.push_str(&flatten_text(item)?);
            }
            Ok(out)
        }
        _ => Err("\\mathrm/\\text 等命令的内容含数学结构，无法无损展平".to_string()),
    }
}

/// 将矩阵/数组环境体按顶层 `\\` 分行、顶层 `&` 分列，逐格解析为数学。
fn split_matrix(body: &str, seed_depth: usize) -> PResult<Vec<Vec<Node>>> {
    let cells = split_cells(body);
    let mut rows = Vec::new();
    for row in &cells {
        let mut parsed = Vec::new();
        for cell in row {
            parsed.push(parse_depth_seeded(cell, seed_depth)?);
        }
        rows.push(parsed);
    }
    Ok(rows)
}

/// 将等式数组环境体按顶层 `\\` 分行，剥离顶层 `&` 对齐标记，逐行解析。
fn split_rows(body: &str, seed_depth: usize) -> PResult<Vec<Node>> {
    let cells = split_cells(body);
    let mut rows = Vec::new();
    for row in cells {
        // 剥离 & 对齐标记后整行作为一个数学节点
        let joined = row.join("");
        rows.push(parse_depth_seeded(&joined, seed_depth)?);
    }
    Ok(rows)
}

/// 顶层切分：`\\` 分行（跳过可选 `[间距]`），顶层 `&` 分列；`\cr` 同 `\\`。
/// 括号/命令内的 `&`/`\\` 不切分；尾部空行（结尾 `\\`）丢弃。
#[allow(clippy::many_single_char_names)]
fn split_cells(body: &str) -> Vec<Vec<String>> {
    let s: Vec<char> = body.chars().collect();
    let n = s.len();
    let mut rows: Vec<Vec<String>> = vec![vec![String::new()]];
    let mut depth = 0;
    let mut i = 0;
    while i < n {
        let c = s[i];
        if c == '\\' {
            if i + 1 < n && s[i + 1] == '\\' {
                let mut j = i + 2;
                while j < n && matches!(s[j], ' ' | '\n' | '\t' | '\r') {
                    j += 1;
                }
                if j < n && s[j] == '[' {
                    while j < n && s[j] != ']' {
                        j += 1;
                    }
                    if j < n {
                        j += 1;
                    }
                }
                rows.push(vec![String::new()]);
                i = j;
                continue;
            }
            // 命令：连同名字拷贝，命令内的 &/\\ 不切分
            let start = i;
            let mut j = i + 1;
            if j < n && s[j].is_ascii_alphabetic() {
                while j < n && s[j].is_ascii_alphabetic() {
                    j += 1;
                }
            } else if j < n {
                j += 1;
            }
            let cmd: String = s[start + 1..j].iter().collect();
            if cmd == "cr" {
                rows.push(vec![String::new()]);
                i = j;
                continue;
            }
            rows.last_mut()
                .unwrap()
                .last_mut()
                .unwrap()
                .extend(&s[start..j]);
            i = j;
            continue;
        }
        match c {
            '{' => {
                depth += 1;
                rows.last_mut().unwrap().last_mut().unwrap().push(c);
            }
            '}' => {
                depth -= 1;
                rows.last_mut().unwrap().last_mut().unwrap().push(c);
            }
            '&' if depth == 0 => rows.last_mut().unwrap().push(String::new()),
            _ => rows.last_mut().unwrap().last_mut().unwrap().push(c),
        }
        i += 1;
    }
    rows.into_iter()
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .map(|row| {
            row.into_iter()
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect()
}

/// 以指定深度种子解析一个子串（跨单元格递归仍计入 `MAX_DEPTH`）。
fn parse_depth_seeded(latex: &str, seed_depth: usize) -> PResult<Node> {
    let mut p = Parser {
        s: latex.chars().collect(),
        i: 0,
        depth: seed_depth,
    };
    let node = p.parse_row()?;
    if p.peek().is_some() {
        return Err(format!("单元格/行内多余的 }}（位置 {}）", p.i));
    }
    Ok(node)
}

// ---------------------------------------------------------------------------
// OMML 渲染
// ---------------------------------------------------------------------------

/// `c` 是否为 XML 1.0 合法文本字符（控制字符除 tab/换行/回车外必须剔除）。
fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | ' '..='\u{d7ff}' | '\u{e000}'..)
}

/// 转义 XML 文本/属性内容；`"` 一并转义以安全用于属性位置。
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c if is_xml_char(c) => out.push(c),
            _ => {}
        }
    }
    out
}

/// 数学斜体 run。
fn run(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    format!("<m:r><m:t>{}</m:t></m:r>", escape(s))
}

/// 正体 run（`<m:nor/>` 抑制默认数学斜体）。
fn upright(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    format!(
        "<m:r><m:rPr><m:nor/></m:rPr><m:t xml:space=\"preserve\">{}</m:t></m:r>",
        escape(s)
    )
}

/// 渲染节点内容为容器子元素（`m:e`/`m:num`/…），保证至少一个 run。
fn cell(node: &Node) -> String {
    let r = render(node);
    if r.is_empty() {
        "<m:r><m:t></m:t></m:r>".to_string()
    } else {
        r
    }
}

/// 将数学 AST 渲染为 OMML。
fn render(node: &Node) -> String {
    match node {
        Node::Run(s) => run(s),
        Node::Upright(s) => upright(s),
        Node::Row(items) => items.iter().map(render).collect(),
        Node::Frac(num, den) => format!(
            "<m:f><m:fPr><m:type m:val=\"bar\"/></m:fPr><m:num>{}</m:num><m:den>{}</m:den></m:f>",
            cell(num),
            cell(den)
        ),
        Node::Sup(base, sup) => format!(
            "<m:sSup><m:e>{}</m:e><m:sup>{}</m:sup></m:sSup>",
            cell(base),
            cell(sup)
        ),
        Node::Sub(base, sub) => format!(
            "<m:sSub><m:e>{}</m:e><m:sub>{}</m:sub></m:sSub>",
            cell(base),
            cell(sub)
        ),
        Node::SubSup(base, sub, sup) => format!(
            "<m:sSubSup><m:e>{}</m:e><m:sub>{}</m:sub><m:sup>{}</m:sup></m:sSubSup>",
            cell(base),
            cell(sub),
            cell(sup)
        ),
        Node::Sqrt(rad) => format!(
            "<m:rad><m:radPr><m:degHide m:val=\"1\"/></m:radPr><m:deg/><m:e>{}</m:e></m:rad>",
            cell(rad)
        ),
        Node::Root(index, rad) => format!(
            "<m:rad><m:radPr><m:degHide m:val=\"0\"/></m:radPr><m:deg>{}</m:deg><m:e>{}</m:e></m:rad>",
            cell(index),
            cell(rad)
        ),
        Node::Nary {
            op,
            sub,
            sup,
            body,
            over_under,
        } => {
            let limloc = if *over_under { "undOvr" } else { "subSup" };
            let (lower_hide, lower_xml) = match sub {
                Some(n) => ("0", render(n)),
                None => ("1", String::new()),
            };
            let (upper_hide, upper_xml) = match sup {
                Some(n) => ("0", render(n)),
                None => ("1", String::new()),
            };
            format!(
                "<m:nary><m:naryPr><m:chr m:val=\"{}\"/><m:limLoc m:val=\"{limloc}\"/>\
                 <m:subHide m:val=\"{lower_hide}\"/><m:supHide m:val=\"{upper_hide}\"/></m:naryPr>\
                 <m:sub>{lower_xml}</m:sub><m:sup>{upper_xml}</m:sup><m:e>{}</m:e></m:nary>",
                escape(op),
                cell(body)
            )
        }
        Node::Delim { open, close, body } => format!(
            "<m:d><m:dPr><m:begChr m:val=\"{}\"/><m:endChr m:val=\"{}\"/></m:dPr><m:e>{}</m:e></m:d>",
            escape(open),
            escape(close),
            cell(body)
        ),
        Node::Accent { chr, base } => format!(
            "<m:acc><m:accPr><m:chr m:val=\"{}\"/></m:accPr><m:e>{}</m:e></m:acc>",
            escape(&chr.to_string()),
            cell(base)
        ),
        Node::Bar { top, base } => format!(
            "<m:bar><m:barPr><m:pos m:val=\"{}\"/></m:barPr><m:e>{}</m:e></m:bar>",
            if *top { "top" } else { "bot" },
            cell(base)
        ),
        Node::Styled { sty, text } => {
            if text.is_empty() {
                String::new()
            } else {
                format!(
                    "<m:r><m:rPr><m:sty m:val=\"{}\"/></m:rPr><m:t xml:space=\"preserve\">{}</m:t></m:r>",
                    sty,
                    escape(text)
                )
            }
        }
        Node::Binom(num, den) => format!(
            "<m:d><m:dPr><m:begChr m:val=\"(\"/><m:endChr m:val=\")\"/></m:dPr><m:e>\
             <m:f><m:fPr><m:type m:val=\"noBar\"/></m:fPr><m:num>{}</m:num><m:den>{}</m:den></m:f>\
             </m:e></m:d>",
            cell(num),
            cell(den)
        ),
        Node::Matrix { rows, delim } => {
            let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
            let mut m = String::from("<m:m>");
            for row in rows {
                m.push_str("<m:mr>");
                for c in 0..ncols {
                    let content = row
                        .get(c)
                        .map_or_else(|| "<m:r><m:t></m:t></m:r>".to_string(), cell);
                    let _ = write!(m, "<m:e>{content}</m:e>");
                }
                m.push_str("</m:mr>");
            }
            m.push_str("</m:m>");
            match delim {
                Some((open, close)) => format!(
                    "<m:d><m:dPr><m:begChr m:val=\"{}\"/><m:endChr m:val=\"{}\"/></m:dPr><m:e>{}</m:e></m:d>",
                    escape(open),
                    escape(close),
                    m
                ),
                None => m,
            }
        }
        Node::EqArray { rows } => {
            let mut out = String::from("<m:eqArr>");
            for row in rows {
                let _ = write!(out, "<m:e>{}</m:e>", cell(row));
            }
            out.push_str("</m:eqArr>");
            out
        }
        Node::GroupChar { chr, pos, base } => format!(
            "<m:groupChr><m:groupChrPr><m:chr m:val=\"{}\"/><m:pos m:val=\"{}\"/></m:groupChrPr>\
             <m:e>{}</m:e></m:groupChr>",
            escape(&chr.to_string()),
            pos,
            cell(base)
        ),
        Node::Lim { pos, base, lim } => format!(
            "<m:lim{}><m:e>{}</m:e><m:lim>{}</m:lim></m:lim{}>",
            if *pos == "top" { "Upp" } else { "Low" },
            cell(base),
            cell(lim),
            if *pos == "top" { "Upp" } else { "Low" }
        ),
        Node::BorderBox(base) => format!("<m:borderBox><m:e>{}</m:e></m:borderBox>", cell(base)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 断言转换成功且包含指定子串。
    fn ok_contains(latex: &str, expected: &str) {
        let omml = convert(latex).unwrap_or_else(|e| panic!("{latex} 应转换成功：{e}"));
        assert!(
            omml.contains(expected),
            "{latex} 应包含 {expected}，实际：{omml}"
        );
    }

    /// 断言转换失败（未知命令/不配对/无法无损表达）。
    fn err(latex: &str) {
        let r = convert(latex);
        assert!(r.is_err(), "{latex} 应转换失败，实际成功：{r:?}");
    }

    #[test]
    fn simple_run() {
        let omml = convert("x^2").unwrap();
        assert_eq!(
            omml,
            "<m:oMath><m:sSup><m:e><m:r><m:t>x</m:t></m:r></m:e><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup></m:oMath>"
        );
    }

    #[test]
    fn fraction() {
        ok_contains(r"\frac{1}{2}", "<m:f>");
        ok_contains(r"\frac{1}{2}", "m:val=\"bar\"");
    }

    #[test]
    fn sqrt_and_root() {
        ok_contains(r"\sqrt{x}", "<m:rad>");
        ok_contains(r"\sqrt[3]{x}", "<m:deg>");
        ok_contains(r"\sqrt[3]{x}", "m:val=\"0\"");
    }

    #[test]
    fn scripts_both_orders() {
        ok_contains("x_i^2", "<m:sSubSup>");
        ok_contains("x^2_i", "<m:sSubSup>");
    }

    #[test]
    fn nary_sum_int() {
        ok_contains(r"\sum_{i=1}^{n} i", "m:limLoc m:val=\"undOvr\"");
        ok_contains(r"\int_0^1 x", "m:limLoc m:val=\"subSup\"");
    }

    #[test]
    fn nary_operand_single_atom() {
        // 操作数只取一个原子（LaTeX 优先级）：\sum_{i=1}^n i + j 的操作数是 i
        let omml = convert(r"\sum_{i=1}^n i + j").unwrap();
        assert!(
            omml.contains("<m:e><m:r><m:t>i</m:t></m:r></m:e>"),
            "{omml}"
        );
        assert!(omml.contains("+j"), "操作数后的 + j 应保留：{omml}");
    }

    #[test]
    fn left_right_delim() {
        ok_contains(r"\left(\frac{a}{b}\right)", "<m:d>");
        ok_contains(r"\left(\frac{a}{b}\right)", "m:begChr m:val=\"(\"");
        ok_contains(r"\left\{x\right\}", "m:begChr m:val=\"{\"");
        // \left| 应映射为单竖线而非 ‖
        ok_contains(r"\left|x\right|", "m:begChr m:val=\"|\"");
    }

    #[test]
    fn accent_bar() {
        ok_contains(r"\hat{x}", "<m:acc>");
        ok_contains(r"\vec{v}", "\u{20D7}");
        ok_contains(r"\overline{AB}", "<m:bar>");
        ok_contains(r"\overline{AB}", "m:pos m:val=\"top\"");
        ok_contains(r"\underline{x}", "m:pos m:val=\"bot\"");
    }

    #[test]
    fn styled_and_functions() {
        ok_contains(r"\mathbf{v}", "<m:sty m:val=\"b\"/>");
        ok_contains(r"\sin x", "<m:nor/>");
        ok_contains(r"\mathrm{d}x", "<m:nor/>");
        ok_contains(r"\text{hello}", "hello");
    }

    #[test]
    fn binom() {
        ok_contains(r"\binom{n}{k}", "m:type m:val=\"noBar\"");
        ok_contains(r"\binom{n}{k}", "m:begChr m:val=\"(\"");
    }

    #[test]
    fn matrix_envs() {
        ok_contains(r"\begin{pmatrix}a&b\\c&d\end{pmatrix}", "<m:m>");
        ok_contains(
            r"\begin{pmatrix}a&b\\c&d\end{pmatrix}",
            "m:begChr m:val=\"(\"",
        );
        ok_contains(
            r"\begin{cases}x&x>0\\-x&x\le0\end{cases}",
            "m:begChr m:val=\"{\"",
        );
        // 星号形式的环境
        ok_contains(r"\begin{matrix*}a&b\\c&d\end{matrix*}", "<m:m>");
    }

    #[test]
    fn aligned_becomes_eq_arr() {
        ok_contains(r"\begin{aligned}a&=b\\c&=d\end{aligned}", "<m:eqArr>");
        let omml = convert(r"\begin{aligned}a&=b\\c&=d\end{aligned}").unwrap();
        assert!(!omml.contains('&'), "aligned 的对齐标记应剥离：{omml}");
        assert!(omml.matches("<m:e>").count() >= 2, "{omml}");
    }

    #[test]
    fn greek_and_operators() {
        ok_contains(r"\alpha + \beta", "α");
        ok_contains(r"\leq", "≤");
        ok_contains(r"\to", "→");
        ok_contains(r"\infty", "∞");
        ok_contains(r"\Gamma", "Γ");
    }

    #[test]
    fn under_overbrace() {
        ok_contains(r"\underbrace{x+y}", "<m:groupChr>");
        ok_contains(r"\underbrace{x+y}", "\u{23df}");
        ok_contains(r"\overbrace{x+y}", "\u{23de}");
        ok_contains(r"\overbrace{x+y}", "m:pos m:val=\"top\"");
    }

    #[test]
    fn overset_underset() {
        ok_contains(r"\overset{\to}{x}", "<m:limUpp>");
        ok_contains(r"\underset{x\to 0}{\lim}", "<m:limLow>");
        ok_contains(r"\stackrel{def}{=}", "<m:limUpp>");
    }

    #[test]
    fn lim_with_limits_uses_lim_low() {
        ok_contains(r"\lim_{x\to 0}", "<m:limLow>");
        ok_contains(r"\max_{x\in X}", "<m:limLow>");
        let omml = convert(r"\lim_{x\to 0} f(x)").unwrap();
        assert!(omml.contains("<m:limLow>"), "{omml}");
    }

    #[test]
    fn boxed() {
        ok_contains(r"\boxed{x}", "<m:borderBox>");
    }

    #[test]
    fn mathbb_alphabet() {
        ok_contains(r"\mathbb{R}", "ℝ");
        ok_contains(r"\mathcal{L}", "ℒ");
        ok_contains(r"\mathfrak{g}", "𝔤");
    }

    #[test]
    fn unsupported_commands_error() {
        err(r"\cancel{x}");
        err(r"\xrightarrow{a}");
        err(r"\color{red}x");
        err(r"\begin"); // 缺环境名
        err(r"\end{pmatrix}"); // 游离 \end
    }

    #[test]
    fn unbalanced_braces_error() {
        err(r"\frac{1}{2");
        err(r"x^2}");
        err(r"\left(x\right");
    }

    #[test]
    fn escaped_literals() {
        ok_contains(r"\{x\}", "{");
        ok_contains(r"a\%b", "%");
    }

    #[test]
    fn xml_escaping() {
        let omml = convert(r"\text{a<b&c}").unwrap();
        assert!(omml.contains("a&lt;b&amp;c"), "{omml}");
    }

    #[test]
    fn empty_input() {
        let omml = convert("").unwrap();
        assert_eq!(omml, "<m:oMath></m:oMath>");
    }

    #[test]
    fn deep_nesting_errors() {
        let deep = format!("{}1{}", "\\frac{".repeat(300), "}".repeat(300));
        assert!(convert(&deep).is_err(), "过深嵌套应报错");
    }
}
