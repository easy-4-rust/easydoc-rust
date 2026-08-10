/// 从文档中提取的图片及其语义信息。
///
/// 对应 OOXML `<w:drawing>` 中的 `<a:blip>` 图片引用。
/// 无直接 Java 对应，是 easydoc-rust 自创的语义模型。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentImage {
    /// 替代文本。
    pub alt_text: Option<String>,
    /// 图片原始字节；解析器无法提取时为 `None`。
    pub data: Option<Vec<u8>>,
    /// 不带点号的文件扩展名。
    pub extension: Option<String>,
}
