//! easydoc-rust 统一错误类型。
//!
//! 对标 easyexcel-rust 的单枚举错误模式。
//!
//! 对应 Java: 各类 checked/unchecked exception 合并为单一 Rust 错误类型。

use thiserror::Error;

/// 库统一错误枚举。
///
/// Java `EasyExcel` 将错误分散在七个 `RuntimeException` 子类中；
/// 本枚举将它们合并为一个惯用的 Rust 类型。
///
/// 对应 Java: `com.alibaba.excel.exception.ExcelAnalysisException`、
/// `ExcelGenerateException`、`ExcelDataConvertException` 等
#[derive(Debug, Error)]
pub enum DocError {
    /// I/O 错误，包装 `std::io::Error`。
    ///
    /// 对应 Java: `IOException`
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// ZIP 包错误（来自 docx-rs 打包）。
    ///
    /// 对应 Java: `ExcelAnalysisException` 中的 ZIP 相关错误
    #[error("ZIP error: {0}")]
    Zip(String),

    /// 无效或不支持的文档格式。
    ///
    /// 对应 Java: `ExcelAnalysisException("The supplied file was not supported")`
    #[error("Format error: {0}")]
    Format(String),

    /// 模板占位符无法解析或处理。
    ///
    /// 对应 Java: `ExcelAnalysisException` 中的模板相关错误（easyexcel-template）
    #[error("Template error at placeholder '{placeholder}': {message}")]
    Template {
        /// 导致错误的占位符标记。
        placeholder: String,
        /// 人类可读的描述。
        message: String,
    },

    /// 单元格或字段值无法与目标类型互转。
    ///
    /// 对应 Java: `com.alibaba.excel.exception.ExcelDataConvertException`
    #[error("Conversion error: field '{field}', value '{value}': {message}")]
    Conversion {
        /// 字段或列名。
        field: String,
        /// 转换失败的值。
        value: String,
        /// 人类可读的描述。
        message: String,
    },

    /// 请求的操作不被当前格式或配置支持。
    ///
    /// 对应 Java: `UnsupportedOperationException`
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// 通用文档级错误。
    ///
    /// 对应 Java: `ExcelAnalysisException` / `ExcelGenerateException`
    #[error("Document error: {0}")]
    Document(String),
}

/// easydoc-rust 标准 `Result` 类型别名。
pub type Result<T> = std::result::Result<T, DocError>;

impl From<zip::result::ZipError> for DocError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Zip(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err = DocError::Io(io_err);
        assert!(format!("{err}").contains("I/O error"));
    }

    #[test]
    fn error_display_zip() {
        let err = DocError::Zip("corrupt".into());
        assert!(format!("{err}").contains("ZIP error"));
    }

    #[test]
    fn error_display_format() {
        let err = DocError::Format("bad xml".into());
        assert!(format!("{err}").contains("Format error"));
    }

    #[test]
    fn error_display_template() {
        let err = DocError::Template {
            placeholder: "name".into(),
            message: "not found".into(),
        };
        let s = format!("{err}");
        assert!(s.contains("name"));
        assert!(s.contains("not found"));
    }

    #[test]
    fn error_display_conversion() {
        let err = DocError::Conversion {
            field: "age".into(),
            value: "abc".into(),
            message: "not a number".into(),
        };
        let s = format!("{err}");
        assert!(s.contains("age"));
        assert!(s.contains("abc"));
    }

    #[test]
    fn error_display_unsupported() {
        let err = DocError::Unsupported("macro".into());
        assert!(format!("{err}").contains("Unsupported operation"));
    }

    #[test]
    fn error_display_document() {
        let err = DocError::Document("corrupted".into());
        assert!(format!("{err}").contains("Document error"));
    }

    #[test]
    fn error_from_zip_error() {
        let zip_err = zip::result::ZipError::FileNotFound;
        let err: DocError = zip_err.into();
        assert!(matches!(err, DocError::Zip(_)));
    }

    #[test]
    fn error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: DocError = io_err.into();
        assert!(matches!(err, DocError::Io(_)));
    }

    #[test]
    fn error_debug() {
        let err = DocError::Document("test".into());
        let dbg = format!("{err:?}");
        assert!(dbg.contains("Document"));
    }
}
