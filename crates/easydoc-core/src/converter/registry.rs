//! Global converter registry with `TypeId`-based dispatch.
//!
//! Mirrors `ConverterRegistry` from `easyexcel-core`.

use crate::error::{DocError, Result};
use crate::metadata::TableColumn;
use crate::traits::DocConverter;
use crate::types::DocValue;
use std::any::{Any, TypeId};
use std::collections::HashMap;

// chrono support
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

/// A registry holding user-registered and built-in [`DocConverter`] instances.
///
/// Converters are keyed by the `TypeId` of the Rust type they handle.
/// The registry is typically populated via builder `.register_converter()` calls,
/// then passed to `from_row_with_converters` / `to_row_with_converters`.
#[derive(Default)]
pub struct ConverterRegistry {
    converters: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ConverterRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            converters: HashMap::new(),
        }
    }

    /// Registers a converter for type `V`.
    pub fn register<V: 'static, C: DocConverter<V> + Send + Sync + 'static>(
        &mut self,
        converter: C,
    ) {
        self.converters
            .insert(TypeId::of::<V>(), Box::new(converter));
    }

    /// Returns `true` if a converter is registered for type `V`.
    #[must_use]
    pub fn contains<V: 'static>(&self) -> bool {
        self.converters.contains_key(&TypeId::of::<V>())
    }

    /// Converts a Rust value into a [`DocValue`] using registered converters.
    ///
    /// Falls back to built-in conversions if no custom converter is registered.
    ///
    /// # Errors
    ///
    /// Returns [`DocError::Conversion`] if no suitable converter is found.
    pub fn to_doc_value<V: 'static + std::fmt::Debug>(
        &self,
        value: &V,
        column: &TableColumn,
    ) -> Result<DocValue> {
        let type_id = TypeId::of::<V>();
        if let Some(boxed) = self.converters.get(&type_id) {
            // We know the type — downcast and call
            if let Some(converter) = boxed.downcast_ref::<Box<dyn DocConverter<V>>>() {
                return converter.to_doc_value(value, column);
            }
        }
        // Fallback: try built-in conversion via Display/Debug
        fallback_to_doc_value(value, column)
    }

    /// Converts a [`DocValue`] into a Rust value of type `V`.
    ///
    /// # Errors
    ///
    /// Returns [`DocError::Conversion`] if no suitable converter is found or the
    /// value cannot be converted.
    pub fn from_doc_value<V: 'static>(&self, value: &DocValue, column: &TableColumn) -> Result<V> {
        let type_id = TypeId::of::<V>();
        if let Some(boxed) = self.converters.get(&type_id)
            && let Some(converter) = boxed.downcast_ref::<Box<dyn DocConverter<V>>>()
        {
            return converter.from_doc_value(value, column);
        }
        fallback_from_doc_value(value, column)
    }
}

impl std::fmt::Debug for ConverterRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConverterRegistry")
            .field("count", &self.converters.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Fallback conversions (used when no custom converter is registered)
// ---------------------------------------------------------------------------

/// Trait for safe fallback conversion — used when no custom converter is registered.
///
/// Types that implement this trait (via the blanket impl below) can be
/// converted to/from `DocValue` using only safe code.
trait FallbackConvert: Sized {
    fn to_doc_value_from_ref(&self) -> DocValue;
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self>;
}

// Direct implementations for common types.
impl FallbackConvert for String {
    fn to_doc_value_from_ref(&self) -> DocValue {
        DocValue::String(self.clone())
    }
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self> {
        match value {
            DocValue::String(s) => Ok(s.clone()),
            DocValue::Int(n) => Ok(n.to_string()),
            DocValue::Float(n) => Ok(n.to_string()),
            DocValue::Bool(b) => Ok(b.to_string()),
            DocValue::Empty => Ok(String::new()),
            other => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "cannot convert to String".to_owned(),
            }),
        }
    }
}

