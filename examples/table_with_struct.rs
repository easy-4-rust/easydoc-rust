//! Demonstrates `#[derive(DocxRow)]` with annotation attributes for typed table writing.

use chrono::NaiveDate;
use easydoc::DocxRowDerive;
use easydoc::EasyDoc;
use easydoc::prelude::*;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

/// Employee record with derive-mapped columns.
///
/// Each `#[docx(...)]` attribute controls the column header, order, width,
/// alignment, and format for the generated DOCX table.
#[derive(DocxRowDerive, Serialize, Deserialize, Debug)]
struct Employee {
    /// Column header: "Name", leftmost column.
    #[docx(name = "Name", order = 0, width = "3cm")]
    name: String,

    /// Column header: "Age", center-aligned.
    #[docx(name = "Age", order = 1, width = "2cm", align = "center")]
    age: u32,

    /// Column header: "Hire Date", date-formatted.
    #[docx(name = "Hire Date", order = 2, width = "4cm", format = "yyyy-mm-dd")]
    hire_date: NaiveDate,
}

fn main() -> easydoc::Result<()> {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("employees.docx");

    // Step 1: Build employee data.
    println!("Step 1: Creating employee records...");
    let employees = vec![
        Employee {
            name: "Alice".into(),
            age: 30,
            hire_date: NaiveDate::from_ymd_opt(2022, 3, 15).unwrap(),
        },
        Employee {
            name: "Bob".into(),
            age: 25,
            hire_date: NaiveDate::from_ymd_opt(2023, 7, 1).unwrap(),
        },
        Employee {
            name: "Charlie".into(),
            age: 35,
            hire_date: NaiveDate::from_ymd_opt(2021, 1, 10).unwrap(),
        },
    ];

    for emp in &employees {
        println!(
            "  {} | Age: {} | Hired: {}",
            emp.name, emp.age, emp.hire_date
        );
    }

    // Step 2: Write the table to DOCX using the derive-generated schema.
    println!("\nStep 2: Writing table to DOCX...");
    EasyDoc::write_table(&path, &employees)
        .title("Employee Directory")
        .banded_rows(true)
        .do_write()?;

    println!("  Saved to: {}", path.display());
    println!("  File size: {} bytes", std::fs::metadata(&path)?.len());

    // Step 3: Read back and verify.
    println!("\nStep 3: Reading back the table...");
    let text = EasyDoc::read_text(&path)?;
    println!("  Contains 'Alice': {}", text.contains("Alice"));
    println!("  Contains 'Bob': {}", text.contains("Bob"));
    println!("  Contains 'Charlie': {}", text.contains("Charlie"));

    // Step 4: Show the generated schema.
    println!("\nStep 4: Generated column schema:");
    for col in Employee::schema() {
        println!(
            "  [{}] name=\"{}\", width={:?}, format={:?}, align={:?}",
            col.index, col.name, col.width, col.format, col.align
        );
    }

    println!("\nDone.");
    Ok(())
}
