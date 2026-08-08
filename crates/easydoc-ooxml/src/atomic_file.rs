use std::fs::File;
use std::io::Write;
use std::path::Path;

use easydoc_core::{DocError, Result};
use tempfile::NamedTempFile;

/// 在目标文件所在目录中完成临时写入和原子替换。
pub struct AtomicFile;

impl AtomicFile {
    /// 调用写入函数生成完整文件，成功后原子替换目标路径。
    ///
    /// 写入或持久化失败时，原目标文件保持不变。
    pub fn write<T>(
        target: impl AsRef<Path>,
        write_fn: impl FnOnce(&mut File) -> Result<T>,
    ) -> Result<T> {
        let target = target.as_ref();
        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        let mut temporary = NamedTempFile::new_in(parent)?;
        let value = write_fn(temporary.as_file_mut())?;
        temporary.as_file_mut().flush()?;
        temporary.as_file_mut().sync_all()?;
        temporary
            .persist(target)
            .map_err(|error| DocError::Io(error.error))?;
        Ok(value)
    }
}
