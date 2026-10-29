//! 会话状态机（SP-F-703，design_docs/11 §7，DR-006 方案 A）：
//! 解锁一次后缓存凭据因子（SecureString=Zeroizing，drop 即零化）；
//! 每请求重开库（含 KDF ~700ms，keepass_ng 无公开复合密钥直开 API，见 10 §3）；
//! 空闲超时自动锁库（丢弃因子）。业务层经 active_factors/touch 使用。

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use zeroize::Zeroizing;

use sp_core::{CredentialFactors, SecureString, SpError};

use crate::actions::{ActionError, EntryDigest, NmActionSession, SaveOutcome, SavePlan};

pub const DEFAULT_SESSION_TIMEOUT_SECS: u64 = 300;

pub struct Session {
    inner: Mutex<Inner>,
    timeout: Duration,
    keyfile: Option<PathBuf>,
}

struct Inner {
    factors: Option<CredentialFactors>,
    db_path: Option<PathBuf>,
    last_activity: Instant,
}

impl Session {
    pub fn new(timeout_secs: u64, keyfile: Option<PathBuf>) -> Self {
        Self {
            inner: Mutex::new(Inner {
                factors: None,
                db_path: None,
                last_activity: Instant::now(),
            }),
            timeout: Duration::from_secs(timeout_secs),
            keyfile,
        }
    }

    pub fn timeout_secs(&self) -> u64 {
        self.timeout.as_secs()
    }

    pub fn set_db_path(&self, path: PathBuf) {
        self.inner_lock().db_path = Some(path);
    }

    pub fn db_path(&self) -> Option<PathBuf> {
        self.inner_lock().db_path.clone()
    }

    /// 当前是否锁定；若已解锁但空闲超时则先惰性锁库（watcher 线程的兜底路径）
    pub fn is_locked(&self) -> bool {
        let mut inner = self.inner_lock();
        if let Some(f) = &inner.factors {
            if inner.last_activity.elapsed() > self.timeout {
                let _ = f;
                inner.factors = None; // zeroize on drop
                return true;
            }
            return false;
        }
        true
    }

    /// 解锁：构建凭据因子并试开库校验（KDF）；失败不缓存任何材料。
    /// 返回条目数（status 展示用）。
    pub fn unlock(&self, password: &str) -> Result<usize, SpError> {
        let mut inner = self.inner_lock();
        let db_path = inner
            .db_path
            .clone()
            .ok_or_else(|| SpError::Config("no database configured".into()))?;
        let factors = CredentialFactors::new(
            SecureString::new(password.to_string()),
            self.keyfile.clone(),
        );
        let db = sp_kdbx::open_db(&db_path, &factors)?;
        let count = sp_kdbx::list_titles(&db).len();
        inner.factors = Some(factors);
        inner.last_activity = Instant::now();
        Ok(count)
    }

    /// 显式锁库：立即丢弃凭据因子
    pub fn lock(&self) {
        self.inner_lock().factors = None;
    }

    /// 是否处于解锁态且空闲计时已超过阈值（watcher 线程判断主动锁库用）
    pub fn idle_expired(&self) -> bool {
        let inner = self.inner_lock();
        inner.factors.is_some() && inner.last_activity.elapsed() > self.timeout
    }

    /// 取当前有效凭据因子（若空闲超时则先锁库）；调用方用毕即丢。
    pub fn active_factors(&self) -> Result<CredentialFactors, SpError> {
        let mut inner = self.inner_lock();
        match inner.factors.clone() {
            Some(f) if inner.last_activity.elapsed() <= self.timeout => Ok(f),
            Some(_) => {
                inner.factors = None; // 超时：零化
                Err(SpError::Credentials)
            }
            None => Err(SpError::Credentials),
        }
    }

    /// 刷新空闲计时（每次成功请求后调用）
    pub fn touch(&self) {
        let mut inner = self.inner_lock();
        if inner.factors.is_some() {
            inner.last_activity = Instant::now();
        }
    }

    fn inner_lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// 动作分发面实现：锁定/取值语义与 handle 的纪律对齐（锁定态机密操作 → locked；
/// 无值 → notfound；机密值不进错误链）。
impl NmActionSession for Session {
    fn is_locked(&self) -> bool {
        Session::is_locked(self)
    }

