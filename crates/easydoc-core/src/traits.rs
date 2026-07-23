//! Core extension traits for `easydoc-rust`.
//!
//! These four traits form the backbone of the extensibility system,
//! mirroring `ExcelRow`, `Converter<T>`, `ReadListener<T>`, and
//! `WriteHandler` from `easyexcel-rust`.

use crate::converter::ConverterRegistry;
use crate::error::Result;
use crate::metadata::TableColumn;
use crate::types::{CellData, DocValue, ErrorAction, RowData, TableData};

// ============================================================================
// DocxRow — struct ↔ table row mapping
// ============================================================================

/// Maps a Rust struct to/from a DOCX table row.
///
/// Analogous to [`ExcelRow`](easyexcel_core::ExcelRow) in `easyexcel-rust`.
/// Generated automatically by `#[derive(DocxRow)]`.
///
/// # Example
///
/// ```ignore
/// #[derive(DocxRow)]
/// struct User {
///     #[docx(name = "Name", order = 0)]
///     name: String,
///     #[docx(name = "Age", order = 1)]
///     age: u32,
/// }
/// ```
pub trait DocxRow {
    /// Returns the column schema: header names, indices, and format hints.
    fn schema() -> &'static [TableColumn]
    where
        Self: Sized;

    /// Deserialises a row from raw cell values using default converters.
    fn from_row(row: &RowData) -> Result<Self>
    where
        Self: Sized;

    /// Deserialises a row using a custom converter registry.
    fn from_row_with_converters(row: &RowData, registry: &ConverterRegistry) -> Result<Self>
    where
        Self: Sized;

    /// Serialises self into a row of cell values using default converters.
    fn to_row(&self) -> Result<Vec<CellData>>;

    /// Serialises self into a row of cell values using a custom converter registry.
    fn to_row_with_converters(&self, registry: &ConverterRegistry) -> Result<Vec<CellData>>;
}

// ============================================================================
// DocConverter — bidirectional type conversion
// ============================================================================

/// Converts between a Rust type `T` and a [`DocValue`].
///
/// Analogous to `Converter<T>` in `easyexcel-rust`. Register custom converters
/// via [`ConverterRegistry`] or the builder's `register_converter` method.
pub trait DocConverter<T> {
    /// Returns the `TypeId` this converter handles.
    fn support_type() -> std::any::TypeId
    where
        Self: Sized;

    /// Converts a Rust value into a document value for writing.
    ///
    /// # Errors
    ///
    /// Returns [`DocError::Conversion`] if the value cannot be converted.
    fn to_doc_value(&self, value: &T, column: &TableColumn) -> Result<DocValue>;

    /// Converts a document value back into a Rust value for reading.
    ///
    /// # Errors
    ///
    /// Returns [`DocError::Conversion`] if the value cannot be converted.
    fn from_doc_value(&self, value: &DocValue, column: &TableColumn) -> Result<T>;
}

// ============================================================================
// DocReadListener — streaming read callbacks
// ============================================================================

/// Context passed to read listener callbacks.
#[derive(Debug, Clone)]
pub struct DocReadContext {
    /// Current document path.
    pub path: String,
    /// Current paragraph or table index (0-based).
    pub index: usize,
}

/// Receives parsed content during streaming document reads.
///
/// Analogous to `ReadListener<T>` in `easyexcel-rust`.
pub trait DocReadListener<T> {
    /// Called for each parsed data item (paragraph text or row).
    ///
    /// # Errors
    ///
    /// Return an error to stop reading; use `ErrorAction` via `on_error`
    /// for recoverable errors.
    fn invoke(&mut self, data: T, context: &DocReadContext) -> Result<()>;

    /// Called when a complete table is encountered.
    fn invoke_table(&mut self, table: &TableData, context: &DocReadContext) -> Result<()> {
        let _ = (table, context);
        Ok(())
    }

    /// Called after all content has been parsed.
    fn on_complete(&mut self, _context: &DocReadContext) {}

    /// Called when a non-fatal error occurs during reading.
    ///
    /// Return [`ErrorAction::Stop`] to propagate the error, or
    /// [`ErrorAction::Skip`] / [`ErrorAction::Continue`] to proceed.
    fn on_error(
        &mut self,
        _error: &crate::error::DocError,
        _context: &DocReadContext,
    ) -> ErrorAction {
        ErrorAction::Stop
    }

    /// Called before processing each item to check whether reading should continue.
    /// Return `false` to stop early.
    fn has_next(&self, _context: &DocReadContext) -> bool {
        true
    }
}

// ============================================================================
// DocWriteHandler — write lifecycle hooks
// ============================================================================

/// Context for document-level write events.
#[derive(Debug, Clone)]
pub struct DocWriteContext {
    /// Output path.
    pub path: String,
}

/// Context for paragraph-level write events.
#[derive(Debug, Clone)]
pub struct ParagraphContext {
    /// Paragraph index (0-based).
    pub index: usize,
}

/// Context for table-level write events.
#[derive(Debug, Clone)]
pub struct TableWriteContext {
    /// Table index (0-based).
    pub index: usize,
    /// Number of rows in the table.
    pub row_count: usize,
}

/// Context for cell-level write events.
#[derive(Debug, Clone)]
pub struct CellContext {
    /// Row index (0-based).
    pub row: usize,
    /// Column index (0-based).
    pub column: usize,
    /// Cell value.
    pub value: DocValue,
}

/// Write lifecycle interceptor — hooks at document, paragraph, table, and cell level.
///
/// Analogous to `WriteHandler` in `easyexcel-rust`. All methods have no-op defaults;
/// override only the hooks you need.
pub trait DocWriteHandler {
    /// Execution order (lower values execute first).
    #[must_use]
    fn order() -> i32 {
        0
    }

    /// Called before the document is created.
    fn before_document(&mut self, _ctx: &DocWriteContext) -> Result<()> {
        Ok(())
    }

    /// Called after the document is finalised.
    fn after_document(&mut self, _ctx: &DocWriteContext) -> Result<()> {
        Ok(())
    }

    /// Called before a paragraph is written.
    fn before_paragraph(&mut self, _ctx: &ParagraphContext) -> Result<()> {
        Ok(())
    }

    /// Called after a paragraph is written.
    fn after_paragraph(&mut self, _ctx: &ParagraphContext) -> Result<()> {
        Ok(())
    }

    /// Called before a table is written.
    fn before_table(&mut self, _ctx: &TableWriteContext) -> Result<()> {
        Ok(())
    }

    /// Called after a table is written.
    fn after_table(&mut self, _ctx: &TableWriteContext) -> Result<()> {
        Ok(())
    }

    /// Called before a cell is written.
    fn before_cell(&mut self, _ctx: &CellContext) -> Result<()> {
        Ok(())
    }

    /// Called after a cell is written.
    fn after_cell(&mut self, _ctx: &CellContext) -> Result<()> {
        Ok(())
    }
}
