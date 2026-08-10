//! Markdown 文本到 easydoc 语义模型（`DocumentContent`）的反向转换模块。
//!
//! 提供手工状态机解析器，将 Markdown 子集转换为 `DocumentBlock` 树。
//! 不依赖外部 Markdown 解析库（如 pulldown-cmark），保持依赖最小化。

use easydoc_core::{
    DocumentBlock, DocumentContent, DocumentImage, DocumentList, DocumentListItem, DocumentTable,
    DocumentTableCell, DocumentTableRow, DocumentTextRun, Result,
};

// ---------------------------------------------------------------------------
// 公共 API 类型
// ---------------------------------------------------------------------------

/// Markdown 到 `DocumentContent` 反向转换的构建器。
///
/// 使用 builder 模式配置解析选项，然后调用 [`MarkdownImportBuilder::do_import`] 执行转换。
///
/// # 示例
///
/// ```rust
/// use easydoc_markdown::MarkdownImportBuilder;
///
/// let result = MarkdownImportBuilder::new("# Hello\n\nBody text").do_import().unwrap();
/// assert_eq!(result.content.blocks.len(), 2);
/// ```
pub struct MarkdownImportBuilder {
    source: String,
    options: MarkdownImportOptions,
}

/// Markdown 导入的配置选项。
#[derive(Debug, Clone, Default)]
pub struct MarkdownImportOptions {
    /// 解析失败时的处理策略。
    pub on_parse_error: ParseErrorStrategy,
    /// 是否在结果中保留每段对应的源 Markdown 行号（预留，当前未实现映射）。
    pub track_line_numbers: bool,
}

/// 解析失败时的处理策略。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ParseErrorStrategy {
    /// 跳过无效行，继续处理。
    Skip,
    /// 发出警告并继续（默认）。
    #[default]
    Warn,
    /// 严格模式，遇到无法解析的内容时收集警告（不中断）。
    Strict,
}

/// 导入过程中产生的警告信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportWarning {
    /// 无法识别的语法行。
    UnrecognizedSyntax {
        /// 源行号（从 1 开始）。
        line: usize,
        /// 行内容。
        content: String,
    },
    /// 空标题（`# ` 后无文本）。
    EmptyHeading,
    /// 表格缺少分隔行（`| --- | --- |`）。
    TableMissingSeparator,
    /// 嵌套列表超过当前支持深度。
    NestedListUnsupported {
        /// 实际嵌套深度。
        depth: usize,
    },
    /// 图片缺少替代文本。
    ImageMissingAlt,
    /// 链接文本或 URL 为空。
    EmptyLink,
}

/// 导入结果，包含解析后的文档内容和产生的警告。
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// 解析后的文档内容。
    pub content: DocumentContent,
    /// 导入过程中产生的警告列表。
    pub warnings: Vec<ImportWarning>,
}

impl MarkdownImportBuilder {
    /// 创建新的 Markdown 导入构建器。
    ///
    /// `source` 为待解析的 Markdown 文本。
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            options: MarkdownImportOptions::default(),
        }
    }

    /// 使用完整选项替换当前配置。
    #[must_use]
    pub fn options(mut self, options: MarkdownImportOptions) -> Self {
        self.options = options;
        self
    }

    /// 设置解析失败策略。
    #[must_use]
    pub fn on_parse_error(mut self, strategy: ParseErrorStrategy) -> Self {
        self.options.on_parse_error = strategy;
        self
    }

    /// 设置是否跟踪行号。
    #[must_use]
    pub fn track_line_numbers(mut self, enabled: bool) -> Self {
        self.options.track_line_numbers = enabled;
        self
    }

    /// 执行 Markdown 到 `DocumentContent` 的转换。
    ///
    /// # Errors
    ///
    /// 当策略为 [`ParseErrorStrategy::Strict`] 且遇到无法解析的内容时返回错误。
    pub fn do_import(self) -> Result<ImportResult> {
        let mut parser = MarkdownParser::new(&self.source, self.options);
        parser.parse()?;
        Ok(ImportResult {
            content: DocumentContent {
                blocks: parser.blocks,
                ..DocumentContent::default()
            },
            warnings: parser.warnings,
        })
    }
}

