//! 文档内容的核心数据类型。
//!
//! 这些类型构成 Rust 类型值与文档单元格/段落内容之间的通用中间表示，
//! 对标 easyexcel-rust 的 `CellValue`。
//!
//! 对应 Java: `com.alibaba.excel.metadata.data` (`ReadCellData` / `WriteCellData`)

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

/// 通用文档值 -- Rust 类型与 DOCX 内容之间的桥梁。
///
/// 每个 `DocConverter` 都从此枚举读取或写入，正如 `CellValue` 在 Rust 类型与
/// Excel 单元格数据之间充当中介。
///
/// 对应 Java: `com.alibaba.excel.metadata.data.CellValue` / `ReadCellData` / `WriteCellData`
#[derive(Debug, Clone)]
pub enum DocValue {
    /// 纯文本字符串。
    String(String),
    /// 布尔值。
    Bool(bool),
    /// 64 位有符号整数。
    Int(i64),
    /// 64 位浮点数。
    Float(f64),
    /// UTC 日期时间。
    DateTime(DateTime<Utc>),
    /// 仅日期（无时间分量）。
    Date(NaiveDate),
    /// 无时区的日期时间。
    NaiveDateTime(NaiveDateTime),
    /// 空值 / null。
    Empty,
    /// 富文本（格式化文本片段）。
    RichText(Vec<RichRun>),
    /// 图片数据（原始字节 + 元数据）。
    Image(ImageData),
}

/// 富文本单元格中的单个格式化文本片段。
///
/// 对应 Java: `com.alibaba.excel.metadata.data.RichTextString` 的运行片段
#[derive(Debug, Clone)]
pub struct RichRun {
    /// 文本内容。
    pub text: String,
    /// 是否加粗。
    pub bold: bool,
    /// 是否斜体。
    pub italic: bool,
    /// 字号（半磅为单位，如 24 = 12pt）。
    pub size: Option<u32>,
    /// RGB 颜色（如 `0xFF0000` 表示红色）。
    pub color: Option<u32>,
    /// 字体族名称。
    pub font: Option<String>,
}

/// 图片载荷及元数据。
#[derive(Debug, Clone)]
pub struct ImageData {
    /// 原始图片字节。
    pub bytes: Vec<u8>,
    /// 文件扩展名 / MIME 提示（如 "png"、"jpg"）。
    pub extension: String,
    /// 期望宽度（EMU 或像素）。
    pub width: Option<u32>,
    /// 期望高度（EMU 或像素）。
    pub height: Option<u32>,
    /// 替代文本描述。
    pub alt_text: Option<String>,
}

/// 单个表格单元格的数据。
///
/// 携带已转换的值及可选的格式覆盖。
///
/// 对应 Java: `com.alibaba.excel.metadata.data.WriteCellData` / `ReadCellData`
#[derive(Debug, Clone)]
pub struct CellData {
    /// 单元格的已转换值。
    pub value: DocValue,
    /// 可选的水平对齐覆盖。
    pub alignment: Option<HorizontalAlignment>,
    /// 合并单元格的跨列数（1 = 正常）。
    pub col_span: u32,
    /// 合并单元格的跨行数（1 = 正常）。
    pub row_span: u32,
}

impl CellData {
    /// 从任何可转换为 [`DocValue`] 的值创建新单元格。
    pub fn new(value: impl Into<DocValue>) -> Self {
        Self {
            value: value.into(),
            alignment: None,
            col_span: 1,
            row_span: 1,
        }
    }

    /// 设置此单元格的水平对齐方式。
    #[must_use]
    pub fn alignment(mut self, alignment: HorizontalAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
}

/// 单个表格行的数据。
#[derive(Debug, Clone)]
pub struct RowData {
    /// 行内单元格，按列顺序排列。
    pub cells: Vec<CellData>,
    /// 行高提示（twips 为单位，1/20 磅）。
    pub height: Option<u32>,
}

impl RowData {
    /// 从单元格列表创建行。
    #[must_use]
    pub fn new(cells: Vec<CellData>) -> Self {
        Self {
            cells,
            height: None,
        }
    }
}

/// 读取过程中从文档提取的完整表格数据。
#[derive(Debug, Clone)]
pub struct TableData {
    /// 表头行（如有）。
    pub headers: Option<Vec<String>>,
    /// 数据行。
    pub rows: Vec<Vec<String>>,
}

/// 段落对齐选项。
///
/// `Both` 对应 OOXML `<w:jc w:val="both"/>`，在大多数文档处理器中渲染为两端对齐。
///
/// 对应 Java: `com.alibaba.excel.write.metadata.style.WriteCellStyle#getHorizontalAlignment`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlignment {
    /// 左对齐。
    Left,
    /// 居中对齐。
    Center,
    /// 右对齐。
    Right,
    /// 两端对齐（OOXML `both`）。
    Both,
}

/// 结构化文档的标题级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadingLevel {
    /// 标题 1（最大）。
    H1,
    /// 标题 2。
    H2,
    /// 标题 3。
    H3,
    /// 标题 4。
    H4,
    /// 标题 5。
    H5,
    /// 标题 6（最小）。
    H6,
}

/// 读取错误发生时的动作。
///
/// 由 [`DocReadListener::on_error`](crate::traits::DocReadListener::on_error) 返回。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    /// 跳过错误并继续读取。
    Continue,
    /// 跳过当前构造（段落 / 表格行）并继续。
    Skip,
    /// 立即停止读取。
    Stop,
}

