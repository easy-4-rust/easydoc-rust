use std::path::PathBuf;

/// 从源文档提取并写入磁盘的资源。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractedAsset {
    /// 资源输出路径。
    pub path: PathBuf,
    /// Markdown 中使用的引用地址。
    pub reference: String,
}
