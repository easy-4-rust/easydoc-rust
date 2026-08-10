//! Basic reading: write a DOCX in memory, then read its text and tables.

use easydoc::prelude::*;
use easydoc::{DocxRow, EasyDoc, TableColumn};
use tempfile::TempDir;

/// A simple product row for table demonstration.
#[derive(Debug, Clone)]
struct Product {
    name: String,
    price: String,
}

impl DocxRow for Product {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: std::sync::LazyLock<Vec<TableColumn>> = std::sync::LazyLock::new(|| {
            vec![
                TableColumn::new("Name", "name", 0),
                TableColumn::new("Price", "price", 1),
            ]
        });
        &SCHEMA
    }

    fn from_row(row: &easydoc::RowData) -> easydoc::Result<Self> {
        Ok(Product {
            name: match &row.cells.first() {
                Some(cell) => match &cell.value {
                    DocValue::String(s) => s.clone(),
                    other => format!("{other:?}"),
                },
                None => String::new(),
            },
            price: match row.cells.get(1) {
                Some(cell) => match &cell.value {
                    DocValue::String(s) => s.clone(),
                    other => format!("{other:?}"),
                },
                None => String::new(),
            },
        })
    }

    fn from_row_with_converters(
        row: &easydoc::RowData,
        _registry: &easydoc::ConverterRegistry,
    ) -> easydoc::Result<Self> {
        Self::from_row(row)
    }

    fn to_row(&self) -> easydoc::Result<Vec<easydoc::CellData>> {
        Ok(vec![
            easydoc::CellData::new(self.name.clone()),
            easydoc::CellData::new(self.price.clone()),
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
    let path = dir.path().join("read_basic.docx");

    // Step 1: Create a document with paragraphs and a table.
    println!("Step 1: Writing a DOCX with paragraphs and a table...");
    let products = vec![
        Product {
            name: "Widget".into(),
            price: "$9.99".into(),
        },
        Product {
            name: "Gadget".into(),
            price: "$24.99".into(),
        },
    ];

    EasyDoc::document(&path)
        .title("Product Catalog")
        .add_heading("Product Catalog", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("Available products:"))
        .add_table(easydoc::Table::from_data(&products))
        .save()?;

    println!("  Written to: {}", path.display());

    // Step 2: Read all text from the document.
    println!("\nStep 2: Reading full text...");
    let text = EasyDoc::read_text(&path)?;
    println!("  Text content:\n---\n{text}\n---");

    // Step 3: Read all tables as typed structs.
    println!("\nStep 3: Reading tables as typed structs...");
    let tables: Vec<Vec<Product>> = EasyDoc::read_tables::<Product>(&path)?;
    println!("  Found {} table(s)", tables.len());
    for (i, table) in tables.iter().enumerate() {
        println!("  Table {}: {} row(s)", i, table.len());
        for row in table {
            println!("    - {} : {}", row.name, row.price);
        }
    }

    println!("\nDone.");
    Ok(())
}
