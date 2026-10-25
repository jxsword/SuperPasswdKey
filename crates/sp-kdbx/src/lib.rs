//! sp-kdbx：KDBX4 适配层（keepass_ng 封装，DR-005 候选实现）。
//! 设计见 design_docs/02；round-trip 对拍测试见 tests/roundtrip.rs（H1 实现）。

pub use keepass_ng as engine;

/// 适配层占位：H1 spike 中实现 KdbxAdapter trait（create/open/CRUD/attachments/save_atomic）。
pub const ADAPTER_NAME: &str = "keepass_ng";

#[cfg(test)]
mod tests {
    #[test]
    fn adapter_declared() {
        assert_eq!(super::ADAPTER_NAME, "keepass_ng");
    }
}
