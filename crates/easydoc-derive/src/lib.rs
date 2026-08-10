//! 类型化 DOCX 表格行映射的 derive 宏。
//!
//! 提供 `#[derive(DocxRow)]`，生成：
//! - `schema()` -- 静态列元数据
//! - `from_row()` / `from_row_with_converters()` -- 行反序列化
//! - `to_row()` / `to_row_with_converters()` -- 行序列化
//!
//! 对应 Java: `com.alibaba.excel.annotation.ExcelProperty` + 反射机制

use proc_macro::TokenStream;

mod implementation;

/// 派生静态表格列元数据和双向行转换。
///
/// 对应 Java: `@ExcelProperty` 注解 + `EasyExcel` 反射读写机制
///
/// # 结构体属性
///
/// - `#[docx(table_width = Auto)]` -- 自动适配表格宽度
/// - `#[docx(banded_rows = true)]` -- 启用斑马条纹
///
/// # 字段属性
///
/// | 属性 | 类型 | 描述 |
/// |------|------|------|
/// | `name` | string | 列标题文本 |
/// | `index` | integer | 从零开始的列索引 |
/// | `order` | integer | 列排序顺序（值越小越靠左） |
/// | `width` | string | 列宽（`"2cm"`、`"80px"`、`"auto"`） |
/// | `format` | string | 数字/日期格式（`"#,##0.00"`、`"yyyy-mm-dd"`） |
/// | `align` | string | 水平对齐（`"left"`、`"center"`、`"right"`、`"justify"`） |
/// | `converter` | type path | 自定义转换器类型（如 `StatusConverter`） |
/// | `wrap` | bool | 启用文本换行 |
/// | `ignore` | flag | 读写时跳过此字段 |
///
/// # 示例
///
/// ```ignore
/// use easydoc_derive::DocxRow;
///
/// #[derive(DocxRow)]
/// #[docx(banded_rows = true)]
/// struct Report {
///     #[docx(name = "序号", order = 0, width = "2cm")]
///     id: u32,
///
///     #[docx(name = "金额", order = 1, format = "#,##0.00", align = "right")]
///     amount: f64,
///
///     #[docx(name = "日期", order = 2, format = "yyyy-mm-dd")]
///     date: String,
///
///     #[docx(name = "状态", order = 3, converter = StatusConverter)]
///     status: String,
///
///     #[docx(name = "备注", order = 4, wrap = true)]
///     note: Option<String>,
///
///     #[docx(ignore)]
///     internal_id: String,
/// }
/// ```
#[proc_macro_derive(DocxRow, attributes(docx))]
pub fn derive_docx_row(input: TokenStream) -> TokenStream {
    implementation::expand_docx_row_tokens(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