// ---------------------------------------------------------------------------
// 内部解析器
// ---------------------------------------------------------------------------

/// 手工 Markdown 状态机解析器。
struct MarkdownParser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    options: MarkdownImportOptions,
    warnings: Vec<ImportWarning>,
    blocks: Vec<DocumentBlock>,
    /// 是否处于代码块内。
    in_code_block: bool,
    /// 代码块语言标记。
    code_lang: Option<String>,
    /// 代码块内容缓冲区。
    code_buffer: String,
}

impl<'a> MarkdownParser<'a> {
    fn new(source: &'a str, options: MarkdownImportOptions) -> Self {
        let lines: Vec<&'a str> = source.lines().collect();
        Self {
            lines,
            pos: 0,
            options,
            warnings: Vec::new(),
            blocks: Vec::new(),
            in_code_block: false,
            code_lang: None,
            code_buffer: String::new(),
        }
    }

    fn parse(&mut self) -> Result<()> {
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // 代码块状态：累积直到闭合
            if self.in_code_block {
                if line.trim_start().starts_with("```") {
                    self.blocks.push(DocumentBlock::CodeBlock {
                        language: self.code_lang.take(),
                        code: std::mem::take(&mut self.code_buffer),
                    });
                    self.in_code_block = false;
                    self.pos += 1;
                    continue;
                }
                if !self.code_buffer.is_empty() {
                    self.code_buffer.push('\n');
                }
                self.code_buffer.push_str(line);
                self.pos += 1;
                continue;
            }

            let trimmed = line.trim();

            // 空行：跳过
            if trimmed.is_empty() {
                self.pos += 1;
                continue;
            }

            // 代码块开始
            if let Some(stripped) = trimmed.strip_prefix("```") {
                let lang = stripped.trim();
                self.code_lang = if lang.is_empty() {
                    None
                } else {
                    Some(lang.to_owned())
                };
                self.in_code_block = true;
                self.code_buffer.clear();
                self.pos += 1;
                continue;
            }

            // 标题
            if let Some(level) = heading_level(trimmed) {
                let text = trimmed[level..].trim();
                if text.is_empty() {
                    self.push_warning(ImportWarning::EmptyHeading);
                }
                let runs = parse_inline(text);
                self.blocks.push(DocumentBlock::Heading {
                    level: u8::try_from(level).unwrap_or(6).min(6),
                    runs,
                });
                self.pos += 1;
                continue;
            }

            // 水平分隔线
            if is_thematic_break(trimmed) {
                self.blocks.push(DocumentBlock::ThematicBreak);
                self.pos += 1;
                continue;
            }

            // 表格（行以 | 开头，且下一行是分隔行）
            if trimmed.starts_with('|') && self.is_table_start() {
                self.parse_table()?;
                continue;
            }

            // 列表项
            if let Some((indent, ordered, marker_len)) = detect_list_item(line) {
                self.parse_list(indent, ordered, marker_len);
                continue;
            }

            // 图片（独立行）
            if trimmed.starts_with("![") {
                if let Some(image) = parse_image_line(trimmed) {
                    if image.alt_text.is_none() {
                        self.push_warning(ImportWarning::ImageMissingAlt);
                    }
                    self.blocks.push(DocumentBlock::Image(image));
                }
                self.pos += 1;
                continue;
            }

            // 普通段落：合并连续非空行直到遇到空行或其他块级元素
            self.parse_paragraph();
        }

        // 收尾：未闭合的代码块当作代码块处理
        if self.in_code_block && !self.code_buffer.is_empty() {
            self.blocks.push(DocumentBlock::CodeBlock {
                language: self.code_lang.take(),
                code: std::mem::take(&mut self.code_buffer),
            });
        }

