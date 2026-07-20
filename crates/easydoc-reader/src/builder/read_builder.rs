//! Streaming document reader builder.

use std::path::PathBuf;

use easydoc_core::{DocxRow, Result};
use crate::extractor;

/// Fluent builder for streaming document reads.
///
/// Created via [`EasyDoc::read()`].
pub struct DocReadBuilder {
    path: PathBuf,
}

impl DocReadBuilder {
    /// Creates a new reader builder.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
        }
    }

    /// Executes a sync read returning all tables flattened into a single `Vec<T>`.
    ///
    /// Uses office_oxide for backend parsing.
    ///
    /// # Errors
    ///
    /// Returns I/O, format, or conversion errors.
    pub fn do_read<T: DocxRow>(self) -> Result<Vec<T>> {
        let tables: Vec<Vec<T>> = extractor::table::extract_tables::<T>(&self.path)?;
        Ok(tables.into_iter().flatten().collect())
    }
}