    fn db_path(&self) -> Option<PathBuf> {
        Session::db_path(self)
    }

    fn session_timeout_secs(&self) -> u64 {
        self.timeout_secs()
    }

    fn unlock(&self, password: &str) -> Result<usize, ActionError> {
        Session::unlock(self, password).map_err(ActionError::from)
    }

    fn lock(&self) {
        Session::lock(self)
    }

    fn entry_summaries(&self) -> Result<Vec<EntryDigest>, ActionError> {
        let factors = self.active_factors().map_err(|_| ActionError::locked())?;
        let db_path = self.db_path().ok_or_else(ActionError::locked)?;
        let db = sp_kdbx::open_db(&db_path, &factors)?;
        Ok(sp_kdbx::list_entry_summaries(&db)
            .into_iter()
            .map(|s| EntryDigest {
                title: s.title,
                username: s.username,
                url: s.url,
                has_otp: s.has_otp,
            })
            .collect())
    }

    fn entry_secret(&self, id: &str, field: &str) -> Result<String, ActionError> {
        let factors = self.active_factors().map_err(|_| ActionError::locked())?;
        let db_path = self.db_path().ok_or_else(ActionError::locked)?;
        let db = sp_kdbx::open_db(&db_path, &factors)?;
        match field {
            "password" => sp_kdbx::find_entry(&db, id)?
                .password
                .map(|p| p.to_string())
                .ok_or_else(|| {
                    ActionError::new("notfound", format!("entry '{id}' has no password"))
                }),
            "totp" => Ok(sp_kdbx::entry_totp_now(&db, id)?),
            other => Err(ActionError::bad_request(format!(
                "unsupported field '{other}'"
            ))),
        }
    }

    /// 保存登录凭据（SP-F-708）：每请求重开库模型同款（open → 改 → save_atomic）。
    /// 更新路径经 find_entry 读回全字段（notes/otp/custom_fields 保真），仅换
    /// 密码/用户名/url；旧密码由 edit_entry 自动压入 history（SP-F-103）。
    fn save_login(
        &self,
        page_url: &str,
        username: &str,
        password: &str,
        host: &str,
    ) -> Result<SaveOutcome, ActionError> {
        let factors = self.active_factors().map_err(|_| ActionError::locked())?;
        let db_path = self.db_path().ok_or_else(ActionError::locked)?;
        let mut db = sp_kdbx::open_db(&db_path, &factors)?;
        let summaries: Vec<(String, Option<String>, Option<String>)> =
            sp_kdbx::list_entry_summaries(&db)
                .into_iter()
                .map(|s| (s.title, s.username, s.url))
                .collect();
        let outcome = match crate::actions::plan_save_login(&summaries, host, username) {
            SavePlan::Update { title } => {
                let mut model = sp_kdbx::find_entry(&db, &title)?;
                model.username = Some(username.to_string());
                model.password = Some(Zeroizing::new(password.to_string()));
                model.url = Some(page_url.to_string());
                sp_kdbx::edit_entry(&mut db, &title, &model)?;
                SaveOutcome {
                    action: "updated",
                    title,
                }
            }
            SavePlan::Insert { title } => {
                sp_kdbx::add_entry(
                    &mut db,
                    &sp_core::EntryModel {
                        title: title.clone(),
                        username: Some(username.to_string()),
                        password: Some(Zeroizing::new(password.to_string())),
                        url: Some(page_url.to_string()),
                        ..Default::default()
                    },
                )?;
                SaveOutcome {
                    action: "added",
                    title,
                }
            }
        };
        sp_kdbx::save_atomic(&db, &db_path, &factors)?;
        Ok(outcome)
    }

