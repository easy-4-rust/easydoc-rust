use crate::types::HorizontalAlignment;

/// 表格中单列的描述符。
///
/// 由 `#[derive(DocxRow)]` 从字段注解生成。
///
/// 对应 Java: `com.alibaba.excel.metadata.ExcelColumn` / `ExcelProperty` 注解属性
#[derive(Debug, Clone)]
pub struct TableColumn {
    /// Column header text (displayed in the table header row).
    pub name: String,
    /// Rust field name (used for reverse lookup).
    pub field_name: String,
    /// Zero-based column index in the table.
    pub index: usize,
    /// Order for column sorting (lower = leftmost).
    pub order: u32,
    /// Column width as a CSS-like string (e.g. `"2cm"`, `"80px"`, `"auto"`),
    /// or `None` for auto-width.
    pub width: Option<String>,
    /// Number or date format pattern (e.g. `"#,##0.00"`, `"yyyy-mm-dd"`),
    /// if applicable.
    pub format: Option<String>,
    /// Horizontal alignment override for cells in this column.
    pub align: Option<HorizontalAlignment>,
    /// Custom converter type path (e.g. `"StatusConverter"`),
    /// used by `from_row_with_converters` / `to_row_with_converters`.
    pub converter: Option<String>,
    /// Whether text in this column should wrap.
    pub wrap: bool,
    /// Whether the field should be ignored during read/write.
    pub ignored: bool,
}

impl TableColumn {
    /// 创建新的列描述符。
    #[must_use]
    pub fn new(name: impl Into<String>, field_name: impl Into<String>, index: usize) -> Self {
        Self {
            name: name.into(),
            field_name: field_name.into(),
            index,
            order: index as u32,
            width: None,
            format: None,
            align: None,
            converter: None,
            wrap: false,
            ignored: false,
        }
    }

    /// 设置显示名称。
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// 设置列排序顺序。
    #[must_use]
    pub fn order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    /// 设置列宽（CSS 风格字符串，如 `"2cm"`、`"80px"`）。
    #[must_use]
    pub fn width(mut self, width: impl Into<String>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// 设置数字/日期列的格式模式。
    #[must_use]
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// 设置此列的水平对齐方式。
    #[must_use]
    pub fn align(mut self, align: HorizontalAlignment) -> Self {
        self.align = Some(align);
        self
    }

    /// 设置此列的自定义转换器类型名。
    #[must_use]
    pub fn converter(mut self, converter: impl Into<String>) -> Self {
        self.converter = Some(converter.into());
        self
    }

    /// 启用此列的文本换行。
    #[must_use]
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    /// 标记此列在读写时被忽略。
    ///
    /// 对应 Java: `ExcelProperty` 的 `@ExcelIgnore` 注解
    #[must_use]
    pub fn ignore(mut self) -> Self {
        self.ignored = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HorizontalAlignment;

    #[test]
    fn column_new() {
        let c = TableColumn::new("Name", "name", 0);
        assert_eq!(c.name, "Name");
        assert_eq!(c.field_name, "name");
        assert_eq!(c.index, 0);
        assert_eq!(c.order, 0);
        assert!(c.width.is_none());
        assert!(c.format.is_none());
        assert!(c.align.is_none());
        assert!(c.converter.is_none());
        assert!(!c.wrap);
        assert!(!c.ignored);
    }

    #[test]
    fn column_builder_chain() {
        let c = TableColumn::new("Age", "age", 1)
            .name("User Age")
            .order(5)
            .width("2cm")
            .format("%Y-%m-%d")
            .align(HorizontalAlignment::Center)
            .converter("AgeConverter")
            .wrap()
            .ignore();
        assert_eq!(c.name, "User Age");
        assert_eq!(c.order, 5);
        assert_eq!(c.width.as_deref(), Some("2cm"));
        assert_eq!(c.format.as_deref(), Some("%Y-%m-%d"));
        assert_eq!(c.align, Some(HorizontalAlignment::Center));
        assert_eq!(c.converter.as_deref(), Some("AgeConverter"));
        assert!(c.wrap);
        assert!(c.ignored);
    }

    #[test]
    fn column_builder_width_string_variants() {
        let c = TableColumn::new("X", "x", 0).width("80px");
        assert_eq!(c.width.as_deref(), Some("80px"));

        let c = TableColumn::new("X", "x", 0).width("auto");
        assert_eq!(c.width.as_deref(), Some("auto"));
    }
}
