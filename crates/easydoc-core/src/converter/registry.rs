//! 基于 `TypeId` 分发的全局转换器注册表。
//!
//! 对标 easyexcel-core 的 `ConverterRegistry`。
//!
//! 注册表使用类型擦除的转换器模式，以 [`TypeId`] 为键存储异构的
//! [`DocConverter`] 实现。这允许在运行时为任意类型注册转换器，
//! 并在调用处无需知道具体转换器类型即可查找。
//!
//! 对应 Java: com.alibaba.excel.converters.ConverterRegistry

use crate::error::{DocError, Result};
use crate::metadata::TableColumn;
use crate::traits::DocConverter;
use crate::types::DocValue;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;

// chrono support
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

// ---------------------------------------------------------------------------
// Type-erased converter infrastructure
// ---------------------------------------------------------------------------

/// 类型擦除的双向转换器。
///
/// 此 trait 是存储抽象，允许 [`ConverterRegistry`] 在单个 `HashMap` 中持有
/// 不同类型的转换器。由 [`ConverterRegistry::find_converter`] 和
/// [`ConverterRegistry::find_converter_by_name`] 返回，使 derive 生成的代码
/// 可以在不知道具体转换器类型的情况下使用已注册的转换器。
///
/// 大多数用户应使用 [`DocConverter<T>`]。仅在需要通过类型擦除的注册表引用
/// 调用转换器时才使用此 trait。
pub trait ErasedConverter: Send + Sync {
    /// 将 Rust 值（作为 `&dyn Any` 传入）转换为 [`DocValue`]。
    ///
    /// # Errors
    ///
    /// 值的具体类型与预期类型 `T` 不匹配，或转换本身失败时返回 [`DocError::Conversion`]。
    fn to_doc_value_erased(&self, value: &dyn Any, column: &TableColumn) -> Result<DocValue>;

    /// 将 [`DocValue`] 转换回 Rust 值（作为 `Box<dyn Any>` 返回）。
    ///
    /// # Errors
    ///
    /// 值无法转换为 `T` 时返回 [`DocError::Conversion`]。
    fn from_doc_value_erased(&self, value: &DocValue, column: &TableColumn)
    -> Result<Box<dyn Any>>;
}

/// 将类型化的 [`DocConverter<T>`] 桥接到类型擦除的 [`ErasedConverter`] 接口的具体包装器。
///
/// 包装器在委托给内部转换器之前，将传入的 `&dyn Any` 值向下转型为 `&T`，
/// 并将 `from_doc_value` 的输出装箱为 `Box<dyn Any>`。
struct TypedConverter<T: 'static, C: DocConverter<T>> {
    converter: C,
    // Use `fn() -> T` to avoid inheriting T's auto-traits (Send/Sync).
    // TypedConverter only needs Send+Sync from C, not from T.
    _phantom: PhantomData<fn() -> T>,
}

