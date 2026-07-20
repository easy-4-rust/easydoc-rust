//! Fill configuration for template expansion.

/// Fill direction for collection expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillDirection {
    /// Expand collection items vertically (rows in a table).
    Vertical,
    /// Expand collection items horizontally.
    Horizontal,
}

/// Configuration controlling template fill behaviour.
///
/// Analogous to `FillConfig` in `easyexcel-template`.
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
