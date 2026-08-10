//! Extract images from a DOCX: create a document with embedded images,
//! then load it and save each `DocumentBlock::Image` to disk.

use easydoc::prelude::*;
use easydoc::{DocImage, DocumentBlock, EasyDoc};
use std::fs;
use tempfile::TempDir;

/// Minimal valid 1x1 red pixel PNG (67 bytes).
fn tiny_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // 8-bit RGB
        0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
        0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, // compressed data
        0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, // IEND chunk
        0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

fn main() -> easydoc::Result<()> {
    let dir = TempDir::new().expect("create temp dir");
    let src_path = dir.path().join("with_images.docx");
    let output_dir = dir.path().join("output");
    fs::create_dir_all(&output_dir)?;

    // Step 1: Write a tiny PNG to a temp file for DocImage.
    println!("Step 1: Creating a small PNG image file...");
    let img_path = dir.path().join("tiny.png");
    fs::write(&img_path, tiny_png())?;
    println!(
        "  Written: {} ({} bytes)",
        img_path.display(),
        tiny_png().len()
    );

    // Step 2: Build a DOCX with two images and some text.
    println!("\nStep 2: Building DOCX with embedded images...");
    EasyDoc::document(&src_path)
        .title("Image Extraction Demo")
        .add_heading("Image Extraction", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text("Below are two embedded images:"))
        .add_image(
            DocImage::new(&img_path)
                .width(100)
                .height(100)
                .alt_text("Red pixel 1"),
        )
        .add_paragraph(Paragraph::new().add_text("And another one:"))
        .add_image(
            DocImage::new(&img_path)
                .width(50)
                .height(50)
                .alt_text("Red pixel 2"),
        )
        .save()?;
    println!("  Created: {}", src_path.display());

    // Step 3: Load the document into the semantic model.
    println!("\nStep 3: Loading document with EasyDoc::load...");
    let content = EasyDoc::load(&src_path)?;
    println!("  Total blocks: {}", content.blocks.len());

    // Step 4: Traverse blocks, extract images, and write them to disk.
    println!("\nStep 4: Extracting images to {}...", output_dir.display());
    let mut image_count: usize = 0;
    for block in &content.blocks {
        if let DocumentBlock::Image(image) = block {
            let ext = image.extension.as_deref().unwrap_or("png");
            let filename = format!("{image_count}.{ext}");
            let out_path = output_dir.join(&filename);

            if let Some(data) = &image.data {
                fs::write(&out_path, data)?;
                println!(
                    "  [{}] Saved {} ({} bytes) alt={:?}",
                    image_count,
                    filename,
                    data.len(),
                    image.alt_text
                );
            } else {
                println!(
                    "  [{}] Image has no data (alt={:?})",
                    image_count, image.alt_text
                );
            }
            image_count += 1;
        }
    }

    println!("\n  Total images extracted: {image_count}");
    if image_count > 0 {
        println!("  Output directory: {}", output_dir.display());
    }

    println!("\nDone.");
    Ok(())
}
