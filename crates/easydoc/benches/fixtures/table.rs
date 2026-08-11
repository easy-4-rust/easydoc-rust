//! Fixture 2：表格（表头 + 5 行 x 3 列）。

use std::sync::LazyLock;

use easydoc::EasyDoc;
use easydoc::Table;
use easydoc::prelude::{DocxRow, HeadingLevel, Paragraph, RowData};
use easydoc_core::metadata::TableColumn;
use easydoc_core::{CellData, ConverterRegistry, Result};

use super::types::FidelityFixture;

/// Fixture 2 使用的 3 列行类型。
#[derive(Debug, Clone)]
struct FixtureRow {
    name: String,
    value: String,
    score: f64,
}

impl DocxRow for FixtureRow {
    fn schema() -> &'static [TableColumn] {
        static SCHEMA: LazyLock<Vec<TableColumn>> = LazyLock::new(|| {
            vec![
                TableColumn::new("Name", "name", 0),
                TableColumn::new("Value", "value", 1),
                TableColumn::new("Score", "score", 2),
            ]
        });
        &SCHEMA
    }

    fn from_row(_row: &RowData) -> Result<Self> {
        unimplemented!("not used in fidelity fixtures")
    }

    fn from_row_with_converters(_row: &RowData, _registry: &ConverterRegistry) -> Result<Self> {
        unimplemented!("not used in fidelity fixtures")
    }

    fn to_row(&self) -> Result<Vec<CellData>> {
        Ok(vec![
            CellData::new(self.name.clone()),
            CellData::new(self.value.clone()),
            CellData::new(self.score),
        ])
    }

    fn to_row_with_converters(&self, _registry: &ConverterRegistry) -> Result<Vec<CellData>> {
        self.to_row()
    }
}

/// 构建表格 fixture。
pub(super) fn build() -> FidelityFixture {
    let rows = vec![
        FixtureRow {
            name: "Alice".into(),
            value: "alpha".into(),
            score: 95.5,
        },
        FixtureRow {
            name: "Bob".into(),
            value: "beta".into(),
            score: 87.0,
        },
        FixtureRow {
            name: "Charlie".into(),
            value: "gamma".into(),
            score: 72.3,
        },
        FixtureRow {
            name: "Diana".into(),
            value: "delta".into(),
            score: 91.8,
        },
        FixtureRow {
            name: "Eve".into(),
            value: "epsilon".into(),
            score: 68.4,
        },
    ];

    let bytes = EasyDoc::document_to_bytes(|doc| {
        doc.title("Table Fixture")
            .add_heading("Data Table", HeadingLevel::H1)
            .add_paragraph(Paragraph::new().add_text("Below is a table with 5 rows of data."))
            .add_table(Table::from_data(&rows))
    })
    .expect("build table fixture");

    let expected = super::types::Fixtures::roundtrip_text(&bytes);

    FidelityFixture {
        name: "table",
        original_size: bytes.len() as u64,
        expected_text: expected,
        docx_bytes: bytes,
    }
}
