//! Performance benchmarks for `easydoc-rust`.
//!
//! Lints in this file are relaxed: `doc_markdown` is allowed because
//! identifier-style names in benchmark group headers (e.g. `write_throughput`)
//! are intentionally not wrapped in backticks.
//!
#![allow(clippy::doc_markdown, missing_docs)]
//! Modeled after `easyexcel-rust/benchmarks/rust-runner/`.
//! Run with: `cargo bench` or `cargo bench -- --quick` for a fast sanity check.
//!
//! Benchmark groups:
//!   A. write_throughput      — table write at 1K / 10K / 100K rows
//!   B. read_text_throughput  — plain text extraction
//!   C. view_mode_throughput  — ViewMode rendering (Plain / Annotated / Outline / Stats)
//!   D. stream_vs_oneshot     — SAX event streaming vs full DocumentContent load
//!   E. markdown_throughput   — DOCX-to-Markdown conversion

use std::hint::black_box;
use std::sync::LazyLock;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use easydoc::prelude::*;
use easydoc::{ConverterRegistry, DocumentEvent, EasyDoc};
use easydoc_core::metadata::TableColumn;

// ---------------------------------------------------------------------------
// Test row type — manually implements DocxRow (avoids proc-macro dependency)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BenchRow {
    id: u64,
    name: String,
    amount: f64,
    active: bool,
}

impl DocxRow for BenchRow {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: LazyLock<Vec<TableColumn>> = LazyLock::new(|| {
            vec![
                TableColumn::new("ID", "id", 0),
                TableColumn::new("Name", "name", 1),
                TableColumn::new("Amount", "amount", 2),
                TableColumn::new("Active", "active", 3),
            ]
        });
        &SCHEMA
    }

    fn from_row(_row: &RowData) -> easydoc_core::Result<Self> {
        unimplemented!("not used in benchmarks")
    }

    fn from_row_with_converters(
        _row: &RowData,
        _registry: &ConverterRegistry,
    ) -> easydoc_core::Result<Self> {
        unimplemented!("not used in benchmarks")
    }

    fn to_row(&self) -> easydoc_core::Result<Vec<CellData>> {
        Ok(vec![
            CellData::new(i64::try_from(self.id).unwrap_or(i64::MAX)),
            CellData::new(self.name.clone()),
            CellData::new(self.amount),
            CellData::new(self.active),
        ])
    }

    fn to_row_with_converters(
        &self,
        _registry: &ConverterRegistry,
    ) -> easydoc_core::Result<Vec<CellData>> {
        self.to_row()
    }
}

// ---------------------------------------------------------------------------
// Data generators
// ---------------------------------------------------------------------------

