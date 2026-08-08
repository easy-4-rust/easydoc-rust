use std::path::{Path, PathBuf};

use easydoc_core::Result;
use easydoc_ooxml::AtomicFile;

use crate::markdown_renderer::MarkdownRenderer;
use crate::{MarkdownOptions, MarkdownResult};

/// 面向使用者的 DOC/DOCX 到 Markdown 流式配置入口。
pub struct MarkdownBuilder {
    source: PathBuf,
    options: MarkdownOptions,
}

impl MarkdownBuilder {
    /// 为源 DOC 或 DOCX 创建转换构建器。
    #[must_use]
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            options: MarkdownOptions::default(),
        }
    }

    /// 设置图片提取目录。
    #[must_use]
    pub fn image_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.options.image_directory = Some(directory.into());
        self
    }

    /// 设置 Markdown 图片地址使用的前缀。
    #[must_use]
    pub fn image_reference_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.options.image_reference_prefix = Some(prefix.into());
        self
    }

    /// 控制是否输出 YAML front matter。
    #[must_use]
    pub const fn include_front_matter(mut self, enabled: bool) -> Self {
        self.options.include_front_matter = enabled;
        self
    }

    /// 使用完整选项替换当前配置。
    #[must_use]
    pub fn options(mut self, options: MarkdownOptions) -> Self {
        self.options = options;
        self
    }

    /// 执行转换并返回正文、资源和警告。
    ///
    /// # Errors
    ///
    /// 源文档无法解析或资源无法写入时返回错误。
    pub fn do_convert(self) -> Result<MarkdownResult> {
        let document = easydoc_reader::read_document(&self.source)?;
        MarkdownRenderer::new(self.options).render(&document)
    }

    /// 执行转换并将 Markdown 原子写入目标文件。
    ///
    /// # Errors
    ///
    /// 转换或输出失败时返回错误，原目标文件保持不变。
    pub fn write_to(self, output: impl AsRef<Path>) -> Result<MarkdownResult> {
        let result = self.do_convert()?;
        AtomicFile::write(output, |file| {
            use std::io::Write;
            file.write_all(result.markdown.as_bytes())?;
            Ok(())
        })?;
        Ok(result)
    }
}
