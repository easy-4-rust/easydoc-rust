//! 模板展开的填充配置。
//!
//! 对应 Java: `com.alibaba.excel.write.builder.ExcelWriterSheetBuilder` 中的填充配置

/// 集合展开的填充方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillDirection {
    /// 垂直展开集合项（表格中的行）。
    Vertical,
    /// 水平展开集合项。
    Horizontal,
}

/// 控制模板填充行为的配置。
///
/// 对应 Java: `com.alibaba.excel.write.builder.ExcelWriterSheetBuilder` 中的填充配置
#[derive(Debug, Clone)]
pub struct FillConfig {
    /// Direction for collection expansion.
    pub direction: FillDirection,
    /// Whether to insert a new row for each collection item.
    pub force_new_row: bool,
    /// Whether to inherit the placeholder cell's style for filled cells.
    pub auto_style: bool,
}

impl Default for FillConfig {
    fn default() -> Self {
        Self {
            direction: FillDirection::Vertical,
            force_new_row: true,
            auto_style: true,
        }
    }
}

impl FillConfig {
    /// Creates a new fill configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the fill direction.
    #[must_use]
    pub fn direction(mut self, direction: FillDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Sets whether to force new rows for collection items.
    #[must_use]
    pub fn force_new_row(mut self, force: bool) -> Self {
        self.force_new_row = force;
        self
    }

    /// Sets whether to auto-style inherited cells.
    #[must_use]
    pub fn auto_style(mut self, auto: bool) -> Self {
        self.auto_style = auto;
        self
    }
}
