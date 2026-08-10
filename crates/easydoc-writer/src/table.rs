//! 从类型化数据构建的表格。
//!
//! 对应 Java: `com.alibaba.excel.write.metadata.Sheet` / `EasyExcel.write().head(RowClass.class)`

use easydoc_core::{CellData, DocxRow, TableStyle};

/// 从类型化数据构建的表格。
///
/// 对应 Java: `com.alibaba.excel.write.metadata.Sheet` / `EasyExcel.write().head(RowClass.class)`
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<CellData>>,
    style: Option<TableStyle>,
}

impl Table {
    /// 从实现 `DocxRow` 的类型切片创建表格。
    #[must_use]
    pub fn from_data<T: DocxRow>(data: &[T]) -> Self {
        let headers = T::schema()
            .iter()
            .filter(|c| !c.ignored)
            .map(|c| c.name.clone())
            .collect();

        let rows = data.iter().filter_map(|item| item.to_row().ok()).collect();

        Self {
            headers,
            rows,
            style: None,
        }
    }

    /// 设置表格样式。
    #[must_use]
    pub fn header_style(mut self, style: TableStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// 启用斑马条纹。
    #[must_use]
    pub fn banded_rows(mut self, enabled: bool) -> Self {
        self.style.get_or_insert_default().banded_rows = enabled;
        self
    }

    /// 启用自动列宽。
    #[must_use]
    pub fn auto_width(mut self) -> Self {
        self.style.get_or_insert_default().auto_width = true;
        self
    }

    pub(crate) fn headers(&self) -> &[String] {
        &self.headers
    }

    pub(crate) fn rows(&self) -> &[Vec<CellData>] {
        &self.rows
    }

    #[allow(dead_code)]
    pub(crate) fn table_style(&self) -> Option<&TableStyle> {
        self.style.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试辅助结构体，仅用于表格测试。
    #[derive(Debug, Clone)]
    struct TestUser {
        name: String,
        age: u32,
        email: String,
    }

    impl DocxRow for TestUser {
        fn schema() -> &'static [easydoc_core::metadata::TableColumn] {
            static SCHEMA: std::sync::LazyLock<Vec<easydoc_core::metadata::TableColumn>> =
                std::sync::LazyLock::new(|| {
                    vec![
                        easydoc_core::metadata::TableColumn::new("Name", "name", 0),
                        easydoc_core::metadata::TableColumn::new("Age", "age", 1),
                        easydoc_core::metadata::TableColumn::new("Email", "email", 2),
                    ]
                });
            &SCHEMA
        }

        fn from_row(_row: &easydoc_core::RowData) -> easydoc_core::Result<Self> {
            unimplemented!()
        }
        fn from_row_with_converters(
            _row: &easydoc_core::RowData,
            _registry: &easydoc_core::ConverterRegistry,
        ) -> easydoc_core::Result<Self> {
            unimplemented!()
        }
        fn to_row(&self) -> easydoc_core::Result<Vec<easydoc_core::CellData>> {
            Ok(vec![
                easydoc_core::CellData::new(self.name.clone()),
                easydoc_core::CellData::new(i64::from(self.age)),
                easydoc_core::CellData::new(self.email.clone()),
            ])
        }
        fn to_row_with_converters(
            &self,
            _registry: &easydoc_core::ConverterRegistry,
        ) -> easydoc_core::Result<Vec<easydoc_core::CellData>> {
            self.to_row()
        }
    }

    #[test]
    fn table_from_data_empty() {
        let users: Vec<TestUser> = vec![];
        let t = Table::from_data(&users);
        assert!(t.rows().is_empty());
        assert!(!t.headers().is_empty());
    }

    #[test]
    fn table_from_data_with_rows() {
        let users = vec![
            TestUser {
                name: "Alice".into(),
                age: 30,
                email: "a@b.com".into(),
            },
            TestUser {
                name: "Bob".into(),
                age: 25,
                email: "b@c.com".into(),
            },
        ];
        let t = Table::from_data(&users);
        assert_eq!(t.rows().len(), 2);
        assert_eq!(t.headers().len(), 3);
    }

    #[test]
    fn table_builder_methods() {
        let t = Table::from_data::<TestUser>(&[])
            .banded_rows(true)
            .auto_width();
        assert!(t.style.is_some());
        assert!(t.style.as_ref().unwrap().banded_rows);
        assert!(t.style.as_ref().unwrap().auto_width);
    }
}
