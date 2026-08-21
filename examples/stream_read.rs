//! SAX streaming read: implement a custom `EventSink` to count document block types.

use easydoc::prelude::*;
use easydoc::{DocumentEvent, EasyDoc, EventSink};
use std::collections::HashMap;
use tempfile::TempDir;

/// A collector that counts occurrences of each `DocumentEvent` variant.
struct CountingSink {
    counts: HashMap<String, usize>,
}

impl CountingSink {
    fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    fn record(&mut self, label: &str) {
        *self.counts.entry(label.to_owned()).or_insert(0) += 1;
    }
}

impl EventSink for CountingSink {
    fn on_event(&mut self, event: &DocumentEvent) -> easydoc::Result<()> {
        match event {
            DocumentEvent::DocumentStart => self.record("DocumentStart"),
            DocumentEvent::DocumentEnd => self.record("DocumentEnd"),
            DocumentEvent::Heading { .. } => self.record("Heading"),
            DocumentEvent::Paragraph(_) => self.record("Paragraph"),
            DocumentEvent::Table(_) => self.record("Table"),
            DocumentEvent::List(_) => self.record("List"),
            DocumentEvent::Image(_) => self.record("Image"),
            DocumentEvent::PageBreak => self.record("PageBreak"),
            DocumentEvent::ColumnBreak => self.record("ColumnBreak"),
            DocumentEvent::CodeBlock { .. } => self.record("CodeBlock"),
            DocumentEvent::Section { .. } => self.record("Section"),
            // 未来新增的事件类型（#[non_exhaustive]）
            _ => self.record("Unknown"),
        }
        Ok(())
    }

    fn on_complete(&mut self) {
        self.record("Complete");
    }
}

fn main() -> easydoc::Result<()> {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("stream_test.docx");

    // Step 1: Create a test document with various block types.
    println!("Step 1: Creating a document with mixed content...");
    EasyDoc::document(&path)
        .title("Stream Test")
        .add_heading("Chapter 1", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("First paragraph."))
        .add_heading("Section 1.1", HeadingLevel::H2)
        .add_paragraph(
            Paragraph::new()
                .add_text("Second paragraph with ")
                .add_run(easydoc::Run::new("bold text").bold()),
        )
        .add_paragraph(Paragraph::new().add_text("Third paragraph."))
        .add_page_break()
        .add_heading("Chapter 2", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("After page break."))
        .save()?;

    println!("  Created: {}", path.display());

    // Step 2: Stream-read the document with our counting sink.
    println!("\nStep 2: Streaming read with CountingSink...");
    let mut sink = CountingSink::new();
    EasyDoc::read_events(&path, &mut sink)?;

    // Step 3: Display the event counts.
    println!("\nStep 3: Event counts:");
    let mut sorted_keys: Vec<&String> = sink.counts.keys().collect();
    sorted_keys.sort();
    for key in &sorted_keys {
        let count = sink.counts[*key];
        println!("  {key}: {count}");
    }

    let total: usize = sink.counts.values().sum();
    println!("\n  Total events: {total}");

    println!("\nDone.");
    Ok(())
}
