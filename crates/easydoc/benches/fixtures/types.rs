//! 保真度测试的公共类型定义。

use easydoc::EasyDoc;
use easydoc::prelude::ViewMode;

/// 单个保真度测试用例：已知 DOCX 字节加上预期纯文本输出。
pub(crate) struct FidelityFixture {
    /// 可读名称（用作 Criterion 参数）。
    pub name: &'static str,
    /// DOCX 文件的字节。
    pub docx_bytes: Vec<u8>,
    /// DOCX 的字节大小。
    pub original_size: u64,
    /// `view_as(Plain)` 的预期纯文本输出。
    pub expected_text: String,
}

impl FidelityFixture {
    /// 将 DOCX 字节写入带 `.docx` 后缀的 [`tempfile::NamedTempFile`] 并返回。
    ///
    /// 返回的句柄被丢弃时文件自动删除。
    pub fn write_to_temp(&self) -> tempfile::NamedTempFile {
        let file = tempfile::Builder::new()
            .suffix(".docx")
            .tempfile()
            .expect("create temp file for fixture");
        std::fs::write(file.path(), &self.docx_bytes).expect("write fixture docx to temp");
        file
    }
}

/// 五个保真度 fixture 的集合，首次访问时惰性生成。
pub(crate) struct Fixtures {
    /// Fixture 1：简单文本（1 个标题 + 3 个段落）。
    pub simple: FidelityFixture,
    /// Fixture 2：表格（表头 + 5 行 x 3 列）。
    pub table: FidelityFixture,
    /// Fixture 3：列表（无序 + 有序 + 嵌套）。
    pub list: FidelityFixture,
    /// Fixture 4：富文本（粗体、斜体、下划线、颜色、字号）。
    pub rich: FidelityFixture,
    /// Fixture 5：嵌入 1x1 红色 PNG 图片。
    pub image: FidelityFixture,
}

impl Fixtures {
    /// 返回惰性初始化的 fixture 集合的引用。
    pub fn load() -> &'static Self {
        use std::sync::LazyLock;
        static INSTANCE: LazyLock<Fixtures> = LazyLock::new(Fixtures::generate);
        &INSTANCE
    }

    /// 按固定顺序返回所有五个 fixture 的引用。
    pub fn all(&self) -> Vec<&FidelityFixture> {
        vec![
            &self.simple,
            &self.table,
            &self.list,
            &self.rich,
            &self.image,
        ]
    }

    fn generate() -> Self {
        let simple = super::simple::build();
        let table = super::table::build();
        let list = super::list::build();
        let rich = super::rich::build();
        let image = super::image::build();
        Self {
            simple,
            table,
            list,
            rich,
            image,
        }
    }

    /// 将 DOCX 字节写入临时文件，用 `view_as(Plain)` 读回，返回渲染文本。
    /// 这是用于保真度比较的"预期输出"。
    pub(super) fn roundtrip_text(docx_bytes: &[u8]) -> String {
        let tmp = tempfile::Builder::new()
            .suffix(".docx")
            .tempfile()
            .expect("temp file for roundtrip");
        std::fs::write(tmp.path(), docx_bytes).expect("write roundtrip docx");
        EasyDoc::view_as(tmp.path(), &ViewMode::Plain).expect("view_as for roundtrip")
    }
}
