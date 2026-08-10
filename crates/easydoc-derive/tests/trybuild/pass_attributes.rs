use easydoc_derive::DocxRow;
use easydoc_core::DocxRow as _;

/// Test struct exercising all supported `#[docx(...)]` field attributes.
#[derive(DocxRow)]
struct FullAttributes {
    #[docx(name = "序号", order = 0, width = "2cm")]
    id: u32,

    #[docx(name = "金额", order = 1, format = "#,##0.00", align = "right")]
    amount: f64,

    #[docx(name = "日期", order = 2, format = "yyyy-mm-dd")]
    date: String,

    #[docx(name = "状态", order = 3, converter = StatusConverter)]
    status: String,

    #[docx(name = "备注", order = 4, wrap = true)]
    note: String,

    #[docx(name = "居中", order = 5, align = "center")]
    centered: String,

    #[docx(name = "两端", order = 6, align = "justify")]
    justified: String,
}

fn main() {
    let schema = FullAttributes::schema();
    assert_eq!(schema.len(), 7);

    // Verify width
    assert_eq!(schema[0].width.as_deref(), Some("2cm"));

    // Verify format
    assert_eq!(schema[1].format.as_deref(), Some("#,##0.00"));
    assert_eq!(schema[2].format.as_deref(), Some("yyyy-mm-dd"));

    // Verify align (justify maps to Both in OOXML)
    assert_eq!(
        schema[1].align,
        Some(easydoc_core::HorizontalAlignment::Right)
    );
    assert_eq!(
        schema[5].align,
        Some(easydoc_core::HorizontalAlignment::Center)
    );
    assert_eq!(
        schema[6].align,
        Some(easydoc_core::HorizontalAlignment::Both)
    );

    // Verify converter
    assert_eq!(schema[3].converter.as_deref(), Some("StatusConverter"));

    // Verify wrap
    assert!(!schema[0].wrap);
    assert!(schema[4].wrap);
}
