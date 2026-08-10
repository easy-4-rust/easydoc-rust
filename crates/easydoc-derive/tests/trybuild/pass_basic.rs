use easydoc_derive::DocxRow;
use easydoc_core::DocxRow as _;

#[derive(DocxRow)]
#[docx(banded_rows = true, table_width = Auto)]
struct User {
    #[docx(name = "Name", width = "30%", order = 0)]
    name: String,
    #[docx(name = "Age", width = "15%", order = 1)]
    age: u32,
    #[docx(name = "Email", width = "55%", order = 2)]
    email: String,
    #[docx(ignore)]
    #[allow(dead_code)]
    internal_id: String,
}

fn main() {
    let schema = User::schema();
    assert!(!schema.is_empty());
}
