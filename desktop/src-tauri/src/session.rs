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
    /// 编辑中（2026-10-31 验收反馈）：EntryFormModal 打开期间置 true——暂停 idle
    /// 超时（防编辑被空闲锁定打断丢失输入）；absolute 超时不受影响（绝对上限是
    /// 安全底线）。模态关闭/视图卸载必须复位 false（前端 onUnmounted）。
    editing: bool,
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
            editing: false,
        }
    }

    /// 编辑态开关（EntryFormModal 打开/关闭时由前端调用）。
    pub fn set_editing(&mut self, active: bool) {
        self.editing = active;
        // 打开编辑即视为一次活动（idle 计时从此时重算，关闭后按正常空闲逻辑走）
        if active && self.vault.is_some() {
            self.last_active = Some(Instant::now());
        }
    }

    /// 壳层库路径单一来源：%APPDATA%/SuperPasswdKey/vault.kdbx
    pub fn default_vault_path() -> PathBuf {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| dirs_fallback_home().join("AppData").join("Roaming"));
        base.join("SuperPasswdKey").join("vault.kdbx")
    }

    /// 空闲超时（秒；sp.status 的 sessionTimeoutSecs 展示口径，NM 适配面消费）
    pub fn idle_timeout_secs(&self) -> u64 {
        self.settings.idle_ms / 1000
    }

    fn expired(&self) -> bool {
        let Some(t) = self.last_active else {
            return false;
        };
        // 编辑中暂停 idle 超时（absolute 照常判定——绝对上限不因编辑而绕过）
        let idle_hit = !self.editing && t.elapsed().as_millis() as u64 >= self.settings.idle_ms;
        let absolute_hit = self
            .unlocked_at
            .map(|u| u.elapsed().as_millis() as u64 >= self.settings.absolute_ms)
            .unwrap_or(false);
        idle_hit || absolute_hit
    }

    /// 机密操作前置：惰性超时检查（过期即锁库清零），返回可用 Vault。
    /// GUI 条目操作（entry_list/entry_get/entry_totp）与 NM 适配器共用此语义链路。
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
    pub fn unlock(
        &mut self,
        db_path: &std::path::Path,
        password: &str,
        keyfile: Option<&str>,
    ) -> Result<(), sp_kffi::FfiError> {
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
        // 锁定即退出编辑态（前端模态卸载时还会 set_editing(false)，幂等）
        self.editing = false;
    }

    pub fn status(&mut self) -> SessionStatus {
        if self.vault.is_some() && self.expired() {
            self.lock_now();
        }
        let path = self
            .db_path
            .clone()
            .unwrap_or_else(Self::default_vault_path);
        // 编辑中 idle 不触发锁定——展示同步为满值（避免顶栏倒计时归零却不锁的矛盾观感）
        let idle_left = if self.editing && self.vault.is_some() {
            self.settings.idle_ms
        } else {
            self.last_active
                .map(|t| {
                    self.settings
                        .idle_ms
                        .saturating_sub(t.elapsed().as_millis() as u64)
                })
                .unwrap_or(0)
        };
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

    /// 条目摘要列表（GUI entry_list；惰性超时检查，过期即锁库清零）。
    pub fn entry_list(&mut self) -> Result<Vec<EntrySummaryDto>, String> {
        let v = self
            .active_vault()
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())?;
        v.list_entry_summaries()
            .map(|s| s.into_iter().map(EntrySummaryDto::from).collect())
            .map_err(|e| describe(&e))
    }

    /// 条目详情（GUI entry_get；含密码与 custom_fields——仅详情视图瞬态展示，不落前端存储）。
    /// 经 find_entry_full 完整读回：编辑保真前提（旧 FfiEntry 面会丢 custom_fields）。
    pub fn entry_get(&mut self, title: &str) -> Result<EntryDetailDto, String> {
        let v = self
            .active_vault()
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())?;
        v.find_entry_full(title.to_string())
            .map(EntryDetailDto::from)
            .map_err(|e| describe(&e))
    }

    /// 条目当前 TOTP 码（GUI entry_totp；RFC 6238 本地生成，SP-F-601 单源）。
    pub fn entry_totp(&mut self, title: &str) -> Result<String, String> {
        let v = self
            .active_vault()
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())?;
        v.entry_totp_now(title.to_string())
            .map_err(|e| describe(&e))
    }

    /// 新建条目（GUI entry_create；复用 Vault::add_entry + save_session 零凭据保存，
    /// DR-012 A1 同源；同名允许——对齐 CLI item create 口径，靠列表/搜索区分）。
    pub fn entry_create(&mut self, draft: EntryDraftDto) -> Result<(), String> {
        let path = self.session_db_path()?;
        let v = self
            .active_vault()
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())?;
        v.add_entry(draft.into_ffi()).map_err(|e| describe(&e))?;
        v.save_session(path).map_err(|e| describe(&e))?;
        self.touch();
        Ok(())
    }

    /// 编辑条目（GUI entry_edit；复用 Vault::edit_entry——旧快照进 history，
    /// SP-F-103 语义单源；按 old_title 定位，标题可经 draft.title 改写）。
    pub fn entry_edit(&mut self, old_title: &str, draft: EntryDraftDto) -> Result<(), String> {
        let path = self.session_db_path()?;
        let v = self
            .active_vault()
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())?;
        v.edit_entry(old_title.to_string(), draft.into_ffi())
            .map_err(|e| describe(&e))?;
        v.save_session(path).map_err(|e| describe(&e))?;
        self.touch();
        Ok(())
    }

    /// 删除条目（GUI entry_delete；复用 Vault::delete_entry + save_session）。
    pub fn entry_delete(&mut self, title: &str) -> Result<(), String> {
        let path = self.session_db_path()?;
        let v = self
            .active_vault()
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())?;
        v.delete_entry(title.to_string())
            .map_err(|e| describe(&e))?;
        v.save_session(path).map_err(|e| describe(&e))?;
        self.touch();
        Ok(())
    }

    /// 会话库路径（解锁时记录；save_session 的落盘目标单一来源）
    fn session_db_path(&self) -> Result<String, String> {
        self.db_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())
    }

    /// 列条目附件名（GUI attachment_list；S1b-2b）
    pub fn attachment_list(&mut self, title: &str) -> Result<Vec<String>, String> {
        let v = self
            .active_vault()
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())?;
        let names = v
            .list_attachments(title.to_string())
            .map_err(|e| describe(&e))?;
        self.touch();
        Ok(names)
    }

    /// 添加附件（GUI attachment_add；同名覆盖语义 = sp_kdbx::add_attachment）。
    pub fn attachment_add(&mut self, title: &str, name: &str, data: Vec<u8>) -> Result<(), String> {
        let path = self.session_db_path()?;
        let v = self
            .active_vault()
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())?;
        v.add_attachment(title.to_string(), name.to_string(), data)
            .map_err(|e| describe(&e))?;
        v.save_session(path).map_err(|e| describe(&e))?;
        self.touch();
        Ok(())
    }

    /// 读附件字节（GUI 下载用；落盘由 command 层完成并做限权收敛）
    pub fn attachment_get(&mut self, title: &str, name: &str) -> Result<Vec<u8>, String> {
        let v = self
            .active_vault()
            .ok_or_else(|| "会话已锁定，请重新解锁".to_string())?;
        let data = v
            .get_attachment(title.to_string(), name.to_string())
            .map_err(|e| describe(&e))?;
        self.touch();
        Ok(data)
    }
}

