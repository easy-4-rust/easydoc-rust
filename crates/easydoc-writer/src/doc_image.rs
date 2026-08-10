//! 向文档插入图片的配置。

use std::path::PathBuf;

/// 向文档插入图片的配置。
pub struct DocImage {
    /// Path to the image file.
    pub path: PathBuf,
    /// Desired width in pixels (applied via `Pic::new_with_dimensions`).
    pub(crate) width: Option<u32>,
    /// Desired height in pixels.
    pub(crate) height: Option<u32>,
    alt_text: Option<String>,
}

impl DocImage {
    /// 创建图片配置。
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            width: None,
            height: None,
            alt_text: None,
        }
    }

    /// 设置图片宽度（像素）。
    #[must_use]
    pub fn width(mut self, w: u32) -> Self {
        self.width = Some(w);
        self
    }

    /// 设置图片高度（像素）。
    #[must_use]
    pub fn height(mut self, h: u32) -> Self {
        self.height = Some(h);
        self
    }

    /// 设置替代文本。
    #[must_use]
    pub fn alt_text(mut self, text: impl Into<String>) -> Self {
        self.alt_text = Some(text.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_image_builder() {
        let img = DocImage::new("/tmp/test.png")
            .width(100)
            .height(200)
            .alt_text("test image");
        assert_eq!(img.path, std::path::PathBuf::from("/tmp/test.png"));
        assert_eq!(img.width, Some(100));
        assert_eq!(img.height, Some(200));
    }
}
