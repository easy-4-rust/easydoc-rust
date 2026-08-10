//! Convert a DOCX to Markdown using `EasyDoc::to_markdown()`.
//!
//! Demonstrates how headings, bold/italic text, and lists are rendered
//! in the Markdown output.

use easydoc::prelude::*;
use easydoc::{EasyDoc, Run};
use tempfile::TempDir;

fn main() -> easydoc::Result<()> {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("markdown_source.docx");

    // Step 1: Create a document with rich text and a list-like structure.
    println!("Step 1: Creating a document with headings, styled text, and content...");
    EasyDoc::document(&path)
        .title("Markdown Demo")
        .add_heading("Getting Started", HeadingLevel::H1)
        .add_paragraph(
            Paragraph::new()
                .add_text("Welcome to ")
                .add_run(Run::new("easydoc-rust").bold())
                .add_text(", a library for easy DOCX operations."),
        )
        .add_heading("Features", HeadingLevel::H2)
        .add_paragraph(
            Paragraph::new()
                .add_run(Run::new("Read").bold())
                .add_text(" and ")
                .add_run(Run::new("write").italic())
                .add_text(" DOCX documents with a fluent API."),
        )
        .add_paragraph(Paragraph::new().add_text("Supports tables, images, and styled text."))
        .add_heading("Installation", HeadingLevel::H2)
        .add_paragraph(Paragraph::new().add_text("Add to your Cargo.toml:"))
        .add_paragraph(Paragraph::new().add_text("easydoc = \"0.1.0\""))
        .add_heading("Summary", HeadingLevel::H3)
        .add_paragraph(Paragraph::new().add_text("That covers the basics!"))
        .save()?;

    println!("  Created: {}", path.display());

    // Step 2: Convert to Markdown.
    println!("\nStep 2: Converting to Markdown...");
    let markdown = EasyDoc::to_markdown(&path)?;

    // Step 3: Display the result.
    println!("\nStep 3: Markdown output:");
    println!("---");
    println!("{markdown}");
    println!("---");

    // Step 4: Show stats.
    let line_count = markdown.lines().count();
    let char_count = markdown.len();
    println!("\nMarkdown stats: {line_count} lines, {char_count} characters");

    println!("\nDone.");
    Ok(())
}
