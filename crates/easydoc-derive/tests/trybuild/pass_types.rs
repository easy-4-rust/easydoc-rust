use easydoc_derive::DocxRow;
use easydoc_core::DocxRow as _;

#[derive(DocxRow)]
struct Mixed {
    #[docx(name = "Text", order = 0)]
    text: String,
    #[docx(name = "Count", order = 1)]
    count: i32,
    #[docx(name = "Active", order = 2)]
    active: bool,
    #[docx(name = "Score", order = 3)]
    score: f64,
}

fn main() {
    let schema = Mixed::schema();
    assert_eq!(schema.len(), 4);
}
