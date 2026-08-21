//! 模糊测试目标：easydoc_reader::DocxSaxReader 的 XML 层解析
//!
//! 覆盖入口：`DocxSaxReader::from_reader(reader)` + `read_blocks()`
//! 目标：任意字节当作 XML 文档流式解析，不 panic、不崩溃。
//! 对应 easyofd 的 `fuzz_xml` 目标（纯内存，无需文件系统）。
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

use easydoc_reader::DocxSaxReader;

fuzz_target!(|data: &[u8]| {
    // from_reader 包装任意 Read 源，直接消费原始 XML 字节
    let mut reader = DocxSaxReader::from_reader(Cursor::new(data));
    // 错误一律忽略，重点检测 panic/abort；read_blocks 覆盖完整块解析路径
    let _ = reader.read_blocks();
});