// ---------------------------------------------------------------------------
// From impls — convenience conversions into DocValue
// ---------------------------------------------------------------------------

impl From<String> for DocValue {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for DocValue {
    fn from(v: &str) -> Self {
        Self::String(v.to_owned())
    }
}

impl From<bool> for DocValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i64> for DocValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<i32> for DocValue {
    fn from(v: i32) -> Self {
        Self::Int(i64::from(v))
    }
}

impl From<u32> for DocValue {
    fn from(v: u32) -> Self {
        Self::Int(i64::from(v))
    }
}

impl From<f64> for DocValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<DateTime<Utc>> for DocValue {
    fn from(v: DateTime<Utc>) -> Self {
        Self::DateTime(v)
    }
}

impl From<NaiveDate> for DocValue {
    fn from(v: NaiveDate) -> Self {
        Self::Date(v)
    }
}

impl From<NaiveDateTime> for DocValue {
    fn from(v: NaiveDateTime) -> Self {
        Self::NaiveDateTime(v)
    }
}

impl<T: Into<DocValue>> From<Option<T>> for DocValue {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(inner) => inner.into(),
            None => Self::Empty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn doc_value_from_string() {
        let v: DocValue = "hello".into();
        assert!(matches!(v, DocValue::String(s) if s == "hello"));
    }

    #[test]
    fn doc_value_from_owned_string() {
        let v: DocValue = String::from("world").into();
        assert!(matches!(v, DocValue::String(s) if s == "world"));
    }

    #[test]
    fn doc_value_from_bool() {
        let v: DocValue = true.into();
        assert!(matches!(v, DocValue::Bool(true)));
    }

    #[test]
    fn doc_value_from_i64() {
        let v: DocValue = 42i64.into();
        assert!(matches!(v, DocValue::Int(42)));
    }

    #[test]
    fn doc_value_from_i32() {
        let v: DocValue = 7i32.into();
        assert!(matches!(v, DocValue::Int(7)));
    }

    #[test]
    fn doc_value_from_u32() {
        let v: DocValue = 99u32.into();
        assert!(matches!(v, DocValue::Int(99)));
    }

    #[test]
    fn doc_value_from_f64() {
        let v: DocValue = std::f64::consts::PI.into();
        assert!(matches!(v, DocValue::Float(f) if (f - std::f64::consts::PI).abs() < f64::EPSILON));
    }

    #[test]
    fn doc_value_from_datetime_utc() {
        let dt = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let v: DocValue = dt.into();
        assert!(matches!(v, DocValue::DateTime(_)));
    }

    #[test]
    fn doc_value_from_naive_date() {
        let d = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let v: DocValue = d.into();
        assert!(matches!(v, DocValue::Date(_)));
    }

    #[test]
    fn doc_value_from_naive_datetime() {
        let ndt = NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let v: DocValue = ndt.into();
        assert!(matches!(v, DocValue::NaiveDateTime(_)));
    }

    #[test]
    fn doc_value_from_option_some() {
        let v: DocValue = Some("test").into();
        assert!(matches!(v, DocValue::String(s) if s == "test"));
    }

    #[test]
    fn doc_value_from_option_none() {
        let v: DocValue = Option::<String>::None.into();
        assert!(matches!(v, DocValue::Empty));
    }

    #[test]
    fn cell_data_new_and_alignment() {
        let cell = CellData::new("hello").alignment(HorizontalAlignment::Center);
        assert!(matches!(cell.value, DocValue::String(_)));
        assert_eq!(cell.alignment, Some(HorizontalAlignment::Center));
        assert_eq!(cell.col_span, 1);
        assert_eq!(cell.row_span, 1);
    }

    #[test]
    fn row_data_new() {
        let cells = vec![CellData::new("a"), CellData::new("b")];
        let row = RowData::new(cells);
        assert_eq!(row.cells.len(), 2);
        assert!(row.height.is_none());
    }

    #[test]
    fn horizontal_alignment_debug_and_eq() {
        assert_eq!(HorizontalAlignment::Left, HorizontalAlignment::Left);
        assert_ne!(HorizontalAlignment::Left, HorizontalAlignment::Right);
        let _ = format!("{:?}", HorizontalAlignment::Both);
    }

    #[test]
    fn heading_level_variants() {
        assert_ne!(HeadingLevel::H1, HeadingLevel::H2);
        assert_ne!(HeadingLevel::H3, HeadingLevel::H6);
    }

    #[test]
    fn error_action_variants() {
        assert_eq!(ErrorAction::Continue, ErrorAction::Continue);
        assert_ne!(ErrorAction::Stop, ErrorAction::Skip);
    }

    #[test]
    fn rich_run_debug() {
        let run = RichRun {
            text: "hi".into(),
            bold: true,
            italic: false,
            size: Some(24),
            color: Some(0xFF0000),
            font: Some("Arial".into()),
        };
        let dbg = format!("{run:?}");
        assert!(dbg.contains("hi"));
    }

    #[test]
    fn image_data_debug() {
        let img = ImageData {
            bytes: vec![0x89, 0x50],
            extension: "png".into(),
            width: Some(100),
            height: Some(200),
            alt_text: Some("test".into()),
        };
        assert!(format!("{img:?}").contains("png"));
    }
}
