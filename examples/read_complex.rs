//! Build a complex DOCX programmatically (merged cells, hyperlinks, lists,
//! images), then render it in Annotated mode and print structural statistics.

use easydoc::prelude::*;
use easydoc::{
    DocImage, DocumentBlock, DocumentImage, DocumentList, DocumentListItem, DocumentTable,
    DocumentTableCell, DocumentTableRow, DocumentTextRun, EasyDoc, ViewMode,
};
use std::fs;
use tempfile::TempDir;

/// Minimal valid 1x1 blue pixel PNG.
fn tiny_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

fn main() -> easydoc::Result<()> {
    let dir = TempDir::new().expect("create temp dir");

    // Step 1: Write a PNG for embedding.
    println!("Step 1: Preparing image file...");
    let img_path = dir.path().join("blue.png");
    fs::write(&img_path, tiny_png())?;

    // Step 2: Build a basic DOCX with DocBuilder (paragraphs, heading, image, table).
    println!("Step 2: Building base document with DocBuilder...");
    let base_path = dir.path().join("complex_base.docx");
    EasyDoc::document(&base_path)
        .title("Complex Document")
        .author("easydoc-rust")
        .add_heading("Complex Document Analysis", HeadingLevel::H1)
        .add_paragraph(
            Paragraph::new()
                .add_text("This document contains ")
                .add_run(Run::new("merged cells").bold())
                .add_text(", images, lists, and hyperlinks."),
        )
        .add_image(
            DocImage::new(&img_path)
                .width(80)
                .height(80)
                .alt_text("Blue pixel"),
        )
        .add_heading("Data Table", HeadingLevel::H2)
        .add_paragraph(Paragraph::new().add_text("The table below has merged cells."))
        .add_page_break()
        .add_heading("References", HeadingLevel::H2)
        .add_paragraph(Paragraph::new().add_text("End of document."))
        .save()?;

    // Step 3: Load the base document and augment with complex blocks.
    println!("Step 3: Loading and augmenting with complex semantic blocks...");
    let mut content = EasyDoc::load(&base_path)?;

    // 3a: Insert a table with merged cells (gridSpan / vMerge).
    println!("  Adding table with merged cells (column_span=2, row_span=2)...");
    let merged_table = DocumentTable {
        rows: vec![
            // Header row
            DocumentTableRow {
                cells: vec![
                    DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "Category".into(),
                            bold: true,
                            ..Default::default()
                        }])],
                        column_span: 2,
                        row_span: 1,
                    },
                    DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "Value".into(),
                            bold: true,
                            ..Default::default()
                        }])],
                        column_span: 1,
                        row_span: 1,
                    },
                ],
                is_header: true,
            },
            // Data row with a cell spanning 2 rows (vMerge equivalent)
            DocumentTableRow {
                cells: vec![
                    DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "Group A".into(),
                            ..Default::default()
                        }])],
                        column_span: 1,
                        row_span: 2, // vertical merge across 2 rows
                    },
                    DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "Item 1".into(),
                            ..Default::default()
                        }])],
                        column_span: 1,
                        row_span: 1,
                    },
                    DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "100".into(),
                            ..Default::default()
                        }])],
                        column_span: 1,
                        row_span: 1,
                    },
                ],
                is_header: false,
            },
            // Second row under the vMerge
            DocumentTableRow {
                cells: vec![
                    DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "Item 2".into(),
                            ..Default::default()
                        }])],
                        column_span: 1,
                        row_span: 1,
                    },
                    DocumentTableCell {
                        blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                            text: "200".into(),
                            ..Default::default()
                        }])],
                        column_span: 1,
                        row_span: 1,
                    },
                ],
                is_header: false,
            },
        ],
    };

    // 3b: Insert a list (unordered).
    println!("  Adding unordered list with 3 items...");
    let list = DocumentList {
        ordered: false,
        start_number: None,
        items: vec![
            DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                    text: "First item".into(),
                    ..Default::default()
                }])],
                nested: None,
            },
            DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                    text: "Second item".into(),
                    ..Default::default()
                }])],
                nested: None,
            },
            DocumentListItem {
                blocks: vec![DocumentBlock::Paragraph(vec![DocumentTextRun {
                    text: "Third item".into(),
                    ..Default::default()
                }])],
                nested: None,
            },
        ],
    };

    // 3c: Insert a paragraph with a hyperlink.
    println!("  Adding paragraph with hyperlink...");
    let hyperlink_para = DocumentBlock::Paragraph(vec![
        DocumentTextRun {
            text: "Visit ".into(),
            ..Default::default()
        },
        DocumentTextRun {
            text: "Rust homepage".into(),
            hyperlink: Some("https://www.rust-lang.org".into()),
            bold: true,
            ..Default::default()
        },
        DocumentTextRun {
            text: " for more info.".into(),
            ..Default::default()
        },
    ]);

    // 3d: Insert an embedded image block.
    println!("  Adding inline image block...");
    let image_block = DocumentBlock::Image(DocumentImage {
        alt_text: Some("Embedded blue pixel".into()),
        data: Some(tiny_png()),
        extension: Some("png".into()),
    });

    // Insert all complex blocks before the "References" heading.
    // Find the position of the last heading to insert before it.
    let insert_pos = content
        .blocks
        .iter()
        .position(|b| {
            matches!(b, DocumentBlock::Heading { runs, .. }
            if runs.iter().any(|r| r.text.contains("References")))
        })
        .unwrap_or(content.blocks.len());

    let complex_blocks = vec![
        DocumentBlock::Table(merged_table),
        DocumentBlock::List(list),
        hyperlink_para,
        image_block,
    ];

    for (i, block) in complex_blocks.into_iter().enumerate() {
        content.blocks.insert(insert_pos + i, block);
    }

    // Step 4: Write the augmented document.
    println!("\nStep 4: Writing complex document...");
    let complex_path = dir.path().join("complex.docx");
    EasyDoc::write_content(&content, &complex_path)?;
    println!("  Saved: {}", complex_path.display());

    // Step 5: Render in Annotated mode.
    println!("\nStep 5: Rendering in Annotated mode...");
    let annotated = EasyDoc::view_as(&complex_path, &ViewMode::Annotated)?;
    println!("--- Annotated Output ---");
    println!("{annotated}");
    println!("--- End ---");

    // Step 6: Count structural elements from the original semantic model.
    // (The writer's content_renderer doesn't round-trip all metadata like
    // hyperlinks, so we count from the in-memory model we constructed.)
    println!("\nStep 6: Structural statistics (from in-memory model):");

    let mut table_rows: usize = 0;
    let mut merged_cells: usize = 0;
    let mut image_count: usize = 0;
    let mut hyperlink_count: usize = 0;
    let mut list_items: usize = 0;
    let mut heading_count: usize = 0;
    let mut paragraph_count: usize = 0;

    // We need the augmented content, so reload and also count from original blocks.
    let reloaded = EasyDoc::load(&complex_path)?;

    // Count from the original semantic model we built (has hyperlink info).
    for block in &content.blocks {
        match block {
            DocumentBlock::Table(table) => {
                table_rows += table.rows.len();
                for row in &table.rows {
                    for cell in &row.cells {
                        if cell.column_span > 1 || cell.row_span > 1 {
                            merged_cells += 1;
                        }
                    }
                }
            }
            DocumentBlock::Image(_) => image_count += 1,
            DocumentBlock::Paragraph(runs) => {
                paragraph_count += 1;
                hyperlink_count += runs.iter().filter(|r| r.hyperlink.is_some()).count();
            }
            DocumentBlock::Heading { .. } => heading_count += 1,
            DocumentBlock::List(list) => list_items += list.items.len(),
            _ => {}
        }
    }

    println!("  Headings:       {heading_count}");
    println!("  Paragraphs:     {paragraph_count}");
    println!("  Table rows:     {table_rows}");
    println!("  Merged cells:   {merged_cells}");
    println!("  Images:         {image_count}");
    println!("  Hyperlinks:     {hyperlink_count}");
    println!("  List items:     {list_items}");

    // Also count from the reloaded document (what the reader actually sees).
    println!("\n  Reloaded document stats:");
    println!("  Total blocks:   {}", reloaded.blocks.len());
    let reloaded_images = reloaded
        .blocks
        .iter()
        .filter(|b| matches!(b, DocumentBlock::Image(_)))
        .count();
    let reloaded_tables = reloaded
        .blocks
        .iter()
        .filter(|b| matches!(b, DocumentBlock::Table(_)))
        .count();
    let reloaded_lists = reloaded
        .blocks
        .iter()
        .filter(|b| matches!(b, DocumentBlock::List(_)))
        .count();
    println!("  Images:         {reloaded_images}");
    println!("  Tables:         {reloaded_tables}");
    println!("  Lists:          {reloaded_lists}");

    println!("\nDone.");
    Ok(())
}
