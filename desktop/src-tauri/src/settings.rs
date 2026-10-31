//! 非机密设置持久化（任务书_step7 任务 2，design_docs/15）：
//! %APPDATA%/SuperPasswdKey/settings.json——仅存超时配置与自启偏好，
//! 任何凭据不入此文件（DR-013 机密纪律）；损坏/缺失回落默认值不崩。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 设置文件面（serde_json 读写；超时/自启为任务书口径，keyfile_path 为 2026-10-31
/// 验收反馈新增——路径非机密值（design_docs/15 §6 同口径），解锁成功后自动记忆）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsFile {
    /// 空闲锁库超时（毫秒；默认 300_000 对齐 DR-006）
    pub idle_ms: u64,
    /// 绝对过期超时（毫秒；默认 1_800_000 = 30min，DR-013 用户示例口径）
    pub absolute_ms: u64,
    /// 开机自启偏好（落点为 HKCU Run 注册表值；启动时按本字段对账）
    pub autostart: bool,
    /// 上次成功解锁使用的 keyfile 路径（空=未使用/无 keyfile；文件不存在时
    /// get_settings 返回空串但本字段保留——U 盘 keyfile 临时拔出后恢复即复用）
    #[serde(default)]
    pub keyfile_path: String,
}

impl Default for SettingsFile {
    fn default() -> Self {
        SettingsFile {
            idle_ms: 300_000,
            absolute_ms: 1_800_000,
            autostart: false,
            keyfile_path: String::new(),
        }
    }
}

impl SettingsFile {
    /// → 会话层设置（session::Settings 只关心双超时）
    pub fn session_settings(&self) -> crate::session::Settings {
        crate::session::Settings {
            idle_ms: self.idle_ms,
            absolute_ms: self.absolute_ms,
        }
    }
}

/// 设置文件单一来源路径：%APPDATA%/SuperPasswdKey/settings.json
pub fn settings_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".").join("AppData").join("Roaming"));
    base.join("SuperPasswdKey").join("settings.json")
}

/// 从指定路径加载；缺失或损坏一律回落默认值（不崩，任务书验收口径）
pub fn load_from(path: &std::path::Path) -> SettingsFile {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| {
            // 兼容带 BOM 的编辑器写出（与 nm-host-config.json 同纪律）
            let raw = raw.trim_start_matches('\u{feff}');
            serde_json::from_str(raw).ok()
        })
        .unwrap_or_default()
}

pub fn load_or_default() -> SettingsFile {
    load_from(&settings_path())
}

/// 写入设置文件（目录不存在则创建；写失败返回错误文案供 UI 提示）
pub fn save_to(path: &std::path::Path, s: &SettingsFile) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create settings dir: {e}"))?;
    }
    let body = serde_json::to_string_pretty(s).map_err(|e| format!("serialize settings: {e}"))?;
    std::fs::write(path, body).map_err(|e| format!("write settings: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "sp-desktop-settings-{tag}-{}.json",
            std::process::id()
        ))
    }

    #[test]
    fn roundtrip_preserves_fields() {
        let p = tmp_path("roundtrip");
        let s = SettingsFile {
            idle_ms: 60_000,
            absolute_ms: 900_000,
            autostart: true,
            keyfile_path: "C:\\keys\\vault.keyfile".into(),
        };
        save_to(&p, &s).expect("save");
        assert_eq!(load_from(&p), s);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn missing_file_falls_back_to_default() {
        let p = tmp_path("missing");
        assert_eq!(load_from(&p), SettingsFile::default());
    }

    #[test]
    fn corrupt_file_falls_back_to_default() {
        let p = tmp_path("corrupt");
        std::fs::write(&p, "{ not valid json !!").expect("write garbage");
        assert_eq!(load_from(&p), SettingsFile::default());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn legacy_file_without_keyfield_falls_back() {
        // v0.3.0 settings.json 无 keyfile_path 字段 → serde default 空串，不崩
        let p = tmp_path("legacy");
        std::fs::write(
            &p,
            r#"{"idle_ms":60000,"absolute_ms":900000,"autostart":false}"#,
        )
        .expect("write legacy");
        let s = load_from(&p);
        assert_eq!(s.keyfile_path, String::new());
        assert_eq!(s.idle_ms, 60_000);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn bom_prefixed_file_parses() {
        let p = tmp_path("bom");
        let body = "\u{feff}".to_string()
            + &serde_json::to_string(&SettingsFile {
                idle_ms: 1_000,
                absolute_ms: 2_000,
                autostart: false,
                keyfile_path: String::new(),
            })
            .expect("json");
        std::fs::write(&p, body).expect("write");
        let s = load_from(&p);
        assert_eq!(s.idle_ms, 1_000);
        let _ = std::fs::remove_file(&p);
    }
}
