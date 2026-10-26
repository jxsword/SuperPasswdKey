#![no_main]
//! .env 解析器 fuzz（design_docs/07 §1「解析器属性」家族 / SP-C-106）：
//! 任意 UTF-8 输入下 `parse_env_str` 不得 panic；非 UTF-8 字节序列
//! 按 .env 规范（design_docs/04 §3：UTF-8）本就不进入解析路径。

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = sp_cli::f8::parse_env_str(s);
    }
});
