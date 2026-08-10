//! Compare all four `ViewMode` renderings of the same document.
//!
//! Demonstrates how `Annotated` mode adds structural markers that are
//! particularly useful for LLM context windows.

use easydoc::prelude::*;
use easydoc::{DocxRow, EasyDoc, TableColumn, ViewMode};
use tempfile::TempDir;

/// Simple item for table demonstration.
#[derive(Debug, Clone)]
struct Item {
    name: String,
    qty: String,
}

impl DocxRow for Item {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| {
            vec![
                TableColumn::new("Name", "name", 0),
                TableColumn::new("Qty", "qty", 1),
            ]
        });
        &SCHEMA
    }
    fn from_row(_row: &easydoc::RowData) -> easydoc::Result<Self> {
        unimplemented!()
    }
    fn from_row_with_converters(
        _row: &easydoc::RowData,
        _registry: &easydoc::ConverterRegistry,
    ) -> easydoc::Result<Self> {
        unimplemented!()
    }
    fn to_row(&self) -> easydoc::Result<Vec<easydoc::CellData>> {
        Ok(vec![
            easydoc::CellData::new(self.name.clone()),
            easydoc::CellData::new(self.qty.clone()),
        ])
    }
    fn to_row_with_converters(
        &self,
        _registry: &easydoc::ConverterRegistry,
    ) -> easydoc::Result<Vec<easydoc::CellData>> {
        self.to_row()
    }
}

fn main() -> easydoc::Result<()> {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("view_modes.docx");

    // Step 1: Build a rich document.
    println!("Step 1: Building a document with headings, paragraphs, and a table...");
    let items = vec![
        Item {
            name: "Widget".into(),
            qty: "10".into(),
        },
        Item {
            name: "Gadget".into(),
            qty: "5".into(),
        },
    ];

    EasyDoc::document(&path)
        .title("View Mode Demo")
        .add_heading("Introduction", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("This document demonstrates the four view modes."))
        .add_heading("Details", HeadingLevel::H2)
        .add_paragraph(
            Paragraph::new()
                .add_text("Each mode renders the document differently. ")
                .add_run(easydoc::Run::new("Annotated").bold())
                .add_text(" mode is especially useful for LLMs."),
        )
        .add_table(easydoc::Table::from_data(&items))
        .add_heading("Conclusion", HeadingLevel::H2)
        .add_paragraph(Paragraph::new().add_text("End of document."))
        .save()?;

    println!("  Created: {}", path.display());

    // Step 2: Render in all four modes.
    let modes = [
        ("Plain", ViewMode::Plain),
        ("Annotated", ViewMode::Annotated),
        ("Outline (max_level=3)", ViewMode::Outline { max_level: 3 }),
        ("Stats", ViewMode::Stats),
    ];

    for (label, mode) in &modes {
        println!("\n{}", "=".repeat(60));
        println!("ViewMode: {label}");
        println!("{}", "=".repeat(60));
        let rendered = EasyDoc::view_as(&path, mode)?;
        println!("{rendered}");
    }

    // Step 3: Highlight Annotated mode's LLM benefits.
    println!("\n{}", "=".repeat(60));
    println!("Why Annotated mode is LLM-friendly:");
    println!("{}", "=".repeat(60));
    println!("  - Structural markers like [Title 1], [Paragraph 2] help LLMs");
    println!("    understand document layout without visual formatting.");
    println!("  - Table annotations show dimensions (e.g. [Table 1: 2 rows x 2 cols]),");
    println!("    so LLMs can reason about tabular data structure.");
    println!("  - Unlike Plain mode, Annotated preserves semantic boundaries,");
    println!("    reducing hallucination when the LLM references specific sections.");

    println!("\nDone.");
    Ok(())
}