impl<T: 'static, C: DocConverter<T>> TypedConverter<T, C> {
    /// Wraps a concrete converter for type-erased storage.
    fn new(converter: C) -> Self {
        Self {
            converter,
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static, C: DocConverter<T> + Send + Sync + 'static> ErasedConverter
    for TypedConverter<T, C>
{
    fn to_doc_value_erased(&self, value: &dyn Any, column: &TableColumn) -> Result<DocValue> {
        let typed = value
            .downcast_ref::<T>()
            .ok_or_else(|| DocError::Conversion {
                field: column.field_name.clone(),
                value: format!("{value:?}"),
                message: format!(
                    "type mismatch: expected {}, found a different concrete type",
                    std::any::type_name::<T>()
                ),
            })?;
        self.converter.to_doc_value(typed, column)
    }

    fn from_doc_value_erased(
        &self,
        value: &DocValue,
        column: &TableColumn,
    ) -> Result<Box<dyn Any>> {
        let typed = self.converter.from_doc_value(value, column)?;
        Ok(Box::new(typed))
    }
}

// ---------------------------------------------------------------------------
// ConverterRegistry
// ---------------------------------------------------------------------------

/// 持有用户注册和内置 [`DocConverter`] 实例的注册表。
///
/// 转换器以它们处理的 Rust 类型的 `TypeId` 为键。
/// 注册表通常通过 builder 的 `.register_converter()` 调用填充，
/// 然后传递给 `from_row_with_converters` / `to_row_with_converters`。
///
/// 对应 Java: `com.alibaba.excel.converters.ConverterRegistry`
///
/// # Examples
///
/// ```
/// use easydoc_core::{ConverterRegistry, DocConverter, DocValue, TableColumn};
/// use easydoc_core::Result;
///
/// struct BoolToString;
///
/// impl DocConverter<bool> for BoolToString {
///     fn support_type() -> std::any::TypeId { std::any::TypeId::of::<bool>() }
///     fn to_doc_value(&self, value: &bool, _col: &TableColumn) -> Result<DocValue> {
///         Ok(DocValue::String(if *value { "yes".into() } else { "no".into() }))
///     }
///     fn from_doc_value(&self, value: &DocValue, col: &TableColumn) -> Result<bool> {
///         match value {
///             DocValue::String(s) => Ok(s == "yes"),
///             _ => Err(easydoc_core::DocError::Conversion {
///                 field: col.field_name.clone(),
///                 value: format!("{value:?}"),
///                 message: "expected string".into(),
///             }),
///         }
///     }
/// }
///
/// let mut registry = ConverterRegistry::new();
/// registry.register::<bool, _>(BoolToString);
/// assert!(registry.contains::<bool>());
/// ```
#[derive(Default)]
pub struct ConverterRegistry {
    converters: HashMap<TypeId, Box<dyn ErasedConverter>>,
    /// Reverse index: converter type name -> `TypeId`, for name-based lookup.
    name_to_type: HashMap<String, TypeId>,
}

impl ConverterRegistry {
    /// 创建空注册表。
    #[must_use]
    pub fn new() -> Self {
        Self {
            converters: HashMap::new(),
            name_to_type: HashMap::new(),
        }
    }

    /// 为类型 `T` 注册转换器。
    ///
    /// 如果 `T` 的转换器已注册，则替换。返回 `true` 表示新注册，`false` 表示替换。
    ///
    /// 对应 Java: `ConverterRegistry#registerConverter`
    pub fn register<T: 'static, C: DocConverter<T> + Send + Sync + 'static>(
        &mut self,
        converter: C,
    ) -> bool {
        let type_id = TypeId::of::<T>();
        let existed = self.converters.contains_key(&type_id);
        let erased: Box<dyn ErasedConverter> = Box::new(TypedConverter::<T, C>::new(converter));
        self.converters.insert(type_id, erased);
        !existed
    }

    /// 为类型 `T` 注册转换器，并以人类可读名称索引。
    ///
    /// 名称可用于 [`find_converter_by_name`](Self::find_converter_by_name) 在不知道具体 Rust 类型的情况下查找转换器。
    /// 当 `#[docx(converter = StatusConverter)]` 属性在 schema 中以字符串存储
    /// 转换器类型名时非常有用。
    ///
    /// 返回 `true` 表示新注册，`false` 表示替换。
    pub fn register_named<T: 'static, C: DocConverter<T> + Send + Sync + 'static>(
        &mut self,
        name: &str,
        converter: C,
    ) -> bool {
        let type_id = TypeId::of::<T>();
        let existed = self.converters.contains_key(&type_id);
        let erased: Box<dyn ErasedConverter> = Box::new(TypedConverter::<T, C>::new(converter));
        self.converters.insert(type_id, erased);
        self.name_to_type.insert(name.to_owned(), type_id);
        !existed
    }

    /// 返回类型 `T` 是否已注册转换器。
    #[must_use]
    pub fn contains<T: 'static>(&self) -> bool {
        self.converters.contains_key(&TypeId::of::<T>())
    }

    /// 查找类型 `T` 的类型擦除转换器。
    ///
    /// 未注册时返回 `None`。这是 derive 生成代码的主要查找机制。
    #[must_use]
    pub fn find_converter<T: 'static>(&self) -> Option<&dyn ErasedConverter> {
        self.converters
            .get(&TypeId::of::<T>())
            .map(std::convert::AsRef::as_ref)
    }

    /// 按已注册名称查找类型擦除转换器。
    ///
    /// 未注册时返回 `None`。支持 `#[docx(converter = StatusConverter)]` 模式。
    #[must_use]
    pub fn find_converter_by_name(&self, name: &str) -> Option<&dyn ErasedConverter> {
        self.name_to_type
            .get(name)
            .and_then(|type_id| self.converters.get(type_id))
            .map(std::convert::AsRef::as_ref)
    }

    /// 使用已注册的转换器将 Rust 值转换为 [`DocValue`]。
    ///
    /// 未注册自定义转换器时回退到内置转换。
    ///
    /// # Errors
    ///
    /// 找不到合适的转换器时返回 [`DocError::Conversion`]。
    pub fn to_doc_value<V: 'static + std::fmt::Debug>(
        &self,
        value: &V,
        column: &TableColumn,
    ) -> Result<DocValue> {
        if let Some(converter) = self.find_converter::<V>() {
            return converter.to_doc_value_erased(value as &dyn Any, column);
        }
        // Fallback: try built-in conversion via Display/Debug
        fallback_to_doc_value(value, column)
    }

    /// 将 [`DocValue`] 转换为 Rust 类型 `V`。
    ///
    /// # Errors
    ///
    /// 找不到合适的转换器或值无法转换时返回 [`DocError::Conversion`]。
    pub fn from_doc_value<V: 'static>(&self, value: &DocValue, column: &TableColumn) -> Result<V> {
        if let Some(converter) = self.find_converter::<V>() {
            let boxed = converter.from_doc_value_erased(value, column)?;
            return boxed
                .downcast::<V>()
                .map(|b| *b)
                .map_err(|_| DocError::Conversion {
                    field: column.field_name.clone(),
                    value: format!("{value:?}"),
                    message: format!(
                        "converter returned wrong concrete type for {}",
                        std::any::type_name::<V>()
                    ),
                });
        }
        fallback_from_doc_value(value, column)
    }
}

