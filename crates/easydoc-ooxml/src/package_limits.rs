/// 防止畸形或恶意 OOXML ZIP 包耗尽资源的限制。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackageLimits {
    /// 允许的最大 ZIP 条目数。
    pub max_entries: usize,
    /// 单个解压条目的最大字节数。
    pub max_entry_bytes: u64,
    /// 所有解压条目的最大总字节数。
    pub max_total_bytes: u64,
    /// 单个条目允许的最大解压缩比。
    pub max_compression_ratio: u64,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_entry_bytes: 256 * 1024 * 1024,
            max_total_bytes: 1024 * 1024 * 1024,
            max_compression_ratio: 1_000,
        }
    }
}
