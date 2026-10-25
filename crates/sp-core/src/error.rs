//! 统一错误模型（SP-F-204）：错误 → 退出码映射的唯一权威。
//! 规则：错误链不携带密钥值（SP-N-305）；输出一律 stderr。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpError {
    /// 凭据错误（密码错/keyfile 缺失或非法）→ 退出码 2
    #[error("credentials rejected (wrong password or missing keyfile)")]
    Credentials,
    /// 条目/URI 未找到 → 退出码 3
    #[error("entry not found: {0}")]
    NotFound(String),
    /// 库格式问题（损坏/未知版本/非 KDBX4）→ 退出码 1
    #[error("database format error: {0}")]
    Format(String),
    /// 配置文件问题 → 退出码 1
    #[error("config error: {0}")]
    Config(String),
    /// 文件系统 I/O → 退出码 1
    #[error("io error: {0}")]
    Io(String),
    /// 剪贴板后端错误 → 退出码 1
    #[error("clipboard error: {0}")]
    Clipboard(String),
    /// 业务参数校验失败 → 退出码 4
    #[error("invalid parameter: {0}")]
    Parameter(String),
}

impl SpError {
    /// 退出码映射（SP-F-204）：0 成功 / 1 一般 / 2 凭据 / 3 未找到 / 4 参数
    pub fn exit_code(&self) -> i32 {
        match self {
            SpError::Credentials => 2,
            SpError::NotFound(_) => 3,
            SpError::Parameter(_) => 4,
            SpError::Format(_) | SpError::Config(_) | SpError::Io(_) | SpError::Clipboard(_) => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_mapping() {
        assert_eq!(SpError::Credentials.exit_code(), 2);
        assert_eq!(SpError::NotFound("x".into()).exit_code(), 3);
        assert_eq!(SpError::Parameter("x".into()).exit_code(), 4);
        assert_eq!(SpError::Format("x".into()).exit_code(), 1);
        assert_eq!(SpError::Io("x".into()).exit_code(), 1);
    }
}