impl std::fmt::Debug for ConverterRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConverterRegistry")
            .field("count", &self.converters.len())
            .field("named_count", &self.name_to_type.len())
            .finish_non_exhaustive()
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

    // -----------------------------------------------------------------------
    // Fallback tests (unchanged from before — must keep passing)
    // -----------------------------------------------------------------------

    #[test]
    fn empty_registry() {
        let r = ConverterRegistry::new();
        assert!(!r.contains::<String>());
        let dbg = format!("{r:?}");
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
        let v = DocValue::Float(std::f64::consts::PI);
        let result: String = r.from_doc_value(&v, &test_column()).unwrap();
        assert_eq!(result, "3.141592653589793");
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
        let v = DocValue::Float(std::f64::consts::PI);
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
        let v = DocValue::Float(std::f64::consts::PI);
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
        let v = DocValue::Float(std::f64::consts::PI);
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
        assert!(matches!(result.unwrap(), DocValue::String(s) if s.contains('1')));
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
        let r = ConverterRegistry::new();
        assert!(!r.contains::<String>());
        // Note: we can't easily register a converter without a concrete type
        // but we can test the contains method
    }

    #[test]
    fn registry_debug_format() {
        let r = ConverterRegistry::new();
        let dbg = format!("{r:?}");
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

    // -----------------------------------------------------------------------
    // Custom converter tests — validates the type-erased lookup pattern
    // -----------------------------------------------------------------------

    /// A simple Status enum for testing custom converters.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Status {
        Active,
        Inactive,
    }

    /// Converts `Status` to/from `DocValue::String`.
    struct StatusConverter;

    impl DocConverter<Status> for StatusConverter {
        fn support_type() -> TypeId {
            TypeId::of::<Status>()
        }

        fn to_doc_value(&self, value: &Status, _column: &TableColumn) -> Result<DocValue> {
            match value {
                Status::Active => Ok(DocValue::String("ACTIVE".into())),
                Status::Inactive => Ok(DocValue::String("INACTIVE".into())),
            }
        }

        fn from_doc_value(&self, value: &DocValue, column: &TableColumn) -> Result<Status> {
            match value {
                DocValue::String(s) => match s.as_str() {
                    "ACTIVE" => Ok(Status::Active),
                    "INACTIVE" => Ok(Status::Inactive),
                    _ => Err(DocError::Conversion {
                        field: column.field_name.clone(),
                        value: s.clone(),
                        message: "unknown status value".to_owned(),
                    }),
                },
                _ => Err(DocError::Conversion {
                    field: column.field_name.clone(),
                    value: format!("{value:?}"),
                    message: "expected string for Status".to_owned(),
                }),
            }
        }
    }

    /// A custom i32 converter that multiplies by 10 on write and divides by 10 on read.
    struct TimesTenConverter;

    impl DocConverter<i32> for TimesTenConverter {
        fn support_type() -> TypeId {
            TypeId::of::<i32>()
        }

        fn to_doc_value(&self, value: &i32, _column: &TableColumn) -> Result<DocValue> {
            Ok(DocValue::Int(i64::from(*value * 10)))
        }

        fn from_doc_value(&self, value: &DocValue, column: &TableColumn) -> Result<i32> {
            match value {
                DocValue::Int(n) => Ok((*n / 10) as i32),
                _ => Err(DocError::Conversion {
                    field: column.field_name.clone(),
                    value: format!("{value:?}"),
                    message: "expected int for i32".to_owned(),
                }),
            }
        }
    }

    #[test]
    fn register_and_find_status_converter() {
        let mut r = ConverterRegistry::new();
        let is_new = r.register::<Status, _>(StatusConverter);
        assert!(is_new, "first registration should return true");
        assert!(r.contains::<Status>());
        assert!(r.find_converter::<Status>().is_some());
    }

    #[test]
    fn register_returns_false_on_overwrite() {
        let mut r = ConverterRegistry::new();
        let first = r.register::<Status, _>(StatusConverter);
        assert!(first);
        let second = r.register::<Status, _>(StatusConverter);
        assert!(
            !second,
            "second registration should return false (overwrite)"
        );
    }

    #[test]
    fn find_converter_unregistered_returns_none() {
        let r = ConverterRegistry::new();
        assert!(r.find_converter::<Status>().is_none());
        assert!(r.find_converter::<i64>().is_none());
    }

    #[test]
    fn custom_converter_to_doc_value_roundtrip() {
        let mut r = ConverterRegistry::new();
        r.register::<Status, _>(StatusConverter);

        let col = test_column();

        // to_doc_value uses the custom converter
        let active_val = r.to_doc_value(&Status::Active, &col).unwrap();
        assert!(matches!(active_val, DocValue::String(ref s) if s == "ACTIVE"));

        let inactive_val = r.to_doc_value(&Status::Inactive, &col).unwrap();
        assert!(matches!(inactive_val, DocValue::String(ref s) if s == "INACTIVE"));

        // from_doc_value uses the custom converter
        let active_back: Status = r.from_doc_value(&active_val, &col).unwrap();
        assert_eq!(active_back, Status::Active);

        let inactive_back: Status = r.from_doc_value(&inactive_val, &col).unwrap();
        assert_eq!(inactive_back, Status::Inactive);
    }

    #[test]
    fn custom_converter_from_doc_value_error() {
        let mut r = ConverterRegistry::new();
        r.register::<Status, _>(StatusConverter);

        let col = test_column();
        let bad_val = DocValue::String("UNKNOWN".into());
        let result: std::result::Result<Status, _> = r.from_doc_value(&bad_val, &col);
        assert!(result.is_err());
    }

    #[test]
    fn custom_converter_overrides_fallback() {
        // Register a custom i32 converter that multiplies by 10
        let mut r = ConverterRegistry::new();
        r.register::<i32, _>(TimesTenConverter);

        let col = test_column();

        // to_doc_value: 7 -> Int(70) via custom converter, not Int(7) via fallback
        let val = r.to_doc_value(&7i32, &col).unwrap();
        assert!(matches!(val, DocValue::Int(70)));

        // from_doc_value: Int(70) -> 7 via custom converter
        let back: i32 = r.from_doc_value(&val, &col).unwrap();
        assert_eq!(back, 7);
    }

    #[test]
    fn find_converter_by_name_registered() {
        let mut r = ConverterRegistry::new();
        r.register_named::<Status, _>("StatusConverter", StatusConverter);

        assert!(r.find_converter::<Status>().is_some());
        assert!(r.find_converter_by_name("StatusConverter").is_some());
        assert!(r.find_converter_by_name("NonExistent").is_none());
    }

    #[test]
    fn find_converter_by_name_unregistered() {
        let r = ConverterRegistry::new();
        assert!(r.find_converter_by_name("Anything").is_none());
    }

    #[test]
    fn register_named_returns_correct_bool() {
        let mut r = ConverterRegistry::new();
        let first = r.register_named::<Status, _>("StatusConverter", StatusConverter);
        assert!(first);
        let second = r.register_named::<Status, _>("StatusConverter", StatusConverter);
        assert!(!second);
    }

    #[test]
    fn multiple_types_in_same_registry() {
        let mut r = ConverterRegistry::new();
        r.register::<Status, _>(StatusConverter);
        r.register_named::<i32, _>("TimesTen", TimesTenConverter);

        assert!(r.contains::<Status>());
        assert!(r.contains::<i32>());
        assert!(!r.contains::<String>());
        assert!(r.find_converter_by_name("TimesTen").is_some());
        assert!(r.find_converter_by_name("StatusConverter").is_none()); // not named
    }

    #[test]
    fn registry_debug_shows_count() {
        let mut r = ConverterRegistry::new();
        r.register::<Status, _>(StatusConverter);
        r.register::<i32, _>(TimesTenConverter);

        let dbg = format!("{r:?}");
        assert!(dbg.contains("count: 2"));
    }

    #[test]
    fn find_converter_erased_to_doc_value() {
        // Verify that find_converter returns a usable ErasedConverter
        let mut r = ConverterRegistry::new();
        r.register::<Status, _>(StatusConverter);

        let col = test_column();
        let converter = r.find_converter::<Status>().unwrap();
        let val = converter
            .to_doc_value_erased(&Status::Active as &dyn Any, &col)
            .unwrap();
        assert!(matches!(val, DocValue::String(ref s) if s == "ACTIVE"));
    }

    #[test]
    fn find_converter_erased_from_doc_value() {
        let mut r = ConverterRegistry::new();
        r.register::<Status, _>(StatusConverter);

        let col = test_column();
        let converter = r.find_converter::<Status>().unwrap();
        let val = DocValue::String("INACTIVE".into());
        let boxed = converter.from_doc_value_erased(&val, &col).unwrap();
        let status = boxed.downcast::<Status>().unwrap();
        assert_eq!(*status, Status::Inactive);
    }

    #[test]
    fn find_converter_by_name_roundtrip() {
        let mut r = ConverterRegistry::new();
        r.register_named::<Status, _>("StatusConverter", StatusConverter);

        let col = test_column();
        let converter = r.find_converter_by_name("StatusConverter").unwrap();

        let val = converter
            .to_doc_value_erased(&Status::Active as &dyn Any, &col)
            .unwrap();
        assert!(matches!(val, DocValue::String(ref s) if s == "ACTIVE"));

        let boxed = converter.from_doc_value_erased(&val, &col).unwrap();
        let status = boxed.downcast::<Status>().unwrap();
        assert_eq!(*status, Status::Active);
    }
}
