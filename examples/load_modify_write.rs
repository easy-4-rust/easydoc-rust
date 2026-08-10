//! Semantic model round-trip: load a DOCX, modify its blocks, and write it back.

use easydoc::prelude::*;
use easydoc::{DocumentBlock, DocumentContent, DocumentTextRun, EasyDoc};
use tempfile::TempDir;

fn main() -> easydoc::Result<()> {
    let dir = TempDir::new().expect("create temp dir");
    let original = dir.path().join("original.docx");
    let modified = dir.path().join("modified.docx");

    // Step 1: Create an initial document.
    println!("Step 1: Creating original document...");
    EasyDoc::document(&original)
        .title("Original Document")
        .add_heading("Original Title", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("This is the original content."))
        .add_paragraph(Paragraph::new().add_text("Keep this paragraph unchanged."))
        .save()?;

    let text = EasyDoc::read_text(&original)?;
    println!("  Original text:\n---\n{text}\n---");

    // Step 2: Load the document into the semantic model.
    println!("\nStep 2: Loading semantic model...");
    let mut content: DocumentContent = EasyDoc::load(&original)?;
    println!("  Loaded {} blocks", content.blocks.len());

    // Step 3: Modify the model.
    println!("\nStep 3: Modifying the document...");
    // 3a: Change the heading text.
    for block in &mut content.blocks {
        if let DocumentBlock::Heading { runs, .. } = block {
            for run in runs.iter_mut() {
                if run.text.contains("Original Title") {
                    println!("  Replacing heading text: 'Original Title' -> 'Modified Title'");
                    run.text = run.text.replace("Original Title", "Modified Title");
                }
            }
        }
    }

    // 3b: Add a new paragraph at the end.
    println!("  Adding a new paragraph at the end.");
    content
        .blocks
        .push(DocumentBlock::Paragraph(vec![DocumentTextRun {
            text: "This paragraph was added during modification.".into(),
            bold: true,
            ..Default::default()
        }]));

    // Step 4: Write the modified content to a new file.
    println!("\nStep 4: Writing modified document...");
    EasyDoc::write_content(&content, &modified)?;

    // Step 5: Verify the result.
    println!("\nStep 5: Verifying modified document...");
    let modified_text = EasyDoc::read_text(&modified)?;
    println!("  Modified text:\n---\n{modified_text}\n---");

    let has_new_title = modified_text.contains("Modified Title");
    let has_added = modified_text.contains("added during modification");
    let has_unchanged = modified_text.contains("Keep this paragraph unchanged");

    println!("\n  Contains 'Modified Title': {has_new_title}");
    println!("  Contains added paragraph: {has_added}");
    println!("  Contains unchanged paragraph: {has_unchanged}");

    if has_new_title && has_added && has_unchanged {
        println!("\n  Round-trip verified successfully!");
    } else {
        println!("\n  WARNING: Some modifications did not persist.");
    }

    println!("\nDone.");
    Ok(())
}
