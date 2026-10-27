//! sp-core：密钥策略层、条目模型、错误模型、URL 匹配引擎（纯内存域，无 IO）。
//!
//! 职责边界（design_docs/01）：库内加密由 KDBX4 格式承担，本 crate 不做二次加密；
//! 聚焦 KDF 参数策略、双因子组合、内存卫生（zeroize）与统一错误模型。
//! match_url（SP-F-706/803）为桌面扩展与 Android Autofill 共用的单源匹配引擎（M1a 抽取）。

pub mod calibration;
pub mod entry;
pub mod error;
pub mod keymaterial;
pub mod keypolicy;
pub mod match_url;

pub use entry::{CustomField, EntryModel};
pub use error::SpError;
pub use keymaterial::{CredentialFactors, SecureString};
pub use keypolicy::KdfParams;

/// 库名（生成器标识，写入 KDBX Meta）
pub const GENERATOR_NAME: &str = concat!("SuperPasswdKey ", env!("CARGO_PKG_VERSION"));

#[cfg(test)]
mod tests {
    #[test]
    fn generator_name_contains_version() {
        assert!(super::GENERATOR_NAME.starts_with("SuperPasswdKey "));
        assert!(super::GENERATOR_NAME.contains(env!("CARGO_PKG_VERSION")));
    }
}
