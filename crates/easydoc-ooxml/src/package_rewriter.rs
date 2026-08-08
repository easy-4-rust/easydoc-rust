use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;

use easydoc_core::{DocError, Result};
use zip::write::SimpleFileOptions;

use crate::{AtomicFile, PackageLimits};

/// 在保留未修改二进制条目的前提下重写 OOXML ZIP 包。
pub struct PackageRewriter {
    limits: PackageLimits,
}

impl Default for PackageRewriter {
    fn default() -> Self {
        Self::new(PackageLimits::default())
    }
}

impl PackageRewriter {
    /// 使用指定资源限制创建包重写器。
    #[must_use]
    pub const fn new(limits: PackageLimits) -> Self {
        Self { limits }
    }

    /// 重写包；转换函数返回 `Some` 时替换条目，返回 `None` 时逐字节保留原内容。
    pub fn rewrite(
        &self,
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        mut transform: impl FnMut(&str, &[u8]) -> Result<Option<Vec<u8>>>,
    ) -> Result<()> {
        let input_file = File::open(input)?;
        let mut archive = zip::ZipArchive::new(input_file)?;
        self.validate_archive(&mut archive)?;

        AtomicFile::write(output, |file| {
            let mut writer = zip::ZipWriter::new(file);
            for index in 0..archive.len() {
                let mut entry = archive.by_index(index)?;
                let name = entry.name().to_owned();
                let options = entry_options(&entry);
                if entry.is_dir() {
                    writer.add_directory(name, options)?;
                    continue;
                }

                let capacity = usize::try_from(entry.size()).unwrap_or(0);
                let mut content = Vec::with_capacity(capacity);
                entry.read_to_end(&mut content)?;
                let replacement = transform(&name, &content)?;
                writer.start_file(name, options)?;
                writer.write_all(replacement.as_deref().unwrap_or(&content))?;
            }
            writer.finish()?;
            Ok(())
        })
    }

    fn validate_archive<R: Read + Seek>(&self, archive: &mut zip::ZipArchive<R>) -> Result<()> {
        if archive.len() > self.limits.max_entries {
            return Err(DocError::Format(format!(
                "OOXML package contains {} entries, limit is {}",
                archive.len(),
                self.limits.max_entries
            )));
        }

        let mut total = 0_u64;
        for index in 0..archive.len() {
            let entry = archive.by_index(index)?;
            let size = entry.size();
            if size > self.limits.max_entry_bytes {
                return Err(DocError::Format(format!(
                    "OOXML entry '{}' expands to {size} bytes, limit is {}",
                    entry.name(),
                    self.limits.max_entry_bytes
                )));
            }
            total = total.checked_add(size).ok_or_else(|| {
                DocError::Format("OOXML package expanded size overflow".to_owned())
            })?;
            if total > self.limits.max_total_bytes {
                return Err(DocError::Format(format!(
                    "OOXML package expands to more than {} bytes",
                    self.limits.max_total_bytes
                )));
            }
            let compressed = entry.compressed_size();
            if compressed > 0 && size / compressed.max(1) > self.limits.max_compression_ratio {
                return Err(DocError::Format(format!(
                    "OOXML entry '{}' exceeds compression ratio limit {}",
                    entry.name(),
                    self.limits.max_compression_ratio
                )));
            }
        }
        Ok(())
    }
}

fn entry_options(entry: &zip::read::ZipFile<'_, File>) -> SimpleFileOptions {
    let mut options = SimpleFileOptions::default().compression_method(entry.compression());
    if let Some(time) = entry.last_modified() {
        options = options.last_modified_time(time);
    }
    if let Some(mode) = entry.unix_mode() {
        options = options.unix_permissions(mode);
    }
    options
}