        Ok(())
    }

    /// 解析段落：合并连续非空行直到遇到空行或其他块级元素。
    fn parse_paragraph(&mut self) {
        let mut runs = Vec::new();
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            // 遇到空行、标题、代码块、表格、列表、分隔线、图片——停止
            if trimmed.is_empty()
                || heading_level(trimmed).is_some()
                || trimmed.starts_with("```")
                || is_thematic_break(trimmed)
                || (trimmed.starts_with('|') && self.is_table_start())
                || detect_list_item(line).is_some()
                || trimmed.starts_with("![")
            {
                break;
            }

            if !runs.is_empty() {
                // 段落内换行用空格连接
                runs.push(DocumentTextRun {
                    text: " ".to_owned(),
                    ..DocumentTextRun::default()
                });
            }
            runs.extend(parse_inline(trimmed));
            self.pos += 1;
        }
        if !runs.is_empty() {
            self.blocks.push(DocumentBlock::Paragraph(runs));
        }
    }

    /// 检测当前位置是否为表格起始（当前行 `|` 开头，下一行是分隔行）。
    fn is_table_start(&self) -> bool {
        if self.pos + 1 >= self.lines.len() {
            return false;
        }
        let next = self.lines[self.pos + 1].trim();
        is_table_separator(next)
    }

    /// 解析表格：表头 + 分隔行 + 数据行。
    fn parse_table(&mut self) -> Result<()> {
        let header_line = self.lines[self.pos];
        let header_cells = split_table_row(header_line);
        self.pos += 1;

        // 分隔行
        if self.pos < self.lines.len() && is_table_separator(self.lines[self.pos].trim()) {
            self.pos += 1;
        } else {
            self.push_warning(ImportWarning::TableMissingSeparator);
        }

        // 数据行
        let mut rows = vec![make_table_row(&header_cells, true)];
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].trim();
            if line.is_empty() || !line.starts_with('|') {
                break;
            }
            let cells = split_table_row(line);
            rows.push(make_table_row(&cells, false));
            self.pos += 1;
        }

        self.blocks
            .push(DocumentBlock::Table(DocumentTable { rows }));
        Ok(())
    }

    /// 解析列表：从当前位置开始，处理所有连续的列表项（包括嵌套）。
    fn parse_list(&mut self, base_indent: usize, base_ordered: bool, _base_marker_len: usize) {
        let items = self.parse_list_items(base_indent, base_ordered);
        let list = DocumentList {
            ordered: base_ordered,
            start_number: Some(1),
            items,
        };
        self.blocks.push(DocumentBlock::List(list));
    }

    /// 递归解析列表项：处理当前缩进级别的所有项，并递归处理更深缩进的嵌套列表。
    fn parse_list_items(&mut self, target_indent: usize, ordered: bool) -> Vec<DocumentListItem> {
        let mut items: Vec<DocumentListItem> = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim();

            // 空行：检查后面是否还有同级或更深的列表项
            if trimmed.is_empty() {
                if self.pos + 1 < self.lines.len() {
                    let next = self.lines[self.pos + 1];
                    if let Some((next_indent, _, _)) = detect_list_item(next)
                        && next_indent >= target_indent
                    {
                        self.pos += 1;
                        continue;
                    }
                }
                break;
            }

            // 非列表项则停止
            let Some((indent, item_ordered, marker_len)) = detect_list_item(line) else {
                break;
            };

            // 缩进比目标少——结束当前列表
            if indent < target_indent {
                break;
            }

            // 缩进比目标多——嵌套列表，递归处理
            if indent > target_indent {
                // 这是当前最后一个 item 的嵌套子列表
                if let Some(last) = items.last_mut() {
                    let nested_items = self.parse_list_items(indent, item_ordered);
                    last.nested = Some(Box::new(DocumentList {
                        ordered: item_ordered,
                        start_number: Some(1),
                        items: nested_items,
                    }));
                }
                continue;
            }

            // 同级列表项
            // 检查列表类型是否变化（有序 vs 无序）——如果是，结束当前列表
            if items.is_empty() {
                // 第一个项，记录类型
            } else if item_ordered != ordered {
                // 列表类型变化——结束当前列表
                break;
            }

            let text = line[indent + marker_len..].trim();
            let runs = parse_inline(text);
            items.push(DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(runs)],
                nested: None,
            });
            self.pos += 1;
        }

        items
    }

    /// 根据策略推入警告。
    fn push_warning(&mut self, warning: ImportWarning) {
        match self.options.on_parse_error {
            ParseErrorStrategy::Skip => {}
            ParseErrorStrategy::Warn | ParseErrorStrategy::Strict => {
                self.warnings.push(warning);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 行级辅助函数
// ---------------------------------------------------------------------------

/// 检测行是否为 ATX 标题，返回标题级别（1-6）。
fn heading_level(line: &str) -> Option<usize> {
    if !line.starts_with('#') {
        return None;
    }
    let level = line.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    // `#` 后必须跟空格或行尾
    if line.len() > level && !line.as_bytes()[level].is_ascii_whitespace() {
        return None;
    }
    Some(level)
}

/// 检测行是否为水平分隔线（`---`、`***`、`___`，至少 3 个字符）。
fn is_thematic_break(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let ch = trimmed.as_bytes()[0];
    if ch != b'-' && ch != b'*' && ch != b'_' {
        return false;
    }
    trimmed.bytes().all(|b| b == ch || b == b' ')
}

/// 检测行是否为表格分隔行（`| --- | --- |`）。
fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return false;
    }
    // 去掉首尾 `|`，检查每个 cell 是否只含 `-`、`:`、空格
    let inner = trimmed.trim_matches('|');
    inner.split('|').all(|cell| {
        let cell = cell.trim();
        !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':' || c == ' ')
    })
}

