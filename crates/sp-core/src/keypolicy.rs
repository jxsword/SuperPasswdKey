//! KDF 参数策略（SP-N-302/SP-F-101）：Argon2id 参数、校准与硬下限。
//! 库内加密由 KDBX4 格式承担（design_docs/09§1）；本模块只管策略与校验。

use crate::error::SpError;

/// Argon2id 参数（映射 KDBX4 头部字段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    /// 内存 KiB。硬下限 64MiB（红线 #6）
    pub memory_kib: u32,
    /// 迭代次数
    pub iterations: u32,
    /// 并行度
    pub parallelism: u32,
}

/// 硬下限：64 MiB（基线红线 #6，SRS SP-F-101）
pub const MIN_MEMORY_KIB: u32 = 64 * 1024;

impl Default for KdfParams {
    /// 默认 m=64MiB/t=8/p=4（保守下限组合；init 时经交互校准上调至 ~700ms，SP-N-302）
    fn default() -> Self {
        Self {
            memory_kib: MIN_MEMORY_KIB,
            iterations: 8,
            parallelism: 4,
        }
    }
}

impl KdfParams {
    pub fn validate(&self) -> Result<(), SpError> {
        if self.memory_kib < MIN_MEMORY_KIB {
            return Err(SpError::Parameter(format!(
                "argon2 memory {} KiB below hard floor {} KiB",
                self.memory_kib, MIN_MEMORY_KIB
            )));
        }
        if self.iterations == 0 || self.parallelism == 0 {
            return Err(SpError::Parameter(
                "argon2 iterations/parallelism must be > 0".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_passes_validation() {
        assert!(KdfParams::default().validate().is_ok());
    }

    #[test]
    fn below_hard_floor_rejected() {
        let p = KdfParams {
            memory_kib: 32 * 1024,
            ..KdfParams::default()
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn zero_iterations_rejected() {
        let p = KdfParams {
            iterations: 0,
            ..KdfParams::default()
        };
        assert!(p.validate().is_err());
    }
}