    fn touch(&self) {
        Session::touch(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::KdfParams;
    use std::path::PathBuf;

    fn test_params() -> KdfParams {
        // m 下限 64MiB（SP-F-101），单测可接受
        KdfParams {
            memory_kib: 64 * 1024,
            iterations: 2,
            parallelism: 1,
        }
    }

    struct TempDb {
        path: PathBuf,
        _keep: (),
    }

    impl TempDb {
        fn new(password: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sp-nmhost-test-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join(format!("{}.kdbx", rand_suffix()));
            sp_kdbx::create_db(&path, &factors(password), &test_params()).unwrap();
            Self { path, _keep: () }
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn factors(pw: &str) -> CredentialFactors {
        CredentialFactors::new(SecureString::new(pw.to_string()), None)
    }

    fn rand_suffix() -> u32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    }

    #[test]
    fn unlock_failure_leaves_session_locked() {
        let db = TempDb::new("correct-horse");
        let s = Session::new(300, None);
        s.set_db_path(db.path.clone());
        assert!(s.is_locked());
        assert!(matches!(s.unlock("wrong"), Err(SpError::Credentials)));
        assert!(s.is_locked());
        assert!(s.active_factors().is_err());
    }

    #[test]
    fn unlock_success_then_lock() {
        let db = TempDb::new("correct-horse");
        let s = Session::new(300, None);
        s.set_db_path(db.path.clone());
        let count = s.unlock("correct-horse").expect("unlock ok");
        assert_eq!(count, 0);
        assert!(!s.is_locked());
        assert!(s.active_factors().is_ok());
        s.lock();
        assert!(s.is_locked());
    }

    #[test]
    fn missing_db_path_is_config_error() {
        let s = Session::new(300, None);
        assert!(matches!(s.unlock("x"), Err(SpError::Config(_))));
    }

    #[test]
    fn idle_timeout_locks_lazily() {
        let db = TempDb::new("correct-horse");
        let s = Session::new(300, None);
        s.set_db_path(db.path.clone());
        s.unlock("correct-horse").expect("unlock ok");
        // 测试注入口：把 last_activity 拨到超时之前
        {
            let mut inner = s.inner.lock().unwrap_or_else(PoisonError::into_inner);
            inner.last_activity = Instant::now() - Duration::from_secs(301);
        }
        assert!(s.is_locked());
        assert!(s.active_factors().is_err());
    }

    #[test]
    fn touch_extends_session() {
        let db = TempDb::new("correct-horse");
        let s = Session::new(300, None);
        s.set_db_path(db.path.clone());
        s.unlock("correct-horse").expect("unlock ok");
        {
            let mut inner = s.inner.lock().unwrap_or_else(PoisonError::into_inner);
            inner.last_activity = Instant::now() - Duration::from_secs(299);
        }
        s.touch();
        assert!(!s.is_locked());
    }

    /// SP-F-708：save-login 新增 → 更新（同 host+user）→ 撞题消歧（同 host 异 user）
    /// → get-logins 联动；锁定态拒绝。
    #[test]
    fn save_login_added_updated_disambiguated_and_lock() {
        let db = TempDb::new("correct-horse");
        let s = Session::new(300, None);
        s.set_db_path(db.path.clone());
        let save = |s: &Session, url: &str, user: &str, pw: &str| {
            crate::actions::handle(
                s,
                &serde_json::json!({"action": "sp.save-login", "requestID": "t",
                    "url": url, "username": user, "password": pw}),
            )
        };
        // 锁定态 → locked
        assert_eq!(
            save(&s, "https://ex.com/login", "u1", "p1")
                .unwrap_err()
                .code,
            "locked"
        );
        s.unlock("correct-horse").expect("unlock");
        // 新增：标题 = host
        let r = save(&s, "https://ex.com/login", "u1", "p1").expect("save 1");
        assert_eq!(r["saved"], "added");
        assert_eq!(r["title"], "ex.com");
        // 更新：同 host+user（不同 url/密码），旧密码进 history
        let r = save(&s, "https://ex.com/other", "u1", "p2").expect("save 2");
        assert_eq!(r["saved"], "updated");
        assert_eq!(r["title"], "ex.com");
        assert_eq!(s.entry_secret("ex.com", "password").expect("secret"), "p2");
        // 同 host 不同 user → 标题撞题，消歧为 "host (username)"
        let r = save(&s, "https://ex.com", "u2", "p3").expect("save 3");
        assert_eq!(r["title"], "ex.com (u2)");
        // get-logins 匹配到两条，且响应不含任何密码值
        let r = crate::actions::handle(
            &s,
            &serde_json::json!({"action": "sp.get-logins", "requestID": "t", "url": "https://ex.com"}),
        )
        .expect("get-logins");
        assert_eq!(r["matches"].as_array().expect("matches").len(), 2);
        let resp = r.to_string();
        assert!(
            !resp.contains("p1") && !resp.contains("p2") && !resp.contains("p3"),
            "get-logins 不得含密码：{resp}"
        );
    }
}