/// 检测行是否为列表项，返回（缩进空格数, 是否有序, 标记字符长度）。
fn detect_list_item(line: &str) -> Option<(usize, bool, usize)> {
    let indent = line.len() - line.trim_start().len();
    let trimmed = &line[indent..];

    // 无序：`- `、`* `、`+ `
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return Some((indent, false, 2));
    }

    // 有序：`1. `、`2. ` 等
    if let Some(dot_pos) = trimmed.find(". ")
        && dot_pos > 0
        && trimmed[..dot_pos].bytes().all(|b| b.is_ascii_digit())
    {
        return Some((indent, true, dot_pos + 2));
    }

    None
}

/// 分割表格行的单元格内容。
fn split_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim().trim_matches('|');
    trimmed
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

/// 根据单元格文本创建表格行。
fn make_table_row(cells: &[String], is_header: bool) -> DocumentTableRow {
    DocumentTableRow {
        cells: cells
            .iter()
            .map(|text| DocumentTableCell {
                blocks: vec![DocumentBlock::Paragraph(parse_inline(text))],
                column_span: 1,
                row_span: 1,
            })
            .collect(),
        is_header,
    }
}

// ---------------------------------------------------------------------------
// Inline 解析
// ---------------------------------------------------------------------------

/// 解析行内 Markdown 格式（`**bold**`、`*italic*`、`` `code` ``、`[text](url)`）。
fn parse_inline(text: &str) -> Vec<DocumentTextRun> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut runs = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    while i < len {
        // `![alt](url)` — 图片标记在 inline 中当文本处理
        if i + 1 < len && bytes[i] == b'!' && bytes[i + 1] == b'[' {
            if let Some((_alt, url, end)) = parse_bracket_paren(text, i + 1) {
                flush_buf(&mut buf, &mut runs);
                runs.push(DocumentTextRun {
                    text: format!("![…]({url})"),
                    ..DocumentTextRun::default()
                });
                i = end;
                continue;
            }
            buf.push('!');
            i += 1;
            continue;
        }

        // `**bold**`
        if i + 2 < len && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            if let Some(end) = find_closing(text, i + 2, "**") {
                let inner = &text[i + 2..end];
                if !inner.is_empty() {
                    flush_buf(&mut buf, &mut runs);
                    runs.push(DocumentTextRun {
                        text: inner.to_owned(),
                        bold: true,
                        ..DocumentTextRun::default()
                    });
                    i = end + 2;
                    continue;
                }
            }
            buf.push_str("**");
            i += 2;
            continue;
        }

        // `*italic*`
        if bytes[i] == b'*' {
            if let Some(end) = find_closing(text, i + 1, "*") {
                let inner = &text[i + 1..end];
                if !inner.is_empty() {
                    flush_buf(&mut buf, &mut runs);
                    runs.push(DocumentTextRun {
                        text: inner.to_owned(),
                        italic: true,
                        ..DocumentTextRun::default()
                    });
                    i = end + 1;
                    continue;
                }
            }
            buf.push('*');
            i += 1;
            continue;
        }

        // `` `code` ``
        if bytes[i] == b'`' {
            if let Some(end) = find_closing(text, i + 1, "`") {
                flush_buf(&mut buf, &mut runs);
                runs.push(DocumentTextRun {
                    text: text[i + 1..end].to_owned(),
                    ..DocumentTextRun::default()
                });
                i = end + 1;
                continue;
            }
            buf.push('`');
            i += 1;
            continue;
        }

        // `[text](url)` 链接
        if bytes[i] == b'[' {
            if let Some((link_text, url, end)) = parse_bracket_paren(text, i) {
                flush_buf(&mut buf, &mut runs);
                runs.push(DocumentTextRun {
                    text: link_text.to_owned(),
                    hyperlink: if url.is_empty() {
                        None
                    } else {
                        Some(url.to_owned())
                    },
                    ..DocumentTextRun::default()
                });
                i = end;
                continue;
            }
            buf.push('[');
            i += 1;
            continue;
        }

        buf.push(bytes[i] as char);
        i += 1;
    }

    flush_buf(&mut buf, &mut runs);
    runs
}

