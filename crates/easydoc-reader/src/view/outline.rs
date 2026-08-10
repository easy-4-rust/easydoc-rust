//! Outline view renderer -- headings only, Markdown-style.

use easydoc_core::{DocumentBlock, DocumentContent};

/// Renders only headings up to `max_level` (inclusive) in Markdown format.
///
/// Level 1 headings become `# Title`, level 2 become `## Title`, etc.
pub fn render(content: &DocumentContent, max_level: u8) -> String {
    let mut out = String::new();
    for block in &content.blocks {
        if let DocumentBlock::Heading { level, runs } = block
            && *level <= max_level
            && *level >= 1
        {
            let hashes = "#".repeat(usize::from(*level));
            let text: String = runs.iter().map(|r| r.text.as_str()).collect();
            out.push_str(&hashes);
            out.push(' ');
            out.push_str(&text);
            out.push('\n');
        }
    }
    out
}
