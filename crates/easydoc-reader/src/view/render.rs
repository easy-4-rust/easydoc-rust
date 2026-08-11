//! 视图渲染入口函数。

use easydoc_core::{DocumentContent, Result};

use super::ViewMode;
use super::{annotated, outline, plain, stats};

/// 按照指定的 [`ViewMode`] 将 [`DocumentContent`] 渲染为字符串。
///
/// # 错误
///
/// 如果渲染失败则返回错误（当前不可失败，但签名允许未来可失败渲染）。
pub fn render_view(content: &DocumentContent, mode: &ViewMode) -> Result<String> {
    match mode {
        ViewMode::Plain => Ok(plain::render(content)),
        ViewMode::Annotated => Ok(annotated::render(content)),
        ViewMode::Outline { max_level } => Ok(outline::render(content, *max_level)),
        ViewMode::Stats => Ok(stats::render(content)),
    }
}
