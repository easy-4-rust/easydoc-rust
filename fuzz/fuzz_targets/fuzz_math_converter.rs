//! 模糊测试目标：数学转换器（LaTeX→OMML 与 OMML→LaTeX）
//!
//! 覆盖入口：`easydoc_math::latex_to_omml::convert` 与
//! `easydoc_math::omml_to_latex::convert`
//! 目标：任意字节当作 LaTeX 源码 / OMML XML 解析，不 panic、不崩溃。
//! 对应 roadmap 0.1.0 fuzz 目标中的 "math converter"。
#![no_main]
use libfuzzer_sys::fuzz_target;

use easydoc_math::latex_to_omml;
use easydoc_math::omml_to_latex;

fuzz_target!(|data: &[u8]| {
    // 仅接受合法 UTF-8（fuzzer 会生成合法变体）；错误一律忽略，
    // 重点检测 panic/abort/栈溢出。
    if let Ok(source) = std::str::from_utf8(data) {
        let _ = latex_to_omml::convert(source);
        let _ = omml_to_latex::convert(source);
    }
});
