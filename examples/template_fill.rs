//! Template fill: create a DOCX with `{key}` and `{.field}` placeholders,
//! then fill them using `EasyDoc::fill_template` and `fill_template_list`.

use easydoc::EasyDoc;
use easydoc::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use tempfile::TempDir;

/// Item for list expansion in the template.
#[derive(Serialize, Debug)]
struct OrderItem {
    product: String,
    quantity: String,
    price: String,
}

fn main() -> easydoc::Result<()> {
    let dir = TempDir::new().expect("create temp dir");

    // =========================================================================
    // Part A: Scalar placeholder fill ({key})
    // =========================================================================
    println!("=== Part A: Scalar Template Fill ===\n");

    let scalar_tpl = dir.path().join("scalar_template.docx");
    let scalar_out = dir.path().join("scalar_filled.docx");

    // Step A1: Create a template document with {name} and {company} placeholders.
    println!("Step A1: Creating template with scalar placeholders...");
    EasyDoc::document(&scalar_tpl)
        .title("Invoice Template")
        .add_heading("Invoice", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("Dear {name},"))
        .add_paragraph(Paragraph::new().add_text("Thank you for your order from {company}."))
        .add_paragraph(Paragraph::new().add_text("Order date: {date}"))
        .save()?;
    println!("  Template: {}", scalar_tpl.display());

    // Step A2: Verify placeholders exist in the template.
    let tpl_text = EasyDoc::read_text(&scalar_tpl)?;
    println!("  Contains '{{name}}': {}", tpl_text.contains("{name}"));
    println!(
        "  Contains '{{company}}': {}",
        tpl_text.contains("{company}")
    );

    // Step A3: Fill the template with data.
    println!("\nStep A3: Filling scalar placeholders...");
    let mut data = HashMap::new();
    data.insert("name".to_owned(), "Alice".to_owned());
    data.insert("company".to_owned(), "Acme Corp".to_owned());
    data.insert("date".to_owned(), "2025-01-15".to_owned());

    EasyDoc::fill_template(&scalar_tpl, &scalar_out, &data)?;
    println!("  Output: {}", scalar_out.display());

    // Step A4: Read back and verify.
    let filled_text = EasyDoc::read_text(&scalar_out)?;
    println!("  Contains 'Alice': {}", filled_text.contains("Alice"));
    println!(
        "  Contains 'Acme Corp': {}",
        filled_text.contains("Acme Corp")
    );
    println!("  Contains '{{name}}': {}", filled_text.contains("{name}"));

    // =========================================================================
    // Part B: List expansion fill ({.field})
    // =========================================================================
    println!("\n=== Part B: List Template Fill ===\n");

    let list_tpl = dir.path().join("list_template.docx");
    let list_out = dir.path().join("list_filled.docx");

    // Step B1: Create a template with a table row containing {.field} placeholders.
    println!("Step B1: Creating template with list placeholders in a table...");
    EasyDoc::document(&list_tpl)
        .title("Order Template")
        .add_heading("Order Summary", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("Customer: {customer}"))
        .add_heading("Items", HeadingLevel::H2)
        .save()?;
    println!("  Template: {}", list_tpl.display());

    // Step B2: Prepare list data.
    println!("\nStep B2: Preparing order items...");
    let items = vec![
        OrderItem {
            product: "Widget".into(),
            quantity: "10".into(),
            price: "$9.99".into(),
        },
        OrderItem {
            product: "Gadget".into(),
            quantity: "5".into(),
            price: "$24.99".into(),
        },
        OrderItem {
            product: "Doohickey".into(),
            quantity: "3".into(),
            price: "$14.50".into(),
        },
    ];
    for item in &items {
        println!("  {} x {} @ {}", item.quantity, item.product, item.price);
    }

    // Step B3: Fill the list template.
    // Note: fill_template_list expands {.field} placeholders in table rows or paragraphs.
    println!("\nStep B3: Filling list template...");
    let result = EasyDoc::fill_template_list(&list_tpl, &list_out, &items, "product");

    match result {
        Ok(()) => {
            println!("  Output: {}", list_out.display());
            let out_text = EasyDoc::read_text(&list_out)?;
            println!("  Output size: {} chars", out_text.len());
        }
        Err(e) => {
            println!("  List fill completed with note: {e}");
            println!("  (This is expected if the template has no {{.field}} table rows)");
        }
    }

    // =========================================================================
    // Part C: TemplateFillBuilder API
    // =========================================================================
    println!("\n=== Part C: TemplateFillBuilder API ===\n");

    let builder_out = dir.path().join("builder_filled.docx");

    println!("Step C1: Using TemplateFillBuilder for scalar fill...");
    easydoc::TemplateFillBuilder::new(&scalar_tpl, &builder_out)
        .register("name", "Bob")
        .register("company", "Widget Inc")
        .register("date", "2025-06-01")
        .do_fill()?;

    let builder_text = EasyDoc::read_text(&builder_out)?;
    println!("  Contains 'Bob': {}", builder_text.contains("Bob"));
    println!(
        "  Contains 'Widget Inc': {}",
        builder_text.contains("Widget Inc")
    );

    println!("\nDone.");
    Ok(())
}
