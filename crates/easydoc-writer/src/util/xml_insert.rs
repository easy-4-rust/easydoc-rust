//! XML post-processing helpers for attributes not natively supported
//! by `docx-rs` (e.g. `noWrap`, `numFmt`).

/// Inserts `content` immediately after the **n-th** occurrence of `pattern`
/// (0-indexed) inside `xml`.  Returns the modified string, or the original
/// if the pattern occurs fewer than `n + 1` times.
///
/// This is used for targeted XML post-processing where `docx-rs` does not
/// natively emit certain OOXML elements (e.g. `<w:noWrap/>`, `<w:numFmt>`).
///
/// # Examples
///
/// ```
/// use easydoc_writer::util::insert_after_nth;
///
/// let s = "<a/><b/><a/>";
/// let out = insert_after_nth(s, "<a/>", 1, "<!-- insert -->");
/// assert_eq!(out, "<a/><b/><a/><!-- insert -->");
///
/// let first = insert_after_nth(s, "<a/>", 0, "<!-- first -->");
/// assert_eq!(first, "<a/><!-- first --><b/><a/>");
/// ```
#[must_use]
pub fn insert_after_nth(xml: &str, pattern: &str, n: usize, content: &str) -> String {
    let mut offset = 0usize;
    let mut remaining = xml;
    let mut count = 0usize;

    while let Some(pos) = remaining.find(pattern) {
        if count == n {
            let insert_at = offset + pos + pattern.len();
            let mut result = String::with_capacity(xml.len() + content.len());
            result.push_str(&xml[..insert_at]);
            result.push_str(content);
            result.push_str(&xml[insert_at..]);
            return result;
        }
        count += 1;
        let advance = pos + pattern.len();
        offset += advance;
        remaining = &remaining[advance..];
    }

    xml.to_string()
}

/// 在线性时间内将 `contents` 依次插入到 `pattern` 第 `n` 次出现之后。
///
/// 与反复调用 [`insert_after_nth`] 不同（每次 O(n) 扫描，累计 O(n²)），
/// 本函数单次扫描 XML，按序定位所有插入点后一次性构建结果，整体 O(n)。
///
/// `contents` 中的元素按索引对应 `pattern` 的出现序号（0-indexed）；
/// 出现次数不足的条目被忽略。
///
/// # Examples
///
/// ```
/// use easydoc_writer::util::insert_many_after_nth;
///
/// let s = "<a/><b/><a/><b/><a/>";
/// let out = insert_many_after_nth(s, "<b/>", &["!".to_owned(), "?".to_owned()]);
/// assert_eq!(out, "<a/><b/>!<a/><b/>?<a/>");
/// ```
#[must_use]
pub fn insert_many_after_nth(xml: &str, pattern: &str, contents: &[String]) -> String {
    if contents.is_empty() {
        return xml.to_owned();
    }

    let mut result =
        String::with_capacity(xml.len() + contents.iter().map(String::len).sum::<usize>());
    let mut search_from = 0usize;
    let mut occurrence = 0usize;

    while occurrence < contents.len() {
        let Some(rel) = xml[search_from..].find(pattern) else {
            // 剩余内容无更多匹配，追加到结尾
            result.push_str(&xml[search_from..]);
            return result;
        };
        let abs = search_from + rel;
        result.push_str(&xml[search_from..abs + pattern.len()]);
        result.push_str(&contents[occurrence]);
        search_from = abs + pattern.len();
        occurrence += 1;
    }

    result.push_str(&xml[search_from..]);
    result
}

/// Inserts `<w:noWrap/>` into the cell-property XML fragment for a cell
/// whose `wrap` attribute is `false` (i.e. text should **not** wrap).
///
/// Targets either:
/// - `<w:tcW ... />` -- when the cell has an explicit width set, or
/// - `<w:tcPr>` / `<w:tcPr />` -- as a fallback when no width is present.
///
/// In OOXML, the **absence** of `<w:noWrap/>` means wrapping is enabled,
/// so we only need to insert it when wrapping is disabled.
#[must_use]
pub fn insert_no_wrap(xml: &str, cell_index: usize) -> String {
    // Prefer inserting after <w:tcW ... /> when it exists.
    if xml.matches("<w:tcW").count() > cell_index {
        return insert_after_nth(xml, "<w:tcW", cell_index, "<w:noWrap/>");
    }
    // Fallback: insert after <w:tcPr> (covers both empty and non-empty forms).
    if xml.matches("<w:tcPr").count() > cell_index {
        return insert_after_nth(xml, "<w:tcPr", cell_index, "<w:noWrap/>");
    }
    xml.to_string()
}

/// Inserts `<w:numFmt w:val="FORMAT"/>` into the paragraph run-properties
/// of the cell at `cell_index`.
///
/// The number format is placed inside `<w:rPr>` that appears as a direct
/// child of `<w:pPr>` (the paragraph-level default run properties), which
/// is the canonical location for cell-level number formatting in OOXML.
#[must_use]
pub fn insert_num_fmt(xml: &str, cell_index: usize, format: &str) -> String {
    let pattern = "<w:pPr><w:rPr";
    if xml.matches(pattern).count() > cell_index {
        let num_fmt = format!("<w:numFmt w:val=\"{format}\"/>");
        return insert_after_nth(xml, pattern, cell_index, &num_fmt);
    }
    xml.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_after_nth_first() {
        let out = insert_after_nth("AAxBBxAA", "AA", 0, "!");
        assert_eq!(out, "AA!xBBxAA");
    }

    #[test]
    fn insert_after_nth_second() {
        // n=1 means "after the 2nd occurrence" (0-indexed).
        let out = insert_after_nth("AAxBBxAA", "AA", 1, "!");
        assert_eq!(out, "AAxBBxAA!");
    }

    #[test]
    fn insert_after_nth_not_found() {
        let out = insert_after_nth("xBBx", "AA", 0, "!");
        assert_eq!(out, "xBBx");
    }

    #[test]
    fn insert_after_nth_out_of_bounds() {
        let out = insert_after_nth("AA", "AA", 5, "!");
        assert_eq!(out, "AA");
    }

    #[test]
    fn insert_many_basic() {
        let out = insert_many_after_nth("<a/><b/><a/><b/><a/>", "<b/>", &["!".into(), "?".into()]);
        assert_eq!(out, "<a/><b/>!<a/><b/>?<a/>");
    }

    #[test]
    fn insert_many_empty_contents_returns_original() {
        let xml = "<a/><b/><a/>";
        let out = insert_many_after_nth(xml, "<b/>", &[]);
        assert_eq!(out, xml);
    }

    #[test]
    fn insert_many_more_contents_than_matches() {
        // 出现次数不足：多余的 contents 被忽略
        let out = insert_many_after_nth("<a/><b/>", "<b/>", &["!".into(), "?".into()]);
        assert_eq!(out, "<a/><b/>!");
    }

    #[test]
    fn insert_many_no_match_returns_original() {
        let xml = "<a/><a/>";
        let out = insert_many_after_nth(xml, "<b/>", &["!".into()]);
        assert_eq!(out, xml);
    }

    #[test]
    fn insert_many_preserves_remaining_tail() {
        // 插入点之后的内容应完整保留
        let out = insert_many_after_nth("<a/><b/>TAIL", "<b/>", &["!".into()]);
        assert_eq!(out, "<a/><b/>!TAIL");
    }
}
