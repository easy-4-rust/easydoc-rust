//! CollectListener — collects all parsed items into a `Vec<T>`.

use easydoc_core::{DocReadContext, DocReadListener, Result};

/// A listener that collects all items into a `Vec<T>` for synchronous reads.
pub struct CollectListener<T>(pub Vec<T>);

impl<T> DocReadListener<T> for CollectListener<T> {
    fn invoke(&mut self, data: T, _context: &DocReadContext) -> Result<()> {
        self.0.push(data);
        Ok(())
    }

    fn on_complete(&mut self, _context: &DocReadContext) {}
}
