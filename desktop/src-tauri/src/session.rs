//! 桌面解锁会话（DR-013 批次 1；任务书_step7 设计闸门）。
//!
//! 复用 sp-kffi 的线程封闭 actor（keepass_ng Database !Send+!Sync，与 Android 同款解法）：
//! Vault 句柄跨线程可 Send，命令串行执行在 actor 工作线程上。凭据因子随 open_db 留驻
//! actor（DR-012 A1），锁库随 actor drop 清零——桌面侧不新增机密持有面。
//!
//! 超时模型（DR-013 桌面口径）：idle（touch 刷新）与 absolute（自解锁时刻）双超时独立
//! 生效，任一到即锁库；再次使用须重新输入主密码。惰性检查（访问前判定）与 Android
//! VaultSession 同款；托盘"立即锁定"为显式路径。后台定时主动锁为后续增量（不影响语义，
//! 惰性检查保证机密操作永远不会在过期后执行）。

use std::path::PathBuf;
use std::time::Instant;

use sp_kffi::Vault;

/// 非机密设置（任务 2 落 %APPDATA% 持久化；脚手架期用默认值）
#[derive(Debug, Clone)]
pub struct Settings {
    /// 空闲锁库超时（默认 300s，对齐 DR-006 / SP-F-703 口径）
    pub idle_ms: u64,
    /// 绝对过期超时（默认 1800s；DR-013 用户示例口径 30min）
    pub absolute_ms: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            idle_ms: 300_000,
            absolute_ms: 1_800_000,
        }
    }
}

/// 会话状态机（Tauri State<Mutex<Session>> 持有；Vault: Send+!Sync → Mutex 即可跨线程）
#[derive(Debug)]
pub struct Session {
    vault: Option<Vault>,
    /// 当前打开的库路径（解锁后记录；status 展示与后续 save_session 单一来源）
    db_path: Option<PathBuf>,
    unlocked_at: Option<Instant>,
    last_active: Option<Instant>,
    settings: Settings,
}

/// 给前端的会话状态（不含任何机密）
#[derive(serde::Serialize)]
pub struct SessionStatus {
    pub unlocked: bool,
    /// 会话剩余寿命提示（毫秒；仅展示用途，机密操作仍以后端判定为准）
    pub idle_remaining_ms: u64,
    pub absolute_remaining_ms: u64,
    pub vault_path: String,
    pub vault_exists: bool,
}

impl Session {
    pub fn new(settings: Settings) -> Self {
        Session {
            vault: None,
            db_path: None,
            unlocked_at: None,
            last_active: None,
            settings,
        }
    }

    /// 壳层库路径单一来源：%APPDATA%/SuperPasswdKey/vault.kdbx
    pub fn default_vault_path() -> PathBuf {
        let base = std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| {
            dirs_fallback_home().join("AppData").join("Roaming")
        });
        base.join("SuperPasswdKey").join("vault.kdbx")
    }

    fn expired(&self) -> bool {
        let Some(t) = self.last_active else {
            return false;
        };
        let idle_hit = t.elapsed().as_millis() as u64 >= self.settings.idle_ms;
        let absolute_hit = self
            .unlocked_at
            .map(|u| u.elapsed().as_millis() as u64 >= self.settings.absolute_ms)
            .unwrap_or(false);
        idle_hit || absolute_hit
    }

    /// 机密操作前置：惰性超时检查（过期即锁库清零），返回可用 Vault。
    /// 脚手架期仅 status() 消费；任务 3 NM host 嵌入后供条目操作命令使用。
    #[allow(dead_code)]
    pub fn active_vault(&mut self) -> Option<&Vault> {
        if self.vault.is_some() && self.expired() {
            self.lock_now();
        }
        self.vault.as_ref()
    }

    /// 解锁（KDF 阻塞数秒——Tauri command 已在工作线程池执行，不卡 UI）。
    /// 旧会话先锁库；凭据仅存于本次调用栈（open_db 消费 String 进 SecureString），
    /// 不落任何持久层。open_db 返回 Arc<Vault>（UniFFI 对象面约束）；此处引用计数
    /// 恰为 1，try_unwrap 取回所有权——Vault(Send+!Sync) 配外层 Mutex 才能满足
    /// Tauri State 的 Send+Sync 约束。
    pub fn unlock(&mut self, db_path: &std::path::Path, password: &str, keyfile: Option<&str>) -> Result<(), sp_kffi::FfiError> {
        if self.vault.is_some() {
            self.lock_now();
        }
        let arc = sp_kffi::open_db(
            db_path.to_string_lossy().to_string(),
            password.to_string(),
            keyfile.map(|s| s.to_string()),
        )?;
        let v = std::sync::Arc::try_unwrap(arc).map_err(|_| sp_kffi::FfiError::Format {
            msg: "vault handle has unexpected extra references".into(),
        })?;
        self.vault = Some(v);
        self.db_path = Some(db_path.to_path_buf());
        self.unlocked_at = Some(Instant::now());
        self.last_active = Some(Instant::now());
        Ok(())
    }

    pub fn touch(&mut self) {
        if self.vault.is_some() {
            self.last_active = Some(Instant::now());
        }
    }

    /// 显式锁库（SP-F-802 语义同 Android）：drop Vault（凭据随 actor 清零）
    pub fn lock_now(&mut self) {
        if let Some(v) = self.vault.take() {
            let _ = v.lock(); // worker 已退出的场景（外部 drop）按已锁处理
        }
        self.db_path = None;
        self.unlocked_at = None;
        self.last_active = None;
    }

    pub fn status(&mut self) -> SessionStatus {
        if self.vault.is_some() && self.expired() {
            self.lock_now();
        }
        let path = self
            .db_path
            .clone()
            .unwrap_or_else(Self::default_vault_path);
        let idle_left = self
            .last_active
            .map(|t| {
                self.settings
                    .idle_ms
                    .saturating_sub(t.elapsed().as_millis() as u64)
            })
            .unwrap_or(0);
        let abs_left = self
            .unlocked_at
            .map(|t| {
                self.settings
                    .absolute_ms
                    .saturating_sub(t.elapsed().as_millis() as u64)
            })
            .unwrap_or(0);
        SessionStatus {
            unlocked: self.vault.is_some(),
            idle_remaining_ms: idle_left,
            absolute_remaining_ms: abs_left,
            vault_path: path.to_string_lossy().to_string(),
            vault_exists: path.exists(),
        }
    }
}

