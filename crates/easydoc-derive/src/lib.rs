//! Derive macros for typed DOCX table row mapping.
//!
//! Provides `#[derive(DocxRow)]` which generates:
//! - `schema()` — static column metadata
//! - `from_row()` / `from_row_with_converters()` — row deserialisation
//! - `to_row()` / `to_row_with_converters()` — row serialisation

use proc_macro::TokenStream;

mod implementation;

/// Derives static table column metadata and bidirectional row conversion.
///
/// # Struct attributes
///
/// - `#[docx(table_width = Auto)]` — auto-fit table width
/// - `#[docx(banded_rows = true)]` — enable zebra striping
///
/// # Field attributes
///
/// - `#[docx(name = "Display Name")]` — column header text
/// - `#[docx(index = 0)]` — zero-based column index
/// - `#[docx(order = 0)]` — column sort order (lower = leftmost)
/// - `#[docx(width = 0.3)]` — column width as fraction (0.0–1.0)
/// - `#[docx(format = "%Y-%m-%d")]` — date/time format pattern
/// - `#[docx(ignore)]` — skip this field during read/write
///
/// # Example
///
/// ```ignore
/// use easydoc_derive::DocxRow;
///
/// #[derive(DocxRow)]
/// #[docx(banded_rows = true)]
/// struct User {
///     #[docx(name = "姓名", width = 0.3, order = 0)]
///     name: String,
///
///     #[docx(name = "年龄", width = 0.15, order = 1)]
///     age: u32,
///
///     #[docx(name = "邮箱", width = 0.55, order = 2)]
///     email: String,
///
///     #[docx(ignore)]
///     internal_id: String,
/// }
/// ```
#[proc_macro_derive(DocxRow, attributes(docx))]
pub fn derive_docx_row(input: TokenStream) -> TokenStream {
    implementation::expand_docx_row_tokens(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
