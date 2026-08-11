//! 保真度测试的 fixture 集合。
//!
//! 每个 fixture 是程序化生成的 DOCX 文档，以内存字节存储。
//! 预期输出（`view_as(Plain)` 的纯文本）在初始化时通过将字节写入临时文件再读回捕获。
//!
//! 这些 fixture 验证性能优化不会引入数据偏差——预期输出的每个字节都必须匹配。

mod image;
mod list;
mod png;
mod rich;
mod simple;
mod table;
mod types;

pub(crate) use types::Fixtures;
