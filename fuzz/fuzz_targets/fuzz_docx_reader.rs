//! 模糊测试目标：easydoc-reader 完整 DOCX/ZIP 读取管线（纯内存）
//!
//! 覆盖入口：`easydoc_reader::read_text_from_bytes()` / `read_document_from_bytes()`
//! 目标：任意字节直接按 DOCX/DOC 解析（magic bytes 检测 + ZIP 解压 + XML
//! 解析 + office_oxide 语义转换），不 panic、不崩溃。
//! 对应 easyofd 的 `fuzz_ofd_reader` 目标；`from_bytes` 入口保证零文件
//! 系统访问，fuzz 吞吐远高于临时文件方案。
#![no_main]
use libfuzzer_sys::fuzz_target;

use easydoc_reader::{read_document_from_bytes, read_text_from_bytes};

fuzz_target!(|data: &[u8]| {
    // 错误一律忽略，重点检测 panic/abort
    let _ = read_text_from_bytes(data);
    let _ = read_document_from_bytes(data);
});
