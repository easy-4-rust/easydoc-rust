//! 模糊测试目标：easydoc-markdown 的 Markdown 导入解析器
//!
//! 覆盖入口：`MarkdownImportBuilder::do_import()`
//! 目标：任意字节当作 Markdown 源码解析（标题/列表/表格/代码块/引用/
//! 数学公式/脚注等语法），不 panic、不崩溃。
//! 对应 roadmap 0.1.0 fuzz 目标中的 "Markdown parser"。
#![no_main]
use libfuzzer_sys::fuzz_target;

use easydoc_markdown::MarkdownImportBuilder;

fuzz_target!(|data: &[u8]| {
    // 仅接受合法 UTF-8（fuzzer 会生成合法变体）；错误一律忽略，
    // 重点检测 panic/abort
    if let Ok(source) = std::str::from_utf8(data) {
        let _ = MarkdownImportBuilder::new(source).do_import();
    }
});
