# Changelog

## [0.1.0] — 2026-07-21

### Added
- 6-crate workspace: `easydoc`, `easydoc-core`, `easydoc-derive`, `easydoc-writer`, `easydoc-reader`, `easydoc-template`
- Static factory `EasyDoc` with fluent builder API
- `#[derive(DocxRow)]` proc-macro for compile-time struct-to-table mapping
- DOCX writing: paragraphs, headings (H1-H6), tables, page breaks, styled runs (bold, italic, underline, color, size, font)
- Quick table write: `EasyDoc::write_table(path, &[T])` — one-liner
- DOCX/DOC reading: text extraction, table extraction via `office_oxide`
- Format auto-detection: DOCX (ZIP magic) vs DOC (OLE2 magic)
- Template fill: `{key}` scalar placeholder replacement with ZIP structure preservation
- Template fill: `{.field}` collection expansion in table rows and paragraphs
- Style system: `FontConfig`, `ParagraphStyle`, `TableStyle`, `Color`
- Document metadata: `DocumentMeta` (title, author, subject, page size)
- Extensible converters: `DocConverter<T>` with `ConverterRegistry`
- Built-in fallback converters: String, i32, i64, u32, f64, bool, DateTime<Utc>, NaiveDate, NaiveDateTime
- Write lifecycle hooks: `DocWriteHandler` at document/paragraph/table/cell level
- Stream read listener: `DocReadListener<T>` with `CollectListener`
- Error handling: 7 variant `DocError` enum with `thiserror`
- Image insertion via `DocImage` (PNG/JPEG)
- 15 integration tests + 4 unit tests
- Bilingual README (EN/ZH), architecture design document, usage guide
