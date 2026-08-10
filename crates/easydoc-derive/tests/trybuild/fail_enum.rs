use easydoc_derive::DocxRow;
use easydoc_core::DocxRow as _;

#[derive(DocxRow)]
enum NotSupported {
    Variant1,
    Variant2,
}

fn main() {}