impl FallbackConvert for i64 {
    fn to_doc_value_from_ref(&self) -> DocValue {
        DocValue::Int(*self)
    }
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self> {
        match value {
            DocValue::Int(n) => Ok(*n),
            DocValue::String(s) => s.parse().map_err(|_| DocError::Conversion {
                field: column.field_name.clone(),
                value: s.clone(),
                message: "cannot parse as i64".to_owned(),
            }),
            other => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "cannot convert to i64".to_owned(),
            }),
        }
    }
}

impl FallbackConvert for i32 {
    fn to_doc_value_from_ref(&self) -> DocValue {
        DocValue::Int(i64::from(*self))
    }
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self> {
        match value {
            DocValue::Int(n) => Ok(*n as i32),
            DocValue::String(s) => s.parse().map_err(|_| DocError::Conversion {
                field: column.field_name.clone(),
                value: s.clone(),
                message: "cannot parse as i32".to_owned(),
            }),
            other => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "cannot convert to i32".to_owned(),
            }),
        }
    }
}

impl FallbackConvert for u32 {
    fn to_doc_value_from_ref(&self) -> DocValue {
        DocValue::Int(i64::from(*self))
    }
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self> {
        match value {
            DocValue::Int(n) => Ok(*n as u32),
            DocValue::String(s) => s.parse().map_err(|_| DocError::Conversion {
                field: column.field_name.clone(),
                value: s.clone(),
                message: "cannot parse as u32".to_owned(),
            }),
            other => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "cannot convert to u32".to_owned(),
            }),
        }
    }
}

impl FallbackConvert for f64 {
    fn to_doc_value_from_ref(&self) -> DocValue {
        DocValue::Float(*self)
    }
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self> {
        match value {
            DocValue::Float(n) => Ok(*n),
            DocValue::Int(n) => Ok(*n as f64),
            DocValue::String(s) => s.parse().map_err(|_| DocError::Conversion {
                field: column.field_name.clone(),
                value: s.clone(),
                message: "cannot parse as f64".to_owned(),
            }),
            other => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "cannot convert to f64".to_owned(),
            }),
        }
    }
}

impl FallbackConvert for bool {
    fn to_doc_value_from_ref(&self) -> DocValue {
        DocValue::Bool(*self)
    }
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self> {
        match value {
            DocValue::Bool(b) => Ok(*b),
            DocValue::String(s) => {
                let lower = s.to_lowercase();
                if lower == "true" || lower == "1" || lower == "yes" {
                    Ok(true)
                } else if lower == "false" || lower == "0" || lower == "no" {
                    Ok(false)
                } else {
                    Err(DocError::Conversion {
                        field: column.field_name.clone(),
                        value: s.clone(),
                        message: "cannot parse as bool".to_owned(),
                    })
                }
            }
            DocValue::Int(n) => Ok(*n != 0),
            other => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "cannot convert to bool".to_owned(),
            }),
        }
    }
}

impl FallbackConvert for DateTime<Utc> {
    fn to_doc_value_from_ref(&self) -> DocValue {
        DocValue::DateTime(*self)
    }
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self> {
        match value {
            DocValue::DateTime(dt) => Ok(*dt),
            DocValue::String(s) => s.parse().map_err(|_| DocError::Conversion {
                field: column.field_name.clone(),
                value: s.clone(),
                message: "cannot parse as DateTime<Utc>".to_owned(),
            }),
            other => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "cannot convert to DateTime<Utc>".to_owned(),
            }),
        }
    }
}

impl FallbackConvert for NaiveDate {
    fn to_doc_value_from_ref(&self) -> DocValue {
        DocValue::Date(*self)
    }
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self> {
        match value {
            DocValue::Date(d) => Ok(*d),
            DocValue::DateTime(dt) => Ok(dt.date_naive()),
            DocValue::String(s) => {
                NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| DocError::Conversion {
                    field: column.field_name.clone(),
                    value: s.clone(),
                    message: "cannot parse as NaiveDate (expected YYYY-MM-DD)".to_owned(),
                })
            }
            other => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "cannot convert to NaiveDate".to_owned(),
            }),
        }
    }
}