/// Generates `n` synthetic rows with realistic field values.
fn generate_rows(n: usize) -> Vec<BenchRow> {
    (0..n as u64)
        .map(|i| BenchRow {
            id: i,
            name: format!("item_{i:06}"),
            amount: (i as f64) * 1.23,
            active: i % 3 != 0,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Fixture: pre-built DOCX file on disk (for read benchmarks)
// ---------------------------------------------------------------------------

struct Fixture {
    _dir: tempfile::TempDir,
    small_docx: std::path::PathBuf,  // ~50 rows, headings, paragraphs
    medium_docx: std::path::PathBuf, // ~1 000 rows
    large_docx: std::path::PathBuf,  // ~5 000 rows
}

static FIXTURE: LazyLock<Fixture> = LazyLock::new(|| {
    let dir = tempfile::tempdir().expect("failed to create temp dir for bench fixtures");

    // Small document: mixed content (headings + paragraphs + table)
    let small_path = dir.path().join("small.docx");
    let small_rows = generate_rows(50);
    EasyDoc::document(&small_path)
        .title("Benchmark Document")
        .add_heading("Introduction", HeadingLevel::H1)
        .add_paragraph(Paragraph::new().add_text(
            "This is a synthetic document generated for benchmarking purposes.\
             It contains multiple paragraphs, headings, and tables to simulate\
             real-world document structure.",
        ))
        .add_heading("Data Table", HeadingLevel::H2)
        .add_paragraph(Paragraph::new().add_text("Below is a table with 50 rows of sample data."))
        .add_table(Table::from_data(&small_rows))
        .add_heading("Conclusion", HeadingLevel::H2)
        .add_paragraph(Paragraph::new().add_text("End of benchmark document."))
        .save()
        .expect("failed to create small fixture");

    // Medium document: 1 000 rows
    let medium_path = dir.path().join("medium.docx");
    let medium_rows = generate_rows(1_000);
    EasyDoc::write_table(&medium_path, &medium_rows)
        .title("Medium Benchmark Table")
        .do_write()
        .expect("failed to create medium fixture");

    // Large document: 5 000 rows
    let large_path = dir.path().join("large.docx");
    let large_rows = generate_rows(5_000);
    EasyDoc::write_table(&large_path, &large_rows)
        .title("Large Benchmark Table")
        .do_write()
        .expect("failed to create large fixture");

    Fixture {
        _dir: dir,
        small_docx: small_path,
        medium_docx: medium_path,
        large_docx: large_path,
    }
});

// ---------------------------------------------------------------------------
// A. Write throughput: Vec<BenchRow> -> DOCX bytes
// ---------------------------------------------------------------------------

fn bench_write_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_throughput");
    group.sample_size(50);

    for &n in &[100, 500, 1_000] {
        let rows = generate_rows(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &rows, |b, rows| {
            b.iter(|| {
                // Write to bytes — isolates serialization + ZIP from disk I/O
                black_box(EasyDoc::write_table_to_bytes(rows).expect("write failed"));
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// B. Read text throughput
// ---------------------------------------------------------------------------

fn bench_read_text(c: &mut Criterion) {
    let fx = &*FIXTURE;
    let mut group = c.benchmark_group("read_text_throughput");
    group.sample_size(50);

    group.bench_function("small_docx", |b| {
        b.iter(|| {
            black_box(EasyDoc::read_text(&fx.small_docx).expect("read failed"));
        });
    });

    group.bench_function("medium_docx", |b| {
        b.iter(|| {
            black_box(EasyDoc::read_text(&fx.medium_docx).expect("read failed"));
        });
    });

    group.bench_function("large_docx", |b| {
        b.iter(|| {
            black_box(EasyDoc::read_text(&fx.large_docx).expect("read failed"));
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// C. ViewMode rendering throughput — compare 4 modes on the same document
// ---------------------------------------------------------------------------

fn bench_view_mode(c: &mut Criterion) {
    let fx = &*FIXTURE;
    let mut group = c.benchmark_group("view_mode_throughput");
    group.sample_size(50); // large doc is slow; reduce sample count

    let modes: &[(&str, ViewMode)] = &[
        ("Plain", ViewMode::Plain),
        ("Annotated", ViewMode::Annotated),
        ("Outline_3", ViewMode::Outline { max_level: 3 }),
        ("Stats", ViewMode::Stats),
    ];

    for (label, mode) in modes {
        group.bench_with_input(BenchmarkId::new("small", label), mode, |b, mode| {
            b.iter(|| {
                black_box(EasyDoc::view_as(&fx.small_docx, mode).expect("view_as failed"));
            });
        });
    }

    for (label, mode) in modes {
        group.bench_with_input(BenchmarkId::new("medium", label), mode, |b, mode| {
            b.iter(|| {
                black_box(EasyDoc::view_as(&fx.medium_docx, mode).expect("view_as failed"));
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// D. Streaming (SAX events) vs one-shot (full DocumentContent load)
// ---------------------------------------------------------------------------

/// A no-op EventSink that counts events without allocating.
struct CountingSink {
    count: usize,
}

impl CountingSink {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl EventSink for CountingSink {
    fn on_event(&mut self, _event: &DocumentEvent) -> easydoc_core::Result<()> {
        self.count += 1;
        Ok(())
    }
}

fn bench_stream_vs_oneshot(c: &mut Criterion) {
    let fx = &*FIXTURE;
    let mut group = c.benchmark_group("stream_vs_oneshot");
    group.sample_size(50);

    // One-shot: load full DocumentContent
    group.bench_function("oneshot_medium", |b| {
        b.iter(|| {
            black_box(EasyDoc::load(&fx.medium_docx).expect("load failed"));
        });
    });

    // Streaming: SAX events via read_events
    group.bench_function("stream_medium", |b| {
        b.iter(|| {
            let mut sink = CountingSink::new();
            EasyDoc::read_events(&fx.medium_docx, &mut sink).expect("read_events failed");
            black_box(sink.count);
        });
    });

    // Large document
    group.bench_function("oneshot_large", |b| {
        b.iter(|| {
            black_box(EasyDoc::load(&fx.large_docx).expect("load failed"));
        });
    });

    group.bench_function("stream_large", |b| {
        b.iter(|| {
            let mut sink = CountingSink::new();
            EasyDoc::read_events(&fx.large_docx, &mut sink).expect("read_events failed");
            black_box(sink.count);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// E. Markdown conversion throughput
// ---------------------------------------------------------------------------

fn bench_markdown_throughput(c: &mut Criterion) {
    let fx = &*FIXTURE;
    let mut group = c.benchmark_group("markdown_throughput");
    group.sample_size(50);

    group.bench_function("small_docx", |b| {
        b.iter(|| {
            black_box(EasyDoc::to_markdown(&fx.small_docx).expect("to_markdown failed"));
        });
    });

    group.bench_function("medium_docx", |b| {
        b.iter(|| {
            black_box(EasyDoc::to_markdown(&fx.medium_docx).expect("to_markdown failed"));
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion harness
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_write_throughput,
    bench_read_text,
    bench_view_mode,
    bench_stream_vs_oneshot,
    bench_markdown_throughput,
);
criterion_main!(benches);
