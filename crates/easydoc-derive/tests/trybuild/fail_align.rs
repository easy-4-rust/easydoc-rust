use easydoc_derive::DocxRow;

#[derive(DocxRow)]
struct BadAlign {
    #[docx(name = "X", align = "top")]
    x: String,
}

fn main() {}