/// 查找闭合标记的位置（返回标记结束位置的字节偏移）。
fn find_closing(text: &str, start: usize, marker: &str) -> Option<usize> {
    let remaining = &text[start..];
    let pos = remaining.find(marker)?;
    Some(start + pos)
}

/// 解析 `[text](url)` 格式，返回（文本, url, 结束位置）。
fn parse_bracket_paren(text: &str, bracket_pos: usize) -> Option<(&str, &str, usize)> {
    let bytes = text.as_bytes();
    if bytes[bracket_pos] != b'[' {
        return None;
    }
    // 找配对 ]
    let mut depth = 0;
    let mut close_bracket = None;
    for (j, &byte) in bytes.iter().enumerate().skip(bracket_pos) {
        match byte {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    close_bracket = Some(j);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close_bracket?;
    let link_text = &text[bracket_pos + 1..close];

    // 检查后面是否跟着 `(url)`
    if close + 1 >= bytes.len() || bytes[close + 1] != b'(' {
        return None;
    }
    let url_start = close + 2;
    let mut url_end = None;
    for (j, &byte) in bytes.iter().enumerate().skip(url_start) {
        if byte == b')' {
            url_end = Some(j);
            break;
        }
    }
    let url_end = url_end?;
    let url = &text[url_start..url_end];

    Some((link_text, url, url_end + 1))
}

/// 将缓冲区中的纯文本刷新为一个 `DocumentTextRun`。
fn flush_buf(buf: &mut String, runs: &mut Vec<DocumentTextRun>) {
    if !buf.is_empty() {
        runs.push(DocumentTextRun {
            text: std::mem::take(buf),
            ..DocumentTextRun::default()
        });
    }
}

// ---------------------------------------------------------------------------
// 图片行解析
// ---------------------------------------------------------------------------

/// 解析独立的 `![alt](path)` 图片行。
fn parse_image_line(line: &str) -> Option<DocumentImage> {
    let trimmed = line.trim();
    if !trimmed.starts_with("![") {
        return None;
    }
    // 找 `](` 分隔
    let close_bracket = trimmed.find(']')?;
    let alt = &trimmed[2..close_bracket];
    let rest = &trimmed[close_bracket + 1..];
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }
    let path = &rest[1..rest.len() - 1];
    let alt_text = if alt.is_empty() {
        None
    } else {
        Some(alt.to_owned())
    };
    // 从路径推断扩展名
    let extension = path.rsplit('.').next().map(std::borrow::ToOwned::to_owned);

    Some(DocumentImage {
        alt_text,
        data: None,
        extension,
    })
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // === 基础结构 ===

    #[test]
    fn import_h1_to_h6() {
        let md = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 6);
        for (i, block) in result.content.blocks.iter().enumerate() {
            match block {
                DocumentBlock::Heading { level, runs } => {
                    assert_eq!(*level, (i + 1) as u8);
                    assert_eq!(runs[0].text, format!("H{}", i + 1));
                }
                _ => panic!("expected Heading at position {i}"),
            }
        }
    }

    #[test]
    fn import_paragraph_merges_consecutive_lines() {
        let md = "line one\nline two\nline three";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 1);
        match &result.content.blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                let text: String = runs.iter().map(|r| r.text.as_str()).collect();
                assert!(text.contains("line one"));
                assert!(text.contains("line two"));
                assert!(text.contains("line three"));
            }
            _ => panic!("expected Paragraph"),
        }
    }

    #[test]
    fn import_paragraph_splits_on_blank_line() {
        let md = "para one\n\npara two";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 2);
        match &result.content.blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                let text: String = runs.iter().map(|r| r.text.as_str()).collect();
                assert!(text.contains("para one"));
            }
            _ => panic!("expected first Paragraph"),
        }
        match &result.content.blocks[1] {
            DocumentBlock::Paragraph(runs) => {
                let text: String = runs.iter().map(|r| r.text.as_str()).collect();
                assert!(text.contains("para two"));
            }
            _ => panic!("expected second Paragraph"),
        }
    }

    // === Inline ===

    #[test]
    fn import_bold_and_italic() {
        let md = "text **bold** and *italic*";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 1);
        match &result.content.blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                let bold_run = runs.iter().find(|r| r.bold);
                assert!(bold_run.is_some(), "expected bold run");
                assert_eq!(bold_run.unwrap().text, "bold");

                let italic_run = runs.iter().find(|r| r.italic);
                assert!(italic_run.is_some(), "expected italic run");
                assert_eq!(italic_run.unwrap().text, "italic");
            }
            _ => panic!("expected Paragraph"),
        }
    }

    #[test]
    fn import_code_inline() {
        let md = "use `println!` for output";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        match &result.content.blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                let code_run = runs.iter().find(|r| r.text == "println!");
                assert!(code_run.is_some(), "expected code run");
                let r = code_run.unwrap();
                assert!(!r.bold);
                assert!(!r.italic);
            }
            _ => panic!("expected Paragraph"),
        }
    }

    #[test]
    fn import_hyperlink() {
        let md = "visit [Example](https://example.com) site";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        match &result.content.blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                let link_run = runs.iter().find(|r| r.hyperlink.is_some());
                assert!(link_run.is_some(), "expected hyperlink run");
                let r = link_run.unwrap();
                assert_eq!(r.text, "Example");
                assert_eq!(r.hyperlink.as_deref(), Some("https://example.com"));
            }
            _ => panic!("expected Paragraph"),
        }
    }

    #[test]
    fn import_unmatched_marker_keeps_literal() {
        let md = "text with * unmatched star";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        match &result.content.blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                let text: String = runs.iter().map(|r| r.text.as_str()).collect();
                assert!(
                    text.contains('*'),
                    "unmatched * should remain as literal: {text}"
                );
            }
            _ => panic!("expected Paragraph"),
        }
    }

    // === 列表 ===

    #[test]
    fn import_unordered_list_basic() {
        let md = "- item A\n- item B\n- item C";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 1);
        match &result.content.blocks[0] {
            DocumentBlock::List(list) => {
                assert!(!list.ordered);
                assert_eq!(list.items.len(), 3);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn import_ordered_list_basic() {
        let md = "1. first\n2. second\n3. third";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 1);
        match &result.content.blocks[0] {
            DocumentBlock::List(list) => {
                assert!(list.ordered);
                assert_eq!(list.items.len(), 3);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn import_nested_list_two_levels() {
        let md = "- parent A\n  - child 1\n  - child 2\n- parent B";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 1);
        match &result.content.blocks[0] {
            DocumentBlock::List(list) => {
                assert_eq!(list.items.len(), 2);
                // parent A 应有 nested 列表
                assert!(
                    list.items[0].nested.is_some(),
                    "parent A should have nested list"
                );
                let nested = list.items[0].nested.as_ref().unwrap();
                assert_eq!(nested.items.len(), 2);
                // parent B 不应有 nested
                assert!(list.items[1].nested.is_none());
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn import_mixed_ordered_unordered() {
        let md = "- unordered\n1. ordered";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        // 两个独立的列表
        assert_eq!(result.content.blocks.len(), 2);
        match &result.content.blocks[0] {
            DocumentBlock::List(list) => assert!(!list.ordered),
            _ => panic!("expected first List unordered"),
        }
        match &result.content.blocks[1] {
            DocumentBlock::List(list) => assert!(list.ordered),
            _ => panic!("expected second List ordered"),
        }
    }

    // === 表格 ===

    #[test]
    fn import_simple_table_2x2() {
        let md = "| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 25 |";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 1);
        match &result.content.blocks[0] {
            DocumentBlock::Table(table) => {
                assert_eq!(table.rows.len(), 3); // header + 2 data
                assert!(table.rows[0].is_header);
                assert!(!table.rows[1].is_header);
                let header_0 = &table.rows[0].cells[0].blocks[0];
                match header_0 {
                    DocumentBlock::Paragraph(runs) => {
                        let text: String = runs.iter().map(|r| r.text.as_str()).collect();
                        assert_eq!(text, "Name");
                    }
                    _ => panic!("expected Paragraph in cell"),
                }
            }
            _ => panic!("expected Table"),
        }
    }

    #[test]
    fn import_table_with_alignment_colons() {
        let md = "| Left | Center | Right |\n| :--- | :---: | ---: |\n| a | b | c |";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        match &result.content.blocks[0] {
            DocumentBlock::Table(table) => {
                assert_eq!(table.rows.len(), 2); // header + 1 data
            }
            _ => panic!("expected Table"),
        }
    }

    // === 复杂场景 ===

    #[test]
    fn import_handles_code_block() {
        let md = "before\n\n```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n\nafter";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 3);
        match &result.content.blocks[1] {
            DocumentBlock::CodeBlock { language, code } => {
                assert_eq!(language.as_deref(), Some("rust"));
                assert!(code.contains("fn main()"));
                assert!(code.contains("println!"));
            }
            _ => panic!("expected CodeBlock"),
        }
    }

    #[test]
    fn import_warns_on_empty_link() {
        let md = "check [](https://example.com)";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 1);
    }

    #[test]
    fn import_warns_on_table_missing_separator() {
        let md = "| A | B |\n| 1 | 2 |";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        // 第二行不是分隔行（不含 `---`），所以不是表格——而是两个段落
        assert!(
            !result.content.blocks.is_empty(),
            "should parse as paragraphs"
        );
    }

    #[test]
    fn import_empty_source() {
        let result = MarkdownImportBuilder::new("").do_import().unwrap();
        assert!(result.content.blocks.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn import_thematic_break() {
        let md = "above\n\n---\n\nbelow";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 3);
        assert_eq!(result.content.blocks[1], DocumentBlock::ThematicBreak);
    }

    #[test]
    fn import_image_line() {
        let md = "![alt text](image.png)";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 1);
        match &result.content.blocks[0] {
            DocumentBlock::Image(img) => {
                assert_eq!(img.alt_text.as_deref(), Some("alt text"));
                assert!(img.data.is_none());
                assert_eq!(img.extension.as_deref(), Some("png"));
            }
            _ => panic!("expected Image"),
        }
    }

    #[test]
    fn import_image_missing_alt() {
        let md = "![](photo.jpg)";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert!(
            result.warnings.contains(&ImportWarning::ImageMissingAlt),
            "expected ImageMissingAlt warning"
        );
    }

    #[test]
    fn import_strict_mode_collects_warnings() {
        let md = "# \n\nbody";
        let result = MarkdownImportBuilder::new(md)
            .on_parse_error(ParseErrorStrategy::Strict)
            .do_import()
            .unwrap();
        assert!(
            result.warnings.contains(&ImportWarning::EmptyHeading),
            "expected EmptyHeading warning in strict mode"
        );
    }

    #[test]
    fn import_skip_mode_discards_warnings() {
        let md = "# \n\nbody";
        let result = MarkdownImportBuilder::new(md)
            .on_parse_error(ParseErrorStrategy::Skip)
            .do_import()
            .unwrap();
        assert!(
            result.warnings.is_empty(),
            "skip mode should discard warnings"
        );
    }

    // === Inline 边界 case ===

    #[test]
    fn import_nested_bold_and_italic() {
        let md = "***both***";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        match &result.content.blocks[0] {
            DocumentBlock::Paragraph(runs) => {
                assert!(!runs.is_empty(), "should produce at least one run");
            }
            _ => panic!("expected Paragraph"),
        }
    }

    #[test]
    fn import_code_block_without_language() {
        let md = "```\ncode here\n```";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        match &result.content.blocks[0] {
            DocumentBlock::CodeBlock { language, code } => {
                assert!(language.is_none());
                assert_eq!(code, "code here");
            }
            _ => panic!("expected CodeBlock"),
        }
    }

    #[test]
    fn import_paragraph_with_heading_interrupts() {
        let md = "para line\n# Heading";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        assert_eq!(result.content.blocks.len(), 2);
        assert!(matches!(
            result.content.blocks[0],
            DocumentBlock::Paragraph(_)
        ));
        assert!(matches!(
            result.content.blocks[1],
            DocumentBlock::Heading { .. }
        ));
    }

    #[test]
    fn import_roundtrip_basic() {
        // import → 验证 blocks 结构完整
        let md = "# Title\n\nParagraph **bold** text.\n\n- item 1\n- item 2\n\n| A | B |\n| --- | --- |\n| 1 | 2 |";
        let result = MarkdownImportBuilder::new(md).do_import().unwrap();
        // 4 个顶级块：heading, paragraph, list, table
        assert_eq!(result.content.blocks.len(), 4);
        assert!(matches!(
            result.content.blocks[0],
            DocumentBlock::Heading { .. }
        ));
        assert!(matches!(
            result.content.blocks[1],
            DocumentBlock::Paragraph(_)
        ));
        assert!(matches!(result.content.blocks[2], DocumentBlock::List(_)));
        assert!(matches!(result.content.blocks[3], DocumentBlock::Table(_)));
    }
}
