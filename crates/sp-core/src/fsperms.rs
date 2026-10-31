//! 下载/导出文件的权限收敛（单源；SP-F-104/908 附件与导出共用）。
//! 2026-10-29 自 sp-cli 下沉：icacls 用户名须展开为实际值（原 %username% 写法
//! 实测静默失败=限权未生效）。

use std::path::Path;

use crate::error::SpError;

/// 把已落盘文件收敛为仅当前用户可读（Windows: icacls 移除继承仅留当前用户，
/// 等价 0600 语义；unix: chmod 0600）。
pub fn restrict(path: &Path) -> Result<(), SpError> {
    #[cfg(windows)]
    {
        // icacls：移除继承、仅保留当前用户（等价 0600 的 Windows 语义）。
        // 用户名须展开为实际值——icacls 不展开 %VAR%（原 %username% 写法实测
        // 静默失败=限权未生效；2026-10-29 修复）。
        if let Ok(user) = std::env::var("USERNAME") {
            let _ = std::process::Command::new("icacls")
                .arg(path)
                .args(["/inheritance:r", "/grant:r"])
                .arg(format!("{user}:F"))
                .status();
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