impl FallbackConvert for NaiveDateTime {
    fn to_doc_value_from_ref(&self) -> DocValue {
        DocValue::NaiveDateTime(*self)
    }
    fn from_doc_value(value: &DocValue, column: &TableColumn) -> Result<Self> {
        match value {
            DocValue::NaiveDateTime(ndt) => Ok(*ndt),
            DocValue::DateTime(dt) => Ok(dt.naive_utc()),
            DocValue::String(s) => {
                NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").map_err(|_| {
                    DocError::Conversion {
                        field: column.field_name.clone(),
                        value: s.clone(),
                        message: "cannot parse as NaiveDateTime".to_owned(),
                    }
                })
            }
            other => Err(DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{other:?}"),
                message: "cannot convert to NaiveDateTime".to_owned(),
            }),
        }
    }
}

fn fallback_to_doc_value<V: 'static + std::fmt::Debug>(
    value: &V,
    _column: &TableColumn,
) -> Result<DocValue> {
    let type_id = TypeId::of::<V>();

    if type_id == TypeId::of::<String>() {
        // Use the FallbackConvert impl — but we need to get the value as &String
        // Since TypeId matches, we can use Any::downcast_ref safely
        let any_val = value as &dyn Any;
        if let Some(s) = any_val.downcast_ref::<String>() {
            return Ok(<String as FallbackConvert>::to_doc_value_from_ref(s));
        }
    }
    if type_id == TypeId::of::<i64>() {
        let any_val = value as &dyn Any;
        if let Some(n) = any_val.downcast_ref::<i64>() {
            return Ok(<i64 as FallbackConvert>::to_doc_value_from_ref(n));
        }
    }
    if type_id == TypeId::of::<i32>() {
        let any_val = value as &dyn Any;
        if let Some(n) = any_val.downcast_ref::<i32>() {
            return Ok(<i32 as FallbackConvert>::to_doc_value_from_ref(n));
        }
    }
    if type_id == TypeId::of::<u32>() {
        let any_val = value as &dyn Any;
        if let Some(n) = any_val.downcast_ref::<u32>() {
            return Ok(<u32 as FallbackConvert>::to_doc_value_from_ref(n));
        }
    }
    if type_id == TypeId::of::<f64>() {
        let any_val = value as &dyn Any;
        if let Some(n) = any_val.downcast_ref::<f64>() {
            return Ok(<f64 as FallbackConvert>::to_doc_value_from_ref(n));
        }
    }
    if type_id == TypeId::of::<bool>() {
        let any_val = value as &dyn Any;
        if let Some(b) = any_val.downcast_ref::<bool>() {
            return Ok(<bool as FallbackConvert>::to_doc_value_from_ref(b));
        }
    }
    if type_id == TypeId::of::<DateTime<Utc>>() {
        let any_val = value as &dyn Any;
        if let Some(dt) = any_val.downcast_ref::<DateTime<Utc>>() {
            return Ok(<DateTime<Utc> as FallbackConvert>::to_doc_value_from_ref(
                dt,
            ));
        }
    }
    if type_id == TypeId::of::<NaiveDate>() {
        let any_val = value as &dyn Any;
        if let Some(d) = any_val.downcast_ref::<NaiveDate>() {
            return Ok(<NaiveDate as FallbackConvert>::to_doc_value_from_ref(d));
        }
    }
    if type_id == TypeId::of::<NaiveDateTime>() {
        let any_val = value as &dyn Any;
        if let Some(ndt) = any_val.downcast_ref::<NaiveDateTime>() {
            return Ok(<NaiveDateTime as FallbackConvert>::to_doc_value_from_ref(
                ndt,
            ));
        }
    }

    // Last resort: format via Debug
    Ok(DocValue::String(format!("{value:?}")))
}

