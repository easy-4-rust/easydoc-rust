//! Implementation of the `#[derive(DocxRow)]` proc-macro.
//!
//! Generates an `impl DocxRow for YourStruct` block with:
//! - `schema()` — returns `&'static [TableColumn]`
//! - `from_row()` / `from_row_with_converters()` — row → struct
//! - `to_row()` / `to_row_with_converters()` — struct → row

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, MetaNameValue, Type, punctuated::Punctuated, token::Comma};

/// Parsed per-field attribute configuration.
struct FieldConfig {
    ident: syn::Ident,
    name: String,
    index: usize,
    order: u32,
    width: Option<f64>,
    format: Option<String>,
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
        let width = f.width.map(|w| quote! { Some(#w) }).unwrap_or_else(|| quote! { None });
        let format = f.format.as_ref().map(|fmt| quote! { Some(#fmt.to_owned()) }).unwrap_or_else(|| quote! { None });

        quote! {
            easydoc_core::TableColumn {
                name: #name.to_owned(),
                field_name: #field_name.to_owned(),
                index: #index,
                order: #order,
                width: #width,
                format: #format,
                ignored: false,
            }
        }
    });

    // Generate to_row body
    let to_row_cells = fields.iter().filter(|f| !f.ignored).map(|f| {
        let ident = &f.ident;
        let value_expr = match &f.format {
            Some(fmt_str) => {
                quote! {
                    {
                        let formatted = format!("{}", #fmt_str); // simple format hint
                        easydoc_core::CellData::new(self.#ident.to_string())
                    }
                }
            }
            None => {
                quote! {
                    easydoc_core::CellData::new(self.#ident.clone())
                }
            }
        };
        value_expr
    });

    let field_count = fields.iter().filter(|f| !f.ignored).count();

    // Generate from_row body
    let from_row_bindings = fields.iter().filter(|f| !f.ignored).enumerate().map(|(i, f)| {
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

    let from_row_self = fields.iter().filter(|f| !f.ignored).map(|f| {
        let ident = &f.ident;
        quote! { #ident }
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
                _registry: &easydoc_core::ConverterRegistry,
            ) -> easydoc_core::Result<Self> {
                Self::from_row(row)
            }

            fn to_row(&self) -> easydoc_core::Result<Vec<easydoc_core::CellData>> {
                Ok(vec![
                    #(#to_row_cells,)*
                ])
            }

            fn to_row_with_converters(
                &self,
                _registry: &easydoc_core::ConverterRegistry,
            ) -> easydoc_core::Result<Vec<easydoc_core::CellData>> {
                self.to_row()
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
        if let Ok(nested) = attr.parse_args_with(
            Punctuated::<MetaNameValue, Comma>::parse_terminated,
        ) {
            for nv in nested {
                let key = nv.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
                match key.as_str() {
                    "banded_rows" => {
                        if let syn::Expr::Lit(lit) = &nv.value {
                            if let syn::Lit::Bool(b) = &lit.lit {
                                config.banded_rows = b.value();
                            }
                        }
                    }
                    "auto_width" | "table_width" => {
                        if let syn::Expr::Lit(lit) = &nv.value {
                            if let syn::Lit::Bool(b) = &lit.lit {
                                config.auto_width = b.value();
                            }
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
        let mut ignored = false;

        for attr in &field.attrs {
            if !attr.path().is_ident("docx") {
                continue;
            }

            // Parse as list of name-value pairs
            if let Ok(nested) = attr.parse_args_with(
                Punctuated::<MetaNameValue, Comma>::parse_terminated,
            ) {
                for nv in nested {
                    let key = nv.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
                    match key.as_str() {
                        "name" => {
                            if let syn::Expr::Lit(lit) = &nv.value {
                                if let syn::Lit::Str(s) = &lit.lit {
                                    name = s.value();
                                }
                            }
                        }
                        "index" => {
                            if let syn::Expr::Lit(lit) = &nv.value {
                                if let syn::Lit::Int(n) = &lit.lit {
                                    if let Ok(v) = n.base10_parse::<usize>() {
                                        index = v;
                                        order = v as u32;
                                    }
                                }
                            }
                        }
                        "order" => {
                            if let syn::Expr::Lit(lit) = &nv.value {
                                if let syn::Lit::Int(n) = &lit.lit {
                                    if let Ok(v) = n.base10_parse::<u32>() {
                                        order = v;
                                    }
                                }
                            }
                        }
                        "width" => {
                            if let syn::Expr::Lit(lit) = &nv.value {
                                if let syn::Lit::Float(n) = &lit.lit {
                                    if let Ok(v) = n.base10_parse::<f64>() {
                                        width = Some(v);
                                    }
                                }
                            }
                        }
                        "format" => {
                            if let syn::Expr::Lit(lit) = &nv.value {
                                if let syn::Lit::Str(s) = &lit.lit {
                                    format = Some(s.value());
                                }
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
                if let Ok(path) = attr.parse_args::<syn::Path>() {
                    if path.is_ident("ignore") {
                        ignored = true;
                    }
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
            ignored,
            ty,
        });
    }

    Ok(result)
}
