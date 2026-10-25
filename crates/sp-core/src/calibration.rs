//! KDF 参数校准（SP-N-302：init 时交互基准校准至目标毫秒，硬下限 64MiB）。
//!
//! 校准原理：Argon2id 时间 ≈ 线性于 memory × iterations。以默认参数实测，
//! 线性外推至目标时长，再实测验证一次（不收敛则再修正一轮）。

use std::time::Instant;

use argon2::{Algorithm, Argon2, Params, Version};

use crate::error::SpError;
use crate::keypolicy::KdfParams;

const ARGON2_SALT: &[u8] = b"sp-kdf-calibration-salt"; // 校准专用，非安全材料
const ARGON2_SECRET_PWD: &[u8] = b"calibration";

fn measure(p: &KdfParams) -> Result<u128, SpError> {
    let params = Params::new(
        p.memory_kib,
        p.iterations,
        p.parallelism,
        Some(32), // 输出长度与 KDBX 用途一致
    )
    .map_err(|e| SpError::Parameter(format!("argon2 params: {e}")))?;
    let a2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let start = Instant::now();
    let mut out = [0u8; 32];
    a2.hash_password_into(ARGON2_SECRET_PWD, ARGON2_SALT, &mut out)
        .map_err(|e| SpError::Parameter(format!("argon2: {e}")))?;
    // out 为校准副产品，即刻零化（drop 由数组作用域处理；显式覆盖）
    out.fill(0);
    Ok(start.elapsed().as_millis())
}

/// 校准：实测默认参数，线性外推至 target_ms，二次验证。
/// 返回的参数保证通过 validate()（硬下限以上）。
pub fn calibrate(target_ms: u64) -> Result<KdfParams, SpError> {
    let p = KdfParams::default();
    p.validate()?;
    let measured = measure(&p)?;
    if measured == 0 {
        return Ok(p); // 异常快（不可能路径），保守返回默认
    }
    // 线性外推（time ∝ memory × iterations）
    let scale = target_ms as f64 / measured as f64;
    let projected = (p.memory_kib as f64 * scale) as u32;
    // 钳制：硬下限以上、合理上限 1GiB（防意外）
    let mut candidate = KdfParams {
        memory_kib: projected.clamp(crate::keypolicy::MIN_MEMORY_KIB, 1024 * 1024),
        ..p
    };
    candidate.validate()?;
    // 二次实测：偏差 >30% 再修正一轮
    let m2 = measure(&candidate)?;
    if m2 > 0 {
        let ratio = m2 as f64 / target_ms as f64;
        if !(0.7..=1.3).contains(&ratio) {
            let refined = (candidate.memory_kib as f64 / ratio) as u32;
            candidate.memory_kib = refined.clamp(crate::keypolicy::MIN_MEMORY_KIB, 1024 * 1024);
            candidate.validate()?;
        }
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 校准目标故意设小（本机毫秒级），验证流程收敛且结果合法
    #[test]
    fn calibrate_converges_and_valid() {
        let p = calibrate(50).expect("calibrate");
        p.validate().expect("params valid");
        assert!(p.memory_kib >= crate::keypolicy::MIN_MEMORY_KIB);
    }
}