fn fallback_from_doc_value<V: 'static>(value: &DocValue, column: &TableColumn) -> Result<V> {
    let type_id = TypeId::of::<V>();

    let err = |msg: &str| -> Result<V> {
        Err(DocError::Conversion {
            field: column.field_name.clone(),
            value: format!("{value:?}"),
            message: msg.to_owned(),
        })
    };

    if type_id == TypeId::of::<String>() {
        let s: String = <String as FallbackConvert>::from_doc_value(value, column)?;
        // Convert through Any — since TypeId matches, this is safe
        let any_box: Box<dyn Any> = Box::new(s);
        match any_box.downcast::<V>() {
            Ok(boxed) => Ok(*boxed),
            Err(_) => err("type mismatch for String"),
        }
    } else if type_id == TypeId::of::<i64>() {
        let n: i64 = <i64 as FallbackConvert>::from_doc_value(value, column)?;
        let any_box: Box<dyn Any> = Box::new(n);
        match any_box.downcast::<V>() {
            Ok(boxed) => Ok(*boxed),
            Err(_) => err("type mismatch for i64"),
        }
    } else if type_id == TypeId::of::<i32>() {
        let n: i32 = <i32 as FallbackConvert>::from_doc_value(value, column)?;
        let any_box: Box<dyn Any> = Box::new(n);
        match any_box.downcast::<V>() {
            Ok(boxed) => Ok(*boxed),
            Err(_) => err("type mismatch for i32"),
        }
    } else if type_id == TypeId::of::<u32>() {
        let n: u32 = <u32 as FallbackConvert>::from_doc_value(value, column)?;
        let any_box: Box<dyn Any> = Box::new(n);
        match any_box.downcast::<V>() {
            Ok(boxed) => Ok(*boxed),
            Err(_) => err("type mismatch for u32"),
        }
    } else if type_id == TypeId::of::<f64>() {
        let n: f64 = <f64 as FallbackConvert>::from_doc_value(value, column)?;
        let any_box: Box<dyn Any> = Box::new(n);
        match any_box.downcast::<V>() {
            Ok(boxed) => Ok(*boxed),
            Err(_) => err("type mismatch for f64"),
        }
    } else if type_id == TypeId::of::<bool>() {
        let b: bool = <bool as FallbackConvert>::from_doc_value(value, column)?;
        let any_box: Box<dyn Any> = Box::new(b);
        match any_box.downcast::<V>() {
            Ok(boxed) => Ok(*boxed),
            Err(_) => err("type mismatch for bool"),
        }
    } else if type_id == TypeId::of::<DateTime<Utc>>() {
        let dt: DateTime<Utc> = <DateTime<Utc> as FallbackConvert>::from_doc_value(value, column)?;
        let any_box: Box<dyn Any> = Box::new(dt);
        match any_box.downcast::<V>() {
            Ok(boxed) => Ok(*boxed),
            Err(_) => err("type mismatch for DateTime<Utc>"),
        }
    } else if type_id == TypeId::of::<NaiveDate>() {
        let d: NaiveDate = <NaiveDate as FallbackConvert>::from_doc_value(value, column)?;
        let any_box: Box<dyn Any> = Box::new(d);
        match any_box.downcast::<V>() {
            Ok(boxed) => Ok(*boxed),
            Err(_) => err("type mismatch for NaiveDate"),
        }
    } else if type_id == TypeId::of::<NaiveDateTime>() {
        let ndt: NaiveDateTime = <NaiveDateTime as FallbackConvert>::from_doc_value(value, column)?;
        let any_box: Box<dyn Any> = Box::new(ndt);
        match any_box.downcast::<V>() {
            Ok(boxed) => Ok(*boxed),
            Err(_) => err("type mismatch for NaiveDateTime"),
        }
    } else {
        err(&format!(
            "no converter registered for type {}",
            std::any::type_name::<V>()
        ))
    }
}
