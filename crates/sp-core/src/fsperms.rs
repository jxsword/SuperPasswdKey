//! 下载/导出文件的权限收敛（单源；SP-F-104/908 附件与导出共用）。
//! 2026-10-29 自 sp-cli 下沉：icacls 用户名须展开为实际值（原 %username% 写法
//! 实测静默失败=限权未生效）。
//! 2026-10-31 S3 审计修正：**fail-closed**——USERNAME 缺失、icacls 启动失败或
//! 非零退出均返回错误（原实现静默跳过并返回 Ok=限权未生效无感知；AGENTS §3
//! 破坏性操作守卫同口径：读取失败必须中止并报告，不得按成功继续）。

use std::path::Path;

use crate::error::SpError;

/// 把已落盘文件收敛为仅当前用户可读（Windows: icacls 移除继承仅留当前用户，
/// 等价 0600 语义；unix: chmod 0600）。任一环节失败即报错（fail-closed）：
/// 调用方（附件下载/--out-file/inject）会把错误上抛，文件保留但用户得以及时
/// 察觉并手动处理，而不是带着默认继承 ACL 的"限权成功"假象。
pub fn restrict(path: &Path) -> Result<(), SpError> {
    #[cfg(windows)]
    {
        // icacls：移除继承、仅保留当前用户（等价 0600 的 Windows 语义）。
        // 用户名须展开为实际值——icacls 不展开 %VAR%（原 %username% 写法实测
        // 静默失败=限权未生效；2026-10-29 修复）。
        // 注：USERNAME 取自进程环境（同用户会话内可信；跨会话攻击者已拥有
        // 直读文件的能力，不构成新增暴露面——威胁模型见 SRS SP-F-104）。
        let user = std::env::var("USERNAME").map_err(|_| {
            SpError::Io(
                "restrict perms: USERNAME not set; refusing to leave file with default ACLs".into(),
            )
        })?;
        let status = std::process::Command::new("icacls")
            .arg(path)
            .arg("/inheritance:r")
            .arg("/grant:r")
            .arg(format!("{user}:F"))
            .status()
            .map_err(|e| SpError::Io(format!("icacls spawn: {e}")))?;
        if !status.success() {
            return Err(SpError::Io(format!(
                "icacls failed ({status}); file ACL NOT restricted"
            )));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| SpError::Io(format!("chmod: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sp-fsperms-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn restrict_ok_on_existing_file() {
        let path = tmp_path("ok");
        std::fs::write(&path, b"sp-test-only").unwrap();
        restrict(&path).expect("restrict must succeed on an existing file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "file must be 0600 after restrict");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn restrict_errors_on_missing_file_fail_closed() {
        // S3 审计口径：限权对象缺失/icacls 失败必须报错，不得静默 Ok
        let path = tmp_path("missing");
        assert!(restrict(&path).is_err(), "restrict must fail closed");
    }
}
