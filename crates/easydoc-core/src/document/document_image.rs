/// 从文档中提取的图片及其语义信息。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentImage {
    /// 替代文本。
    pub alt_text: Option<String>,
    /// 图片原始字节；解析器无法提取时为 `None`。
    pub data: Option<Vec<u8>>,
    /// 不带点号的文件扩展名。
    pub extension: Option<String>,
}
