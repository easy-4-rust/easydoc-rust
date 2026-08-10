//! `CollectListener` -- 将所有已解析项收集到 `Vec<T>` 中。
//!
//! 对应 Java: `EasyExcel` 内部的 `ReadListener` 默认收集行为

use easydoc_core::{DocReadContext, DocReadListener, Result};

/// 将所有项收集到 `Vec<T>` 中的监听器，用于同步读取。
///
/// 对应 Java: `com.alibaba.excel.read.listener.ReadListener` 的默认收集行为
pub struct CollectListener<T>(pub Vec<T>);

impl<T> DocReadListener<T> for CollectListener<T> {
    fn invoke(&mut self, data: T, _context: &DocReadContext) -> Result<()> {
        self.0.push(data);
        Ok(())
    }

    fn on_complete(&mut self, _context: &DocReadContext) {}
}
