//! DOC/DOCX 到 Markdown 的语义转换模块。

#![deny(unsafe_code)]

mod conversion_warning;
mod extracted_asset;
mod markdown_builder;
mod markdown_options;
mod markdown_renderer;
mod markdown_result;

pub use conversion_warning::ConversionWarning;
pub use extracted_asset::ExtractedAsset;
pub use markdown_builder::MarkdownBuilder;
pub use markdown_options::MarkdownOptions;
pub use markdown_result::MarkdownResult;

use easydoc_core::{DocumentContent, Result};

/// 把已解析的 easydoc 语义文档渲染为 Markdown。
///
/// 该入口适合自定义解析器、测试和内存转换，无需再次读取 DOC/DOCX。
///
/// # Errors
///
/// 图片等伴随资源无法写入时返回错误。
pub fn render_document(
    document: &DocumentContent,
    options: MarkdownOptions,
) -> Result<MarkdownResult> {
    markdown_renderer::MarkdownRenderer::new(options).render(document)
}
