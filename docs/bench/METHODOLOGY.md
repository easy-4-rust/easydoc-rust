# Benchmark Methodology

## Measurement Framework

All benchmarks use [Criterion.rs v0.5](https://bheisler.github.io/criterion.rs/book/)
with HTML report generation enabled. Criterion provides:

- Statistical analysis with warm-up, outlier detection, and confidence intervals
- Automatic comparison against previous runs (regression detection)
- Throughput calculations (elements/second or bytes/second)
- HTML reports with plots in `target/criterion/<group>/<id>/report/index.html`

## Dataset Specifications

### Synthetic Data Generation

All fixtures are generated from deterministic synthetic data. No real-world
documents are used, ensuring full reproducibility across runs and machines.

```rust
struct BenchRow {
    id: u64,        // sequential 0..N
    name: String,   // "item_{i:06}"
    amount: f64,    // i * 1.23
    active: bool,   // i % 3 != 0
}
```

### Fixture Sizes

| Fixture | Table Rows | Paragraphs | Headings | Approximate DOCX Size |
|---|---|---|---|---|
| `small.docx` | 50 | 3 | 3 (H1, H2, H2) | ~15 KB |
| `medium.docx` | 5,000 | 0 | 1 (title) | ~300 KB |
| `large.docx` | 50,000 | 0 | 1 (title) | ~3 MB |

Fixtures are created in a `tempfile::tempdir()` at first access via `LazyLock`
and automatically cleaned up when the benchmark process exits.

## Benchmark Design Principles

### Isolation of Concerns

- **Write benchmarks (Group A)** measure serialization + ZIP only by writing
  to in-memory bytes (`write_table_to_bytes`), eliminating disk I/O variance.
- **Read benchmarks (Groups B, C, D, E)** read from disk to measure the full
  pipeline including decompression and XML parsing.

### Sample Sizes

- Default Criterion sample size (100) for small/medium benchmarks
- Reduced to 50 samples for large document benchmarks to keep runtime
  manageable while maintaining statistical significance

### Black Box

All benchmark iterations wrap results in `std::hint::black_box()` to prevent
the compiler from optimizing away the measured work.

## Environment Requirements

| Requirement | Minimum |
|---|---|
| Rust toolchain | 1.88+ (edition 2024) |
| Disk space | ~500 MB for target/criterion output |
| Memory | ~2 GB (large fixture generation + Criterion buffers) |

## Reproducing Results

```bash
# 1. Ensure clean state
cargo clean

# 2. Run full suite
cd crates/easydoc
cargo bench 2>&1 | tee /tmp/bench-$(date +%Y%m%d).txt

# 3. Save as baseline for future comparison
cargo bench -- --save-baseline initial

# 4. After changes, compare
cargo bench -- --baseline initial
```

## Limitations

- Benchmarks run in `dev` profile (unoptimized). For production performance
  estimates, use `cargo bench --profile release` or measure separately.
- Criterion's statistical model assumes i.i.d. samples. System load, thermal
  throttling, and background processes can affect results.
- Large fixture generation (50K rows) takes several seconds on first run;
  subsequent iterations reuse the cached fixture.