// ---------- GUI DTO（serde 序列化面；字段与前端 api.ts 镜像一一对应） ----------

/// 条目摘要 DTO（不含密码/TOTP，SP-F-706 语义）
#[derive(serde::Serialize, Debug)]
pub struct EntrySummaryDto {
    pub title: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub has_otp: bool,
    /// 标签分类（SP-F-108，S1b-2）
    pub tags: Vec<String>,
}

impl From<sp_kffi::FfiEntrySummary> for EntrySummaryDto {
    fn from(s: sp_kffi::FfiEntrySummary) -> Self {
        EntrySummaryDto {
            title: s.title,
            username: s.username,
            url: s.url,
            has_otp: s.has_otp,
            tags: s.tags,
        }
    }
}

/// 自定义字段 DTO（读回与草稿共用；与前端 api.ts 镜像）
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct CustomFieldDto {
    pub name: String,
    pub value: String,
    pub protected: bool,
}

/// 条目详情 DTO（含密码与 custom_fields——仅经 entry_get 单条显式取回，详情视图瞬态展示）
#[derive(serde::Serialize, Debug)]
pub struct EntryDetailDto {
    pub title: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub otp: Option<String>,
    pub custom_fields: Vec<CustomFieldDto>,
    /// 标签分类（SP-F-108，S1b-2）
    pub tags: Vec<String>,
}

