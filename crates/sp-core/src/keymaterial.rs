//! 密钥材料与凭据因子（SP-F-105、SP-N-305）。
//!
//! 内存卫生纪律：主密码/派生材料一律 `Zeroizing` 包装；错误链与日志不得携带值。

use std::path::PathBuf;

use zeroize::Zeroizing;

/// 主密码缓冲（Drop 时零化）
pub type SecureString = Zeroizing<String>;

/// 凭据双因子（红线 #7）：主密码 + 密钥文件路径（keyfile 内容由 IO 层读取后立即交给适配层）。
#[derive(Debug, Clone)]
pub struct CredentialFactors {
    pub password: SecureString,
    pub keyfile_path: Option<PathBuf>,
}

impl CredentialFactors {
    pub fn new(password: SecureString, keyfile_path: Option<PathBuf>) -> Self {
        Self {
            password,
            keyfile_path,
        }
    }
}