/// FfiError → 类别文案（只映射类别，不携带凭据值——SP-F-804 审查口径）
pub fn describe(e: &sp_kffi::FfiError) -> String {
    match e {
        sp_kffi::FfiError::Credentials => "解锁失败：主密码或 keyfile 不正确".into(),
        sp_kffi::FfiError::NotFound { .. } => "失败：条目不存在".into(),
        sp_kffi::FfiError::Format { .. } => "失败：库文件格式错误或已损坏".into(),
        sp_kffi::FfiError::Io { .. } => "失败：库文件读取/写入错误（IO）".into(),
        sp_kffi::FfiError::Parameter { .. } => "失败：参数错误".into(),
    }
}

/// 无 APPDATA 环境时的兜底（正常 Windows 不会走到）
fn dirs_fallback_home() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::{CredentialFactors, EntryModel, KdfParams, SecureString};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sp-desktop-{tag}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("mkdir");
            let path = dir.join("test.kdbx");
            let factors = CredentialFactors::new(SecureString::new("test-db-pass".into()), None);
            sp_kdbx::create_db(&path, &factors, &KdfParams::default()).expect("create_db");
            let mut db = sp_kdbx::open_db(&path, &factors).expect("seed open");
            sp_kdbx::add_entry(
                &mut db,
                &EntryModel {
                    title: "Demo".into(),
                    username: Some("demo-user".into()),
                    password: Some(SecureString::new("demo-pass".into())),
                    url: Some("https://demo.local".into()),
                    ..Default::default()
                },
            )
            .expect("add entry");
            sp_kdbx::save_atomic(&db, &path, &factors).expect("seed save");
            Fixture(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
        }
    }

    #[test]
    fn unlock_status_lock_roundtrip() {
        let f = Fixture::new("roundtrip");
        let mut s = Session::new(Settings::default());
        assert!(!s.status().unlocked);
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        let st = s.status();
        assert!(st.unlocked);
        assert!(st.vault_exists);
        // actor 可用：列摘要（经 active_vault 语义链路）
        assert!(
            !s.active_vault()
                .expect("vault")
                .list_entry_summaries()
                .unwrap()
                .is_empty()
        );
        s.lock_now();
        assert!(!s.status().unlocked);
    }

    #[test]
    fn wrong_password_is_credentials() {
        let f = Fixture::new("wrongpw");
        let mut s = Session::new(Settings::default());
        match s.unlock(&f.0, "definitely-wrong", None) {
            Err(sp_kffi::FfiError::Credentials) => {}
            other => panic!("expected Credentials, got {other:?}"),
        }
        assert!(!s.status().unlocked);
    }

    #[test]
    fn idle_timeout_zero_locks_lazily() {
        // idle_ms=0 → 解锁后首次机密/状态访问即触发惰性锁库（解锁调用栈本身不判超时，
        // 与 Android VaultSession 懒检查语义一致）
        let f = Fixture::new("timeout");
        let mut s = Session::new(Settings {
            idle_ms: 0,
            absolute_ms: u64::MAX,
        });
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        assert!(
            !s.status().unlocked,
            "idle=0 must lock on first lazy check"
        );
        assert!(s.active_vault().is_none(), "expired session must lock lazily");
        assert!(!s.status().unlocked);
    }

    #[test]
    fn reunlock_replaces_previous_session() {
        let f = Fixture::new("reunlock");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock #1");
        let t1 = s.unlocked_at;
        s.unlock(&f.0, "test-db-pass", None).expect("unlock #2");
        assert!(s.unlocked_at > t1, "absolute timer must reset on re-unlock");
        assert!(s.status().unlocked);
    }
}
