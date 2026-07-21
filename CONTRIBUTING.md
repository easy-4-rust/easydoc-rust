# Contributing to easydoc-rs

## Development Setup

```bash
git clone https://github.com/hiwepy/easydoc-rs
cd easydoc-rs
cargo build
```

## Quality Gates

Before submitting a PR:

```bash
# Format
cargo fmt --all -- --check

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Build
cargo check --workspace

# Test
cargo test --workspace

# Docs
cargo doc --workspace --no-deps
```

## Project Structure

```
easydoc-rs/
├── crates/
│   ├── easydoc/          facade — public API
│   ├── easydoc-core/     shared types and traits
│   ├── easydoc-derive/   proc-macro
│   ├── easydoc-writer/   DOCX generation
│   ├── easydoc-reader/   DOCX/DOC reading
│   └── easydoc-template/ placeholder replacement
└── docs/
    ├── architecture.md   architecture design
    └── usage-guide.md    user guide
```

## Design Principles

1. **Zero unsafe** — `#![forbid(unsafe_code)]` in every crate
2. **Fluent builders** — `mut self -> Self` with `#[must_use]`
3. **Trait extensibility** — `DocxRow`, `DocConverter`, `DocWriteHandler`, `DocReadListener`
4. **Single error type** — `DocError` enum, `type Result<T> = ...`
5. **Follow easyexcel-rs conventions** — consistency across the ecosystem

## Adding a New Feature

1. Define core types in `easydoc-core` (if needed)
2. Implement engine logic in the appropriate crate (writer/reader/template)
3. Expose via `easydoc` facade
4. Add tests in `crates/easydoc/tests/`
5. Update documentation in `docs/`

## Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `test:` tests
- `refactor:` code change without feature/fix
- `chore:` build, CI, dependencies
