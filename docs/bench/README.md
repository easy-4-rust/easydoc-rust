# easydoc-rust Performance Benchmarks

This directory documents the performance benchmark suite for `easydoc-rust`.
The benchmarks are located in `crates/easydoc/benches/read_write.rs` and use
the [Criterion](https://bheisler.github.io/criterion.rs/book/) framework.

## Quick Start

```bash
# Run all benchmarks (full statistical analysis)
cd crates/easydoc
cargo bench

# Quick sanity check (fewer iterations, ~30s)
cargo bench -- --quick

# Run a specific benchmark group
cargo bench -- "write_throughput"
cargo bench -- "read_text_throughput"
cargo bench -- "view_mode_throughput"
cargo bench -- "stream_vs_oneshot"
cargo bench -- "markdown_throughput"

# Compare against a saved baseline
cargo bench -- --save-baseline my-baseline
cargo bench -- --baseline my-baseline
```

Results are written to `target/criterion/` with HTML reports viewable in a browser.

## Benchmark Groups

| Group | ID | Description |
|---|---|---|
| A | `write_throughput` | Table write at 1K / 10K / 100K rows, measured as bytes-only (no disk I/O) |
| B | `read_text_throughput` | Plain text extraction from small / medium / large DOCX fixtures |
| C | `view_mode_throughput` | All 4 ViewMode variants (Plain, Annotated, Outline, Stats) on small and medium docs |
| D | `stream_vs_oneshot` | SAX event streaming vs full `DocumentContent` load on medium and large docs |
| E | `markdown_throughput` | DOCX-to-Markdown conversion on small and medium docs |

## Fixture Sizes

| Fixture | Rows | Content |
|---|---|---|
| `small.docx` | 50 | Headings + paragraphs + table |
| `medium.docx` | 5,000 | Table-only |
| `large.docx` | 50,000 | Table-only |

Fixtures are generated at benchmark startup via `tempfile::tempdir()` and
cleaned up automatically. No real documents are used -- all data is synthetic
to guarantee reproducibility.

## Related Documentation

- [METHODOLOGY.md](METHODOLOGY.md) -- measurement methodology and dataset specifications
- [RESULTS.md](RESULTS.md) -- recorded benchmark results (template; run benchmarks to populate)
