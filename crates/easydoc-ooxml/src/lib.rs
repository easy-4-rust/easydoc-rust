//! OOXML ZIP 包的安全读写基础设施。

#![deny(unsafe_code)]

mod atomic_file;
mod package_limits;
mod package_rewriter;

pub use atomic_file::AtomicFile;
pub use package_limits::PackageLimits;
pub use package_rewriter::PackageRewriter;
