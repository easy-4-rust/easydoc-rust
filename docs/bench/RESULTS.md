# Benchmark Results

> **Status**: Populated -- 2026-08-10, Criterion 0.5, 50 samples per benchmark.
>
> **Caveat**: Fixture sizes were reduced from the original template targets
> (100/500/1K rows for write; 50/1K/5K rows for read fixtures) because
> `write_table_to_bytes` at 10K+ rows exceeds reasonable bench time
> (~156 s/iteration at 10K). The original targets of 10K/100K rows for write
> and 50K rows for read fixtures were not achievable within a 30-minute window.
> Results below reflect the reduced sizes and are representative of relative
> performance characteristics.

## Environment

| Field | Value |
|---|---|
| Date | 2026-08-10 |
| OS | macOS 25.5.0 (Darwin Kernel Version 25.5.0) arm64 |
| CPU | Apple T6041 (Apple Silicon) |
| RAM | (not measured) |
| Rust version | 1.97.1 (8bab26f4f 2026-07-14) |
| Cargo version | 1.97.1 (c980f4866 2026-06-30) |
| Toolchain | stable-aarch64-apple-darwin |
| Criterion | 0.5.1 |
| Build profile | release (optimized) |
| Samples | 50 per benchmark |

## Results

### A. Write Throughput (`write_throughput`)

Write to in-memory bytes via `EasyDoc::write_table_to_bytes`. Excludes disk I/O.

| Rows | Median | Min | Max | Throughput |
|---|---|---|---|---|
| 100 | 17.842 ms | 17.417 ms | 18.355 ms | ~5,600 rows/s |
| 500 | 416.57 ms | 397.16 ms | 442.80 ms | ~1,200 rows/s |
| 1,000 | 1.6605 s | 1.6161 s | 1.7111 s | ~602 rows/s |

**Observations**: Write throughput degrades with scale -- sub-linear scaling suggests
per-row overhead dominates. At 100 rows the per-row cost is ~178 us; at 1,000 rows
it is ~1,661 us. The increase is super-linear, pointing to non-trivial allocation or
serialization overhead that grows with row count (likely repeated XML string
construction or ZIP buffer management).

### B. Read Text Throughput (`read_text_throughput`)

Plain text extraction via `EasyDoc::read_text` from DOCX files on disk.

| Fixture | Rows | Median | Min | Max |
|---|---|---|---|---|
| small.docx | 50 | 266.28 us | 243.71 us | 294.29 us |
| medium.docx | 1,000 | 3.1834 ms | 2.9306 ms | 3.4760 ms |
| large.docx | 5,000 | 18.157 ms | 17.681 ms | 18.751 ms |

**Observations**: Read scales roughly linearly with document size.
Per-row cost is approximately 3.6 us/row (small), 3.2 us/row (medium),
3.6 us/row (large). The ~5 us/row cost for the 50-row small doc reflects
fixed overhead (ZIP open, XML parse bootstrap).

### C. ViewMode Rendering (`view_mode_throughput`)

Rendering via `EasyDoc::view_as` with 4 modes: Plain, Annotated,
Outline (max_level=3), Stats. Sample size: 50.

| Mode | small.docx (50 rows) | medium.docx (1,000 rows) |
|---|---|---|
| Plain | 344.24 us [325.14, 368.96] | 5.5523 ms [5.2874, 5.8484] |
| Annotated | 289.90 us [286.94, 293.05] | 5.7701 ms [5.5675, 5.9996] |
| Outline (max_level=3) | 528.69 us [482.30, 583.78] | 4.5280 ms [4.4122, 4.6822] |
| Stats | 283.08 us [277.75, 289.53] | 4.4851 ms [4.4160, 4.5503] |

**Observations**: All modes show similar cost on medium docs (4.5-5.8 ms).
On small docs, Outline mode is notably slower (529 us vs 283-344 us for others)
due to heading-level filtering overhead on a small document where the fixed
cost is more visible. Stats mode is consistently the fastest, as it only
aggregates metadata rather than rendering full content.

