//! URL 匹配引擎——M1a 已抽取至 sp-core（design_docs/14 §3，SP-F-803 复用）。
//! 本模块保留同路径再导出：`crate::match_url::xxx` 引用与既有外部使用者不受影响。

pub use sp_core::match_url::*;
