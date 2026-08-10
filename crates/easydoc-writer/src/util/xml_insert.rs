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
}
