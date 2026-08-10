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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ImageData;
    use chrono::{TimeZone, Utc};

    fn test_column() -> TableColumn {
        TableColumn::new("Test", "test", 0)
    }

    #[test]
    fn empty_registry() {
        let r = ConverterRegistry::new();
        assert!(!r.contains::<String>());
        let dbg = format!("{:?}", r);
        assert!(dbg.contains("count: 0"));
    }

    #[test]
    fn fallback_string_from_string_value() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("hello".into());
        let result: String = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn fallback_string_from_int_value() {
        let r = ConverterRegistry::new();
        let v = DocValue::Int(42);
        let result: String = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, "42");
    }

    #[test]
    fn fallback_string_from_float_value() {
        let r = ConverterRegistry::new();
        let v = DocValue::Float(3.14);
        let result: String = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, "3.14");
    }

    #[test]
    fn fallback_string_from_bool_value() {
        let r = ConverterRegistry::new();
        let v = DocValue::Bool(true);
        let result: String = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, "true");
    }

    #[test]
    fn fallback_string_from_empty() {
        let r = ConverterRegistry::new();
        let v = DocValue::Empty;
        let result: String = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn fallback_string_from_image_fails() {
        let r = ConverterRegistry::new();
        let v = DocValue::Image(ImageData {
            bytes: vec![],
            extension: "png".into(),
            width: None,
            height: None,
            alt_text: None,
        });
        let result: std::result::Result<String, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_i64_from_int() {
        let r = ConverterRegistry::new();
        let v = DocValue::Int(100);
        let result: i64 = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn fallback_i64_from_string_parse() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("256".into());
        let result: i64 = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, 256);
    }

    #[test]
    fn fallback_i64_from_invalid_string() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("abc".into());
        let result: std::result::Result<i64, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_f64_from_float() {
        let r = ConverterRegistry::new();
        let v = DocValue::Float(2.5);
        let result: f64 = r.from_doc_value(&v, &test_column()).unwrap();
        assert!((result - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn fallback_f64_from_string_parse() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("1.5".into());
        let result: f64 = r.from_doc_value(&v, &test_column()).unwrap();
        assert!((result - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn fallback_bool_from_bool() {
        let r = ConverterRegistry::new();
        let v = DocValue::Bool(false);
        let result: bool = r.from_doc_value(&v, &test_column()).unwrap();
        assert!(!result);
    }

    #[test]
    fn fallback_to_doc_value_string() {
        let r = ConverterRegistry::new();
        let val = String::from("test");
        let result = r.to_doc_value(&val, &test_column()).unwrap();
        assert!(matches!(result, DocValue::String(s) if s == "test"));
    }

    #[test]
    fn fallback_to_doc_value_i64() {
        let r = ConverterRegistry::new();
        let val: i64 = 42;
        let result = r.to_doc_value(&val, &test_column()).unwrap();
        assert!(matches!(result, DocValue::Int(42)));
    }

    #[test]
    fn fallback_to_doc_value_i32() {
        let r = ConverterRegistry::new();
        let val: i32 = 7;
        let result = r.to_doc_value(&val, &test_column()).unwrap();
        assert!(matches!(result, DocValue::Int(7)));
    }

    #[test]
    fn fallback_to_doc_value_u32() {
        let r = ConverterRegistry::new();
        let val: u32 = 99;
        let result = r.to_doc_value(&val, &test_column()).unwrap();
        assert!(matches!(result, DocValue::Int(99)));
    }

    #[test]
    fn fallback_to_doc_value_f64() {
        let r = ConverterRegistry::new();
        let val: f64 = 1.5;
        let result = r.to_doc_value(&val, &test_column()).unwrap();
        assert!(matches!(result, DocValue::Float(f) if (f - 1.5).abs() < f64::EPSILON));
    }

    #[test]
    fn fallback_to_doc_value_bool() {
        let r = ConverterRegistry::new();
        let val = true;
        let result = r.to_doc_value(&val, &test_column()).unwrap();
        assert!(matches!(result, DocValue::Bool(true)));
    }

    #[test]
    fn fallback_to_doc_value_datetime() {
        let r = ConverterRegistry::new();
        let val = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let result = r.to_doc_value(&val, &test_column()).unwrap();
        assert!(matches!(result, DocValue::DateTime(_)));
    }

    #[test]
    fn fallback_to_doc_value_naive_date() {
        let r = ConverterRegistry::new();
        let val = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let result = r.to_doc_value(&val, &test_column()).unwrap();
        assert!(matches!(result, DocValue::Date(_)));
    }

    #[test]
    fn fallback_to_doc_value_naive_datetime() {
        let r = ConverterRegistry::new();
        let val = NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let result = r.to_doc_value(&val, &test_column()).unwrap();
        assert!(matches!(result, DocValue::NaiveDateTime(_)));
    }

    #[test]
    fn fallback_i32_roundtrip() {
        let r = ConverterRegistry::new();
        let v = DocValue::Int(42);
        let result: i32 = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn fallback_u32_roundtrip() {
        let r = ConverterRegistry::new();
        let v = DocValue::Int(42);
        let result: u32 = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn fallback_datetime_from_string() {
        let r = ConverterRegistry::new();
        let v = DocValue::DateTime(Utc.timestamp_opt(1_700_000_000, 0).unwrap());
        let result: DateTime<Utc> = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result.timestamp(), 1_700_000_000);
    }

    #[test]
    fn fallback_naive_date_from_date() {
        let r = ConverterRegistry::new();
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let v = DocValue::Date(d);
        let result: NaiveDate = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, d);
    }

    #[test]
    fn fallback_naive_datetime_from_ndt() {
        let r = ConverterRegistry::new();
        let ndt = NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let v = DocValue::NaiveDateTime(ndt);
        let result: NaiveDateTime = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, ndt);
    }

    #[test]
    fn fallback_i64_from_float_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Float(3.14);
        let result: std::result::Result<i64, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_i64_from_bool_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Bool(true);
        let result: std::result::Result<i64, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_i64_from_empty_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Empty;
        let result: std::result::Result<i64, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_i32_from_string_parse() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("42".into());
        let result: i32 = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn fallback_i32_from_invalid_string() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("abc".into());
        let result: std::result::Result<i32, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_i32_from_float_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Float(3.14);
        let result: std::result::Result<i32, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_u32_from_string_parse() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("42".into());
        let result: u32 = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn fallback_u32_from_invalid_string() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("abc".into());
        let result: std::result::Result<u32, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_u32_from_float_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Float(3.14);
        let result: std::result::Result<u32, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_f64_from_int() {
        let r = ConverterRegistry::new();
        let v = DocValue::Int(42);
        let result: f64 = r.from_doc_value(&v, &test_column()).unwrap();
        assert!((result - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn fallback_f64_from_invalid_string() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("abc".into());
        let result: std::result::Result<f64, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_f64_from_bool_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Bool(true);
        let result: std::result::Result<f64, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_bool_from_string_true() {
        let r = ConverterRegistry::new();
        for s in &["true", "True", "TRUE", "1", "yes", "Yes"] {
            let v = DocValue::String(s.to_string());
            let result: bool = r.from_doc_value(&v, &test_column()).unwrap();
            assert!(result, "failed for {s}");
        }
    }

    #[test]
    fn fallback_bool_from_string_false() {
        let r = ConverterRegistry::new();
        for s in &["false", "False", "FALSE", "0", "no", "No"] {
            let v = DocValue::String(s.to_string());
            let result: bool = r.from_doc_value(&v, &test_column()).unwrap();
            assert!(!result, "failed for {s}");
        }
    }

    #[test]
    fn fallback_bool_from_invalid_string() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("maybe".into());
        let result: std::result::Result<bool, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_bool_from_int() {
        let r = ConverterRegistry::new();
        let v = DocValue::Int(1);
        let result: bool = r.from_doc_value(&v, &test_column()).unwrap();
        assert!(result);
        let v2 = DocValue::Int(0);
        let result2: bool = r.from_doc_value(&v2, &test_column()).unwrap();
        assert!(!result2);
    }

    #[test]
    fn fallback_bool_from_float_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Float(1.0);
        let result: std::result::Result<bool, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_datetime_from_datetime() {
        let r = ConverterRegistry::new();
        let dt = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let v = DocValue::DateTime(dt);
        let result: DateTime<Utc> = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result.timestamp(), 1_700_000_000);
    }

    #[test]
    fn fallback_datetime_from_string_parse() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("2023-11-14T22:13:20Z".into());
        let result: DateTime<Utc> = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result.timestamp(), 1_700_000_000);
    }

    #[test]
    fn fallback_datetime_from_invalid_string() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("not-a-date".into());
        let result: std::result::Result<DateTime<Utc>, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_datetime_from_int_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Int(100);
        let result: std::result::Result<DateTime<Utc>, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_naive_date_from_string_parse() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("2024-01-15".into());
        let result: NaiveDate = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    }

    #[test]
    fn fallback_naive_date_from_datetime() {
        let r = ConverterRegistry::new();
        let dt = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let v = DocValue::DateTime(dt);
        let result: NaiveDate = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, dt.date_naive());
    }

    #[test]
    fn fallback_naive_date_from_invalid_string() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("not-a-date".into());
        let result: std::result::Result<NaiveDate, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_naive_date_from_int_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Int(100);
        let result: std::result::Result<NaiveDate, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_naive_datetime_from_string_parse() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("2024-06-01 12:00:00".into());
        let result: NaiveDateTime = r.from_doc_value(&v, &test_column()).unwrap();
        let expected = NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn fallback_naive_datetime_from_datetime() {
        let r = ConverterRegistry::new();
        let dt = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let v = DocValue::DateTime(dt);
        let result: NaiveDateTime = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, dt.naive_utc());
    }

    #[test]
    fn fallback_naive_datetime_from_invalid_string() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("not-a-datetime".into());
        let result: std::result::Result<NaiveDateTime, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_naive_datetime_from_int_error() {
        let r = ConverterRegistry::new();
        let v = DocValue::Int(100);
        let result: std::result::Result<NaiveDateTime, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_to_doc_value_debug_fallback() {
        let r = ConverterRegistry::new();
        // Vec<u8> has no FallbackConvert impl but implements Debug
        // so it falls back to Debug string representation
        let val: Vec<u8> = vec![1, 2, 3];
        let result = r.to_doc_value(&val, &test_column());
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), DocValue::String(s) if s.contains("1")));
    }

    #[test]
    fn fallback_from_doc_value_unsupported_type() {
        let r = ConverterRegistry::new();
        let v = DocValue::String("test".into());
        // Try to convert to a type with no FallbackConvert
        let result: std::result::Result<Vec<u8>, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn registry_contains_after_register() {
        let mut r = ConverterRegistry::new();
        assert!(!r.contains::<String>());
        // Note: we can't easily register a converter without a concrete type
        // but we can test the contains method
    }

    #[test]
    fn registry_debug_format() {
        let r = ConverterRegistry::new();
        let dbg = format!("{:?}", r);
        assert!(dbg.contains("ConverterRegistry"));
        assert!(dbg.contains("count: 0"));
    }

    #[test]
    fn fallback_string_from_richtext() {
        let r = ConverterRegistry::new();
        let v = DocValue::RichText(vec![]);
        let result: std::result::Result<String, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }

    #[test]
    fn fallback_string_from_datetime() {
        let r = ConverterRegistry::new();
        let dt = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let v = DocValue::DateTime(dt);
        let result: std::result::Result<String, _> = r.from_doc_value(&v, &test_column());
        assert!(result.is_err());
    }
}
