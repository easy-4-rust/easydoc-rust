//! `#[derive(DocxRow)]` 过程宏的实现。
//!
//! 生成 `impl DocxRow for YourStruct` 代码块，包含：
//! - `schema()` -- 返回 `&'static [TableColumn]`
//! - `from_row()` / `from_row_with_converters()` -- 行 -> 结构体
//! - `to_row()` / `to_row_with_converters()` -- 结构体 -> 行

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, MetaNameValue, Type, punctuated::Punctuated, token::Comma};

/// Parsed per-field attribute configuration.
struct FieldConfig {
    ident: syn::Ident,
    name: String,
    index: usize,
    order: u32,
    width: Option<String>,
    format: Option<String>,
    align: Option<String>,
    converter: Option<String>,
    wrap: bool,
    ignored: bool,
    ty: Type,
}

/// Parsed struct-level attribute configuration.
struct StructConfig {
    banded_rows: bool,
    auto_width: bool,
}

pub(crate) fn expand_docx_row_tokens(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();

    // Parse struct-level attributes
    let struct_config = parse_struct_attrs(&input.attrs);

    // Collect fields
    let fields = match &input.data {
        syn::Data::Struct(data) => collect_fields(&data.fields)?,
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "DocxRow can only be derived for structs",
            ));
        }
    };

    // Generate schema array
    let schema_entries = fields.iter().filter(|f| !f.ignored).map(|f| {
        let name = &f.name;
        let field_name = f.ident.to_string();
        let index = f.index;
        let order = f.order;

        let width = f
            .width
            .as_ref()
            .map_or_else(|| quote! { None }, |w| quote! { Some(#w.to_owned()) });
        let format = f
            .format
            .as_ref()
            .map_or_else(|| quote! { None }, |fmt| quote! { Some(#fmt.to_owned()) });
        let align = f.align.as_ref().map_or_else(
            || quote! { None },
            |a| {
                let variant = align_variant_token(a);
                quote! { Some(easydoc_core::HorizontalAlignment::#variant) }
            },
        );
        let converter = f
            .converter
            .as_ref()
            .map_or_else(|| quote! { None }, |c| quote! { Some(#c.to_owned()) });
        let wrap = f.wrap;

        quote! {
            easydoc_core::TableColumn {
                name: #name.to_owned(),
                field_name: #field_name.to_owned(),
                index: #index,
                order: #order,
                width: #width,
                format: #format,
                align: #align,
                converter: #converter,
                wrap: #wrap,
                ignored: false,
            }
        }
    });

    // Generate to_row body — attach alignment from schema when available
    let to_row_cells = fields.iter().filter(|f| !f.ignored).map(|f| {
        let ident = &f.ident;
        let align_expr = f.align.as_ref().map_or_else(
            || quote! { None },
            |a| {
                let variant = align_variant_token(a);
                quote! { Some(easydoc_core::HorizontalAlignment::#variant) }
            },
        );

        quote! {
            {
                let mut cell = easydoc_core::CellData::new(self.#ident.clone());
                cell.alignment = #align_expr;
                cell
            }
        }
    });

    let field_count = fields.iter().filter(|f| !f.ignored).count();

    // Generate from_row body
    let from_row_bindings = fields
        .iter()
        .filter(|f| !f.ignored)
        .enumerate()
        .map(|(i, f)| {
            let ident = &f.ident;
            let idx = syn::Index::from(i);
            let ty = &f.ty;
            let field_name = f.ident.to_string();

            quote! {
                let #ident: #ty = {
                    let cell = &row.cells[#idx];
                    match &cell.value {
                        easydoc_core::DocValue::String(s) => {
                            s.parse().map_err(|_| easydoc_core::DocError::Conversion {
                                field: #field_name.to_owned(),
                                value: s.clone(),
                                message: "parse error".to_owned(),
                            })?
                        }
                        easydoc_core::DocValue::Int(n) => {
                            let n = *n;
                            // Use a type-erased conversion; for now just format as string
                            // and re-parse. The derive is a best-effort starting point.
                            let s = n.to_string();
                            s.parse().map_err(|_| easydoc_core::DocError::Conversion {
                                field: #field_name.to_owned(),
                                value: s.clone(),
                                message: "parse error".to_owned(),
                            })?
                        }
                        easydoc_core::DocValue::Float(n) => {
                            let s = n.to_string();
                            s.parse().map_err(|_| easydoc_core::DocError::Conversion {
                                field: #field_name.to_owned(),
                                value: s.clone(),
                                message: "parse error".to_owned(),
                            })?
                        }
                        easydoc_core::DocValue::Bool(b) => {
                            let s = b.to_string();
                            s.parse().map_err(|_| easydoc_core::DocError::Conversion {
                                field: #field_name.to_owned(),
                                value: s.clone(),
                                message: "parse error".to_owned(),
                            })?
                        }
                        easydoc_core::DocValue::Empty => {
                            return Err(easydoc_core::DocError::Conversion {
                                field: #field_name.to_owned(),
                                value: "<empty>".to_owned(),
                                message: "required field is empty".to_owned(),
                            })
                        }
                        _ => {
                            return Err(easydoc_core::DocError::Conversion {
                                field: #field_name.to_owned(),
                                value: format!("{:?}", cell.value),
                                message: "unsupported value type".to_owned(),
                            })
                        }
                    }
                };
            }
        });

    let from_row_self: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            if f.ignored {
                quote! { #ident: Default::default() }
            } else {
                quote! { #ident }
            }
        })
        .collect();

    // Generate converter-aware from_row bindings.
    // For each non-ignored field, generate code that uses registry.from_doc_value
    // which dispatches to registered converters or built-in fallback.
    let from_row_converter_bindings =
        fields
            .iter()
            .filter(|f| !f.ignored)
            .enumerate()
            .map(|(i, f)| {
                let ident = &f.ident;
                let idx = syn::Index::from(i);
                let ty = &f.ty;
                let field_name = f.ident.to_string();

                quote! {
                    let #ident: #ty = {
                        let __cell = &row.cells[#idx];
                        registry.from_doc_value::<#ty>(&__cell.value, &__schema[#idx])
                            .map_err(|__err| easydoc_core::DocError::Conversion {
                                field: #field_name.to_owned(),
                                value: format!("{:?}", __cell.value),
                                message: format!("converter error: {}", __err),
                            })?
                    };
                }
            });

    // Generate converter-aware to_row body.
    // For each non-ignored field, use registry.to_doc_value which dispatches
    // to registered converters or built-in fallback, then wrap in CellData.
    let to_row_converter_cells = fields
        .iter()
        .filter(|f| !f.ignored)
        .enumerate()
        .map(|(i, f)| {
            let ident = &f.ident;
            let idx = syn::Index::from(i);
            let align_expr = f.align.as_ref().map_or_else(
                || quote! { None },
                |a| {
                    let variant = align_variant_token(a);
                    quote! { Some(easydoc_core::HorizontalAlignment::#variant) }
                },
            );

            quote! {
                {
                    let __doc_val = registry.to_doc_value(&self.#ident, &__schema[#idx])?;
                    easydoc_core::CellData {
                        value: __doc_val,
                        alignment: #align_expr,
                        col_span: 1,
                        row_span: 1,
                    }
                }
            }
        });

    let _banded_rows = struct_config.banded_rows;
    let _auto_width = struct_config.auto_width;

    let expanded = quote! {
        impl easydoc_core::DocxRow for #struct_name {
            fn schema() -> &'static [easydoc_core::TableColumn] {
                static SCHEMA: std::sync::LazyLock<Vec<easydoc_core::TableColumn>> =
                    std::sync::LazyLock::new(|| {
                        vec![
                            #(#schema_entries,)*
                        ]
                    });
                &*SCHEMA
            }

            fn from_row(row: &easydoc_core::RowData) -> easydoc_core::Result<Self> {
                if row.cells.len() < #field_count {
                    return Err(easydoc_core::DocError::Conversion {
                        field: #struct_name_str.to_owned(),
                        value: format!("{} cells, expected {}", row.cells.len(), #field_count),
                        message: "not enough cells in row".to_owned(),
                    });
                }
                #(#from_row_bindings)*
                Ok(Self {
                    #(#from_row_self,)*
                })
            }

            fn from_row_with_converters(
                row: &easydoc_core::RowData,
                registry: &easydoc_core::ConverterRegistry,
            ) -> easydoc_core::Result<Self> {
                let __schema = Self::schema();
                if row.cells.len() < #field_count {
                    return Err(easydoc_core::DocError::Conversion {
                        field: #struct_name_str.to_owned(),
                        value: format!("{} cells, expected {}", row.cells.len(), #field_count),
                        message: "not enough cells in row".to_owned(),
                    });
                }
                #(#from_row_converter_bindings)*
                Ok(Self {
                    #(#from_row_self,)*
                })
            }

            fn to_row(&self) -> easydoc_core::Result<Vec<easydoc_core::CellData>> {
                Ok(vec![
                    #(#to_row_cells,)*
                ])
            }

            fn to_row_with_converters(
                &self,
                registry: &easydoc_core::ConverterRegistry,
            ) -> easydoc_core::Result<Vec<easydoc_core::CellData>> {
                let __schema = Self::schema();
                Ok(vec![
                    #(#to_row_converter_cells,)*
                ])
            }
        }
    };

    Ok(expanded)
}

fn parse_struct_attrs(attrs: &[syn::Attribute]) -> StructConfig {
    let mut config = StructConfig {
        banded_rows: false,
        auto_width: false,
    };

    for attr in attrs {
        if !attr.path().is_ident("docx") {
            continue;
        }
        if let Ok(nested) =
            attr.parse_args_with(Punctuated::<MetaNameValue, Comma>::parse_terminated)
        {
            for nv in nested {
                let key = nv
                    .path
                    .get_ident()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default();
                match key.as_str() {
                    "banded_rows" => {
                        if let syn::Expr::Lit(lit) = &nv.value
                            && let syn::Lit::Bool(b) = &lit.lit
                        {
                            config.banded_rows = b.value();
                        }
                    }
                    "auto_width" | "table_width" => {
                        if let syn::Expr::Lit(lit) = &nv.value
                            && let syn::Lit::Bool(b) = &lit.lit
                        {
                            config.auto_width = b.value();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    config
}

fn collect_fields(fields: &syn::Fields) -> syn::Result<Vec<FieldConfig>> {
    let mut result = Vec::new();

    for (i, field) in fields.iter().enumerate() {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| syn::Error::new_spanned(field, "unnamed fields are not supported"))?;
        let ty = field.ty.clone();

        // Parse #[docx(...)] attributes
        let mut name = ident.to_string();
        let mut index = i;
        let mut order = i as u32;
        let mut width = None;
        let mut format = None;
        let mut align = None;
        let mut converter = None;
        let mut wrap = false;
        let mut ignored = false;

        for attr in &field.attrs {
            if !attr.path().is_ident("docx") {
                continue;
            }

            // Parse as list of name-value pairs
            if let Ok(nested) =
                attr.parse_args_with(Punctuated::<MetaNameValue, Comma>::parse_terminated)
            {
                for nv in nested {
                    let key = nv
                        .path
                        .get_ident()
                        .map(std::string::ToString::to_string)
                        .unwrap_or_default();
                    match key.as_str() {
                        "name" => {
                            if let syn::Expr::Lit(lit) = &nv.value
                                && let syn::Lit::Str(s) = &lit.lit
                            {
                                name = s.value();
                            }
                        }
                        "index" => {
                            if let syn::Expr::Lit(lit) = &nv.value
                                && let syn::Lit::Int(n) = &lit.lit
                                && let Ok(v) = n.base10_parse::<usize>()
                            {
                                index = v;
                                order = v as u32;
                            }
                        }
                        "order" => {
                            if let syn::Expr::Lit(lit) = &nv.value
                                && let syn::Lit::Int(n) = &lit.lit
                                && let Ok(v) = n.base10_parse::<u32>()
                            {
                                order = v;
                            }
                        }
                        "width" => {
                            if let syn::Expr::Lit(lit) = &nv.value
                                && let syn::Lit::Str(s) = &lit.lit
                            {
                                width = Some(s.value());
                            }
                        }
                        "format" => {
                            if let syn::Expr::Lit(lit) = &nv.value
                                && let syn::Lit::Str(s) = &lit.lit
                            {
                                format = Some(s.value());
                            }
                        }
                        "align" => {
                            if let syn::Expr::Lit(lit) = &nv.value
                                && let syn::Lit::Str(s) = &lit.lit
                            {
                                let v = s.value();
                                validate_align(&v, s.span())?;
                                align = Some(v);
                            }
                        }
                        "converter" => {
                            // converter = TypePath — parsed as a path expression
                            if let syn::Expr::Path(path) = &nv.value {
                                let path_str = path
                                    .path
                                    .segments
                                    .iter()
                                    .map(|seg| seg.ident.to_string())
                                    .collect::<Vec<_>>()
                                    .join("::");
                                converter = Some(path_str);
                            }
                        }
                        "wrap" => {
                            if let syn::Expr::Lit(lit) = &nv.value
                                && let syn::Lit::Bool(b) = &lit.lit
                            {
                                wrap = b.value();
                            }
                        }
                        "ignore" => {
                            ignored = true;
                        }
                        _ => {}
                    }
                }
            } else {
                // Try parsing as a path (for #[docx(ignore)])
                if let Ok(path) = attr.parse_args::<syn::Path>()
                    && path.is_ident("ignore")
                {
                    ignored = true;
                }
            }
        }

        result.push(FieldConfig {
            ident,
            name,
            index,
            order,
            width,
            format,
            align,
            converter,
            wrap,
            ignored,
            ty,
        });
    }

    Ok(result)
}

/// Validates an `align` attribute value and returns an error if it is not one
/// of the accepted identifiers.
fn validate_align(value: &str, span: proc_macro2::Span) -> syn::Result<()> {
    match value {
        "left" | "center" | "right" | "justify" | "both" => Ok(()),
        _ => Err(syn::Error::new(
            span,
            format!(
                "unknown align value '{value}', expected one of: left, center, right, justify, both"
            ),
        )),
    }
}

/// Maps an `align` attribute string to the corresponding
/// `HorizontalAlignment` variant identifier for code generation.
///
/// Note: `"justify"` maps to `Both` since they are semantically identical
/// in OOXML (`<w:jc w:val="both"/>` renders as justified text).
fn align_variant_token(value: &str) -> proc_macro2::Ident {
    let variant = match value {
        "left" => "Left",
        "center" => "Center",
        "right" => "Right",
        "justify" | "both" => "Both",
        _ => unreachable!("align value should be validated earlier"),
    };
    proc_macro2::Ident::new(variant, proc_macro2::Span::call_site())
}
