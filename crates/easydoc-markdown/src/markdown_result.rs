use crate::{ConversionWarning, ExtractedAsset};

/// Markdown 转换正文、资源和降级信息的完整结果。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MarkdownResult {
    /// 生成的 Markdown 文本。
    pub markdown: String,
    /// 已提取资源。
    pub assets: Vec<ExtractedAsset>,
    /// 可恢复的语义降级。
    pub warnings: Vec<ConversionWarning>,
}
