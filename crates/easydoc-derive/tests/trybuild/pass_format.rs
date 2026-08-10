use easydoc_derive::DocxRow;
use easydoc_core::DocxRow as _;

#[derive(DocxRow)]
struct Record {
    #[docx(name = "Date", index = 0, format = "%Y-%m-%d")]
    date: String,
    #[docx(name = "Value", index = 1)]
    value: f64,
}

fn main() {
    let schema = Record::schema();
    assert_eq!(schema.len(), 2);
}
