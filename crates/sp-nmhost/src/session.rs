//! 会话状态机（SP-F-703，design_docs/11 §7，DR-006 方案 A）：
//! 解锁一次后缓存凭据因子（SecureString=Zeroizing，drop 即零化）；
//! 每请求重开库（含 KDF ~700ms，keepass_ng 无公开复合密钥直开 API，见 10 §3）；
//! 空闲超时自动锁库（丢弃因子）。业务层经 active_factors/touch 使用。

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use sp_core::{CredentialFactors, SecureString, SpError};

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
}