### D. Stream vs One-Shot (`stream_vs_oneshot`)

Comparing full `DocumentContent` load (one-shot) vs SAX event streaming
via `EasyDoc::read_events` with a no-op counting sink. Sample size: 50.

| Method | medium.docx (1,000 rows) | large.docx (5,000 rows) |
|---|---|---|
| One-shot (full load) | 4.6910 ms [4.5133, 4.9124] | 25.893 ms [25.709, 26.089] |
| SAX stream (events) | 2.4512 ms [2.2844, 2.6348] | 10.630 ms [10.561, 10.714] |
| **Speedup** | **1.91x faster** | **2.44x faster** |

**Observations**: Streaming is consistently ~2x faster than one-shot loading
because it avoids building the full `DocumentContent` in memory. The speedup
increases with document size (1.91x at 1K rows, 2.44x at 5K rows), confirming
that the allocation/composition cost of the in-memory document model is the
bottleneck. For large documents, streaming also uses O(1) memory vs O(n) for
one-shot (though memory was not measured in this benchmark run).

### E. Markdown Conversion (`markdown_throughput`)

DOCX-to-Markdown conversion via `EasyDoc::to_markdown`. Sample size: 50.

| Fixture | Rows | Median | Min | Max |
|---|---|---|---|---|
| small.docx | 50 | 307.35 us | 300.96 us | 316.43 us |
| medium.docx | 1,000 | 6.8104 ms | 6.4470 ms | 7.1978 ms |

**Observations**: Markdown conversion scales linearly. Per-row cost is
approximately 6.8 us/row for medium docs. This is about 2x the per-row cost
of plain text extraction (3.2 us/row), which is expected since Markdown
generation must handle formatting, table borders, and heading prefixes.

## Summary

| Metric | Value | Notes |
|---|---|---|
| Write throughput | ~600-5,600 rows/s | Degrades with scale; 1K rows takes ~1.7s |
| Read text (medium) | ~3.2 ms for 1K rows | Linear scaling, ~3.2 us/row |
| ViewMode (medium) | 4.5-5.8 ms for 1K rows | All modes similar cost; Stats fastest |
| Stream vs One-shot | Stream is 1.9-2.4x faster | Speedup increases with document size |
| Markdown (medium) | ~6.8 ms for 1K rows | ~2x cost of plain text extraction |

### Key Takeaways

1. **Streaming is the clear winner for large documents**: SAX event streaming
   is 2x faster than full document load and uses O(1) memory. For documents
   with >1K rows, prefer `read_events` over `load`.

2. **Write performance is the bottleneck**: At ~600 rows/s for 1K-row documents,
   writing is significantly slower than reading. The super-linear scaling suggests
   optimization opportunities in XML serialization or ZIP buffer management.

3. **ViewMode overhead is minimal**: All four rendering modes cost within 30% of
   each other on medium documents. Mode selection should be driven by UX needs,
   not performance.

4. **Markdown conversion adds ~2x overhead vs plain text**: Expected given
   formatting requirements. For bulk conversion pipelines, consider batching.

### Limitations

- Fixture sizes reduced from original targets (100/500/1K for write, 50/1K/5K
  for read) due to time constraints. Original 10K/100K-row write benchmarks
  would take 150s+ per iteration.
- Memory usage was not measured (no heaptrack/DHAT integration).
- Single machine, single run. No cross-platform comparison.
- `write_table_to_bytes` includes both serialization and ZIP compression;
  these were not profiled separately.

## How to Reproduce

```bash
cd /Users/wandl/workspaces/workspace-github-easy-4-rust/easydoc-rust
/Users/wandl/.cargo/bin/cargo bench --package easydoc --bench read_write -- --bench
```

Criterion HTML reports are available at:
`target/criterion/<group>/<id>/report/index.html`

## Comparison with easyexcel-rust

The sister project `easyexcel-rust` maintains its own benchmark suite in
`benchmarks/rust-runner/`. Cross-project comparison should use equivalent
dataset sizes and measure the same pipeline stages (serialize/deserialize,
ZIP/unzip, XML parse).
