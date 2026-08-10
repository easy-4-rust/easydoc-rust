//! Basic document construction: headings, paragraphs, tables, and styled text.

use easydoc::prelude::*;
use easydoc::{DocxRow, EasyDoc, Run, TableColumn};
use tempfile::TempDir;

/// Employee struct for table demo.
#[derive(Debug, Clone)]
struct Employee {
    name: String,
    department: String,
}

impl DocxRow for Employee {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| {
            vec![
                TableColumn::new("Name", "name", 0),
                TableColumn::new("Department", "department", 1),
            ]
        });
        &SCHEMA
    }

    fn from_row(_row: &easydoc::RowData) -> easydoc::Result<Self> {
        unimplemented!("not needed for write example")
    }
    fn from_row_with_converters(
        _row: &easydoc::RowData,
        _registry: &easydoc::ConverterRegistry,
    ) -> easydoc::Result<Self> {
        unimplemented!("not needed for write example")
    }

    fn to_row(&self) -> easydoc::Result<Vec<easydoc::CellData>> {
        Ok(vec![
            easydoc::CellData::new(self.name.clone()),
            easydoc::CellData::new(self.department.clone()),
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
    let path = dir.path().join("write_basic.docx");

    println!("Step 1: Building a document with headings, paragraphs, and a table...");

    let employees = vec![
        Employee {
            name: "Alice".into(),
            department: "Engineering".into(),
        },
        Employee {
            name: "Bob".into(),
            department: "Marketing".into(),
        },
        Employee {
            name: "Charlie".into(),
            department: "Sales".into(),
        },
    ];

    EasyDoc::document(&path)
        .title("Company Report")
        .author("easydoc-rust")
        .add_heading("Company Report", HeadingLevel::H1)
        .add_heading("Overview", HeadingLevel::H2)
        .add_paragraph(
            Paragraph::new()
                .add_text("This document demonstrates ")
                .add_run(Run::new("basic writing").bold())
                .add_text(" capabilities of easydoc-rust."),
        )
        .add_paragraph(Paragraph::new().add_text("The following table lists our team members:"))
        .add_heading("Team Members", HeadingLevel::H3)
        .add_table(easydoc::Table::from_data(&employees).banded_rows(true))
        .add_page_break()
        .add_heading("Summary", HeadingLevel::H2)
        .add_paragraph(
            Paragraph::new()
                .add_text("Total employees: ")
                .add_run(Run::new("3").bold().color(0x008800)),
        )
        .save()?;

    println!("  Saved to: {}", path.display());
    println!("  File size: {} bytes", std::fs::metadata(&path)?.len());

    // Verify by reading back
    println!("\nStep 2: Verifying by reading the document back...");
    let text = EasyDoc::read_text(&path)?;
    println!(
        "  Contains 'Company Report': {}",
        text.contains("Company Report")
    );
    println!("  Contains 'Alice': {}", text.contains("Alice"));
    println!("  Contains 'Engineering': {}", text.contains("Engineering"));

    println!("\nDone.");
    Ok(())
}