impl From<sp_kffi::FfiEntryFull> for EntryDetailDto {
    fn from(e: sp_kffi::FfiEntryFull) -> Self {
        EntryDetailDto {
            title: e.title,
            username: e.username,
            password: e.password,
            url: e.url,
            notes: e.notes,
            otp: e.otp,
            custom_fields: e
                .custom_fields
                .into_iter()
                .map(CustomFieldDto::from)
                .collect(),
            tags: e.tags,
        }
    }
}

impl From<sp_kffi::FfiCustomField> for CustomFieldDto {
    fn from(f: sp_kffi::FfiCustomField) -> Self {
        CustomFieldDto {
            name: f.name,
            value: f.value,
            protected: f.protected,
        }
    }
}

/// 条目草稿 DTO（GUI 创建/编辑写入面；serde 反序列化自前端 invoke 参数）
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct EntryDraftDto {
    pub title: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub otp: Option<String>,
    /// 编辑时由前端原样回传（保真，不渲染编辑控件——S1b-1 口径，S1b-2 提供 UI）
    #[serde(default)]
    pub custom_fields: Vec<CustomFieldDto>,
    /// 标签分类（SP-F-108，S1b-2）
    #[serde(default)]
    pub tags: Vec<String>,
}

impl EntryDraftDto {
    fn into_ffi(self) -> sp_kffi::FfiEntryDraft {
        sp_kffi::FfiEntryDraft {
            title: self.title,
            username: self.username,
            password: self.password,
            url: self.url,
            notes: self.notes,
            otp: self.otp,
            custom_fields: self
                .custom_fields
                .into_iter()
                .map(|f| sp_kffi::FfiCustomField {
                    name: f.name,
                    value: f.value,
                    protected: f.protected,
                })
                .collect(),
            tags: self.tags,
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
            sp_kdbx::add_entry(
                &mut db,
                &EntryModel {
                    title: "OtpEntry".into(),
                    username: Some("otp-user".into()),
                    password: Some(SecureString::new("otp-pass".into())),
                    otp: Some("otpauth://totp/Test?secret=JBSWY3DPEHPK3PXP&issuer=Test".into()),
                    ..Default::default()
                },
            )
            .expect("add otp entry");
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
        assert!(!s
            .active_vault()
            .expect("vault")
            .list_entry_summaries()
            .unwrap()
            .is_empty());
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
        assert!(!s.status().unlocked, "idle=0 must lock on first lazy check");
        assert!(
            s.active_vault().is_none(),
            "expired session must lock lazily"
        );
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

    #[test]
    fn entry_list_after_unlock_returns_summaries() {
        let f = Fixture::new("gui-list");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        let list = s.entry_list().expect("entry_list");
        assert_eq!(list.len(), 2);
        let demo = list.iter().find(|e| e.title == "Demo").expect("Demo");
        assert_eq!(demo.username.as_deref(), Some("demo-user"));
        assert!(!demo.has_otp);
        let otp = list
            .iter()
            .find(|e| e.title == "OtpEntry")
            .expect("OtpEntry");
        assert!(otp.has_otp);
    }

    #[test]
    fn entry_list_locked_returns_locked_error() {
        let _f = Fixture::new("gui-locked");
        let mut s = Session::new(Settings::default());
        // 未解锁即调用 → locked 语义
        let err = s.entry_list().expect_err("locked");
        assert!(err.contains("锁定"), "unexpected: {err}");
    }

    #[test]
    fn entry_get_returns_detail_with_password() {
        let f = Fixture::new("gui-get");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        let d = s.entry_get("Demo").expect("entry_get");
        assert_eq!(d.username.as_deref(), Some("demo-user"));
        assert_eq!(d.password.as_deref(), Some("demo-pass"));
        assert_eq!(d.url.as_deref(), Some("https://demo.local"));
    }

    #[test]
    fn entry_get_missing_returns_not_found() {
        let f = Fixture::new("gui-missing");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        match s.entry_get("no-such") {
            Err(err) => assert!(err.contains("不存在"), "unexpected: {err}"),
            Ok(_) => panic!("expected not-found error"),
        }
    }

    #[test]
    fn entry_totp_generates_code() {
        let f = Fixture::new("gui-totp");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        let code = s.entry_totp("OtpEntry").expect("entry_totp");
        assert!(!code.is_empty());
        assert!(
            code.len() >= 6 && code.len() <= 8,
            "TOTP code length: {code}"
        );
        assert!(
            code.chars().all(|c| c.is_ascii_digit()),
            "TOTP code digits: {code}"
        );
    }

    #[test]
    fn entry_totp_without_otp_errors() {
        let f = Fixture::new("gui-nootp");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        assert!(
            s.entry_totp("Demo").is_err(),
            "entry without otp must error"
        );
    }

    #[test]
    fn entry_list_expired_session_locks_lazily() {
        // idle=0 → 解锁后首次 entry_list 触发惰性锁库 → locked 语义（与 status 同链路）
        let f = Fixture::new("gui-expire");
        let mut s = Session::new(Settings {
            idle_ms: 0,
            absolute_ms: u64::MAX,
        });
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        let err = s.entry_list().expect_err("lazy lock");
        assert!(err.contains("锁定"), "unexpected: {err}");
        assert!(
            !s.status().unlocked,
            "session must be locked after lazy expiry"
        );
    }

    #[test]
    fn editing_suspends_idle_but_not_absolute() {
        // 2026-10-31 验收反馈：编辑中不因空闲锁定打断（absolute 上限保留）
        let f = Fixture::new("gui-editing");
        let mut s = Session::new(Settings {
            idle_ms: 0, // idle 触底：正常态任何访问即锁
            absolute_ms: u64::MAX,
        });
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");

        // 编辑中：idle=0 也不锁，entry_list 正常返回
        s.set_editing(true);
        assert!(s.entry_list().is_ok(), "editing must suspend idle timeout");
        assert!(s.status().unlocked);

        // 退出编辑态：恢复空闲判定 → 首次访问即锁
        s.set_editing(false);
        let err = s.entry_list().expect_err("idle resumes after editing");
        assert!(err.contains("锁定"), "unexpected: {err}");
    }

    #[test]
    fn editing_does_not_extend_absolute_timeout() {
        // absolute=0：即便编辑中，绝对上限到期仍锁（安全底线不被编辑绕过）
        let f = Fixture::new("gui-editing-abs");
        let mut s = Session::new(Settings {
            idle_ms: u64::MAX,
            absolute_ms: 0,
        });
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        s.set_editing(true);
        let err = s.entry_list().expect_err("absolute must still lock");
        assert!(err.contains("锁定"), "unexpected: {err}");
        assert!(!s.status().unlocked);
    }

    #[test]
    fn lock_now_resets_editing_flag() {
        let f = Fixture::new("gui-editing-lock");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        s.set_editing(true);
        s.lock_now();
        assert!(!s.editing, "lock must clear editing flag");
    }

    fn draft_dto(title: &str, password: &str) -> EntryDraftDto {
        EntryDraftDto {
            title: title.into(),
            username: Some("writer".into()),
            password: Some(password.into()),
            url: Some("https://write.local".into()),
            notes: None,
            otp: None,
            custom_fields: vec![],
            tags: vec![],
        }
    }

    #[test]
    fn entry_tags_roundtrip_through_create_and_edit() {
        // SP-F-108：标签经 GUI 创建/编辑写入，完整读回与摘要均可见
        let f = Fixture::new("gui-tags");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");

        let mut d = draft_dto("Tagged", "p");
        d.tags = vec!["work".into(), " infra ".into()];
        s.entry_create(d).expect("create");
        let got = s.entry_get("Tagged").expect("get");
        assert_eq!(got.tags, vec!["work".to_string(), " infra ".to_string()]);
        let list = s.entry_list().expect("list");
        let summary = list.iter().find(|e| e.title == "Tagged").expect("summary");
        assert_eq!(summary.tags.len(), 2);

        // 编辑改标签 → 覆写生效
        let mut d2 = draft_dto("Tagged", "p");
        d2.tags = vec!["personal".into()];
        s.entry_edit("Tagged", d2).expect("edit");
        let got = s.entry_get("Tagged").expect("get after edit");
        assert_eq!(got.tags, vec!["personal".to_string()]);
    }

    #[test]
    fn entry_create_blank_tag_rejected() {
        // 校验语义与 CLI 同源（sp-core：标签 trim 后不得为空白）
        let f = Fixture::new("gui-blanktag");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        let mut d = draft_dto("BadTag", "p");
        d.tags = vec!["   ".into()];
        match s.entry_create(d) {
            Err(err) => assert!(err.contains("参数错误"), "unexpected: {err}"),
            Ok(()) => panic!("blank tag must be rejected"),
        }
    }

    #[test]
    fn entry_create_then_full_readback() {
        let f = Fixture::new("gui-create");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");

        let mut d = draft_dto("Created", "created-pass");
        d.custom_fields = vec![CustomFieldDto {
            name: "api_key".into(),
            value: "k-1".into(),
            protected: true,
        }];
        s.entry_create(d).expect("entry_create");

        // 列表含新条目；完整读回含 custom_fields（创建即保真；读回顺序不保证——按名断言）
        let list = s.entry_list().expect("list");
        assert!(list.iter().any(|e| e.title == "Created"));
        let got = s.entry_get("Created").expect("entry_get");
        assert_eq!(got.password.as_deref(), Some("created-pass"));
        assert_eq!(got.custom_fields.len(), 1);
        assert_eq!(got.custom_fields[0].name, "api_key");
    }

    #[test]
    fn entry_edit_updates_and_preserves_custom_fields() {
        let f = Fixture::new("gui-edit");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");

        let mut d = draft_dto("Editable", "pass-1");
        d.custom_fields = vec![
            CustomFieldDto {
                name: "api_key".into(),
                value: "k-1".into(),
                protected: true,
            },
            CustomFieldDto {
                name: "region".into(),
                value: "cn".into(),
                protected: false,
            },
        ];
        s.entry_create(d).expect("create");

        // 编辑：改密码 + 改标题，custom_fields 原样回传（保真口径）
        let mut d2 = draft_dto("Editable-Renamed", "pass-2");
        d2.custom_fields = vec![
            CustomFieldDto {
                name: "api_key".into(),
                value: "k-1".into(),
                protected: true,
            },
            CustomFieldDto {
                name: "region".into(),
                value: "cn".into(),
                protected: false,
            },
        ];
        s.entry_edit("Editable", d2).expect("entry_edit");

        match s.entry_get("Editable") {
            Err(err) => assert!(err.contains("不存在"), "old title must be gone: {err}"),
            Ok(_) => panic!("old title must not resolve after rename"),
        }
        let got = s.entry_get("Editable-Renamed").expect("renamed");
        assert_eq!(got.password.as_deref(), Some("pass-2"));
        // keepass-ng additional_attributes 为 HashMap 存储，读回顺序跨平台不确定——按集合断言
        assert_eq!(
            got.custom_fields
                .iter()
                .map(|c| c.name.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from(["api_key", "region"]),
            "custom fields must survive edit"
        );
    }

    #[test]
    fn entry_edit_missing_errors_not_found() {
        let f = Fixture::new("gui-edit-missing");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        match s.entry_edit("no-such", draft_dto("x", "y")) {
            Err(err) => assert!(err.contains("不存在"), "unexpected: {err}"),
            Ok(()) => panic!("expected not-found"),
        }
    }

    #[test]
    fn entry_delete_removes_entry() {
        let f = Fixture::new("gui-delete");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        s.entry_create(draft_dto("Doomed", "p")).expect("create");
        s.entry_delete("Doomed").expect("delete");
        match s.entry_get("Doomed") {
            Err(err) => assert!(err.contains("不存在"), "unexpected: {err}"),
            Ok(_) => panic!("deleted entry must not resolve"),
        }
    }

    #[test]
    fn entry_write_locked_denied() {
        let _f = Fixture::new("gui-write-locked");
        let mut s = Session::new(Settings::default());
        match s.entry_create(draft_dto("X", "y")) {
            Err(err) => assert!(err.contains("锁定"), "unexpected: {err}"),
            Ok(()) => panic!("locked session must deny create"),
        }
    }

    #[test]
    fn entry_create_invalid_otp_rejected() {
        // 校验语义与 CLI 同源（sp-core：otp 必须 otpauth:// 开头）
        let f = Fixture::new("gui-badotp");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        let mut d = draft_dto("BadOtp", "p");
        d.otp = Some("not-otpauth".into());
        match s.entry_create(d) {
            Err(err) => assert!(err.contains("参数错误"), "unexpected: {err}"),
            Ok(()) => panic!("invalid otp must be rejected"),
        }
    }

    #[test]
    fn attachment_add_list_get_roundtrip() {
        // S1b-2b：GUI 附件管理（list 为 kffi Rust-only 面；add 同名覆盖；get 字节往返）
        let f = Fixture::new("gui-attach");
        let mut s = Session::new(Settings::default());
        s.unlock(&f.0, "test-db-pass", None).expect("unlock");
        s.entry_create(draft_dto("WithFiles", "p")).expect("create");

        s.attachment_add("WithFiles", "a.txt", b"alpha".to_vec())
            .expect("add 1");
        s.attachment_add("WithFiles", "b.bin", vec![1, 2, 3])
            .expect("add 2");
        let mut names = s.attachment_list("WithFiles").expect("list");
        names.sort();
        assert_eq!(names, vec!["a.txt".to_string(), "b.bin".to_string()]);

        let got = s.attachment_get("WithFiles", "a.txt").expect("get");
        assert_eq!(got, b"alpha".to_vec());

        // 同名覆盖（对齐 CLI 语义）
        s.attachment_add("WithFiles", "a.txt", b"alpha-2".to_vec())
            .expect("overwrite");
        let got = s.attachment_get("WithFiles", "a.txt").expect("get 2");
        assert_eq!(got, b"alpha-2".to_vec());
        assert_eq!(s.attachment_list("WithFiles").expect("list 2").len(), 2);

        // 锁定态拒绝
        s.lock_now();
        match s.attachment_list("WithFiles") {
            Err(err) => assert!(err.contains("锁定"), "unexpected: {err}"),
            Ok(_) => panic!("locked session must deny attachment list"),
        }
    }
}
