use std::path::PathBuf;

/// DOC/DOCX 到 Markdown 的转换选项。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MarkdownOptions {
    /// 图片提取目录；不设置时图片以文字占位并产生警告。
    pub image_directory: Option<PathBuf>,
    /// Markdown 中的图片引用前缀；默认使用提取目录名。
    pub image_reference_prefix: Option<String>,
    /// 是否把标题、作者等元数据输出为 YAML front matter。
    pub include_front_matter: bool,
}
