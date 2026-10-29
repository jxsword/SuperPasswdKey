//! 动作处理（SP-F-702，design_docs/11 §3）：payload → data 映射。
//! 纪律：不提供任何"导出全库/全字段枚举"动作；密码/TOTP 仅 sp.get-secret 单条显式取回；
//! 每个成功请求刷新空闲计时；机密值不进错误链。
//!
//! 会话实现经 `NmActionSession` trait 注入（step7 桌面单进程嵌入复用同一分发纪律，
//! design_docs/15）；锁定语义由实现方保证——锁定态机密操作返回 Err(locked)。

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::match_url;
use crate::session::DEFAULT_SESSION_TIMEOUT_SECS;

#[derive(Debug)]
pub struct ActionError {
    pub code: &'static str,
    pub message: String,
}

impl ActionError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new("badrequest", msg)
    }

    pub fn locked() -> Self {
        Self::new("locked", "database is locked")
    }
}

/// 条目摘要（不含密码/TOTP；get-logins 匹配面）
#[derive(Clone, Debug)]
pub struct EntryDigest {
    pub title: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub has_otp: bool,
}

/// 动作分发与会话实现的解耦面：sp-nmhost 自带因子缓存会话与桌面 Vault actor 会话
/// 共用一套动作语义（step7 任务 3，单源纪律：无全库导出、touch 于成功后统一执行）。
pub trait NmActionSession {
    fn is_locked(&self) -> bool;
    fn db_path(&self) -> Option<PathBuf>;
    fn session_timeout_secs(&self) -> u64;
    /// 解锁（KDF 校验；失败不缓存任何材料）；返回条目数（status 展示用）
    fn unlock(&self, password: &str) -> Result<usize, ActionError>;
    fn lock(&self);
    /// 全部条目摘要；锁定态 → Err(locked)
    fn entry_summaries(&self) -> Result<Vec<EntryDigest>, ActionError>;
    /// 单条字段显式取回（field 白名单由 handle 校验后才进入实现）；无值 → Err(notfound)
    fn entry_secret(&self, id: &str, field: &str) -> Result<String, ActionError>;
    /// 保存一条登录凭据（SP-F-708；host+username 精确去重，对齐 SP-F-805 语义）。
    /// host 已由 handle 经 login_host 解析（http/https 白名单 + host[:port]）。
    /// 锁定态 → Err(locked)；写库原子保存由实现方负责。
    fn save_login(
        &self,
        page_url: &str,
        username: &str,
        password: &str,
        host: &str,
    ) -> Result<SaveOutcome, ActionError>;
    fn touch(&self);
}

/// save-login 结果（added=新增 / updated=更新既有，旧密码自动进 history）
#[derive(Debug, Clone, PartialEq)]
pub struct SaveOutcome {
    pub action: &'static str,
    pub title: String,
}

/// 去重判定结果：Update（命中既有条目）或 Insert（新增，标题已消歧）
#[derive(Debug, Clone, PartialEq)]
pub enum SavePlan {
    Update { title: String },
    Insert { title: String },
}

/// 登录归属 host（SP-F-708）：仅解析 URL——http/https 白名单 + 小写 `host[:port]`。
/// **不发起任何网络请求**（零网络红线不变；localhost 等本地页面保存不受影响）。
pub fn login_host(page_url: &str) -> Result<String, ActionError> {
    let parsed =
        match_url::parse_url(page_url).ok_or_else(|| ActionError::bad_request("invalid url"))?;
    if parsed.scheme != "http" && parsed.scheme != "https" {
        return Err(ActionError::bad_request("url scheme must be http/https"));
    }
    if parsed.host.is_empty() {
        return Err(ActionError::bad_request("url host is empty"));
    }
    Ok(match parsed.port {
        Some(port) => format!("{}:{}", parsed.host, port),
        None => parsed.host,
    })
}

/// 由条目摘要计算落库动作与标题：**url 的 host(含端口) + username 精确相等** →
/// Update（旧密码自动进 history）；否则 Insert，标题=host，撞题时
/// `host (username)` 多级消歧（对齐 SP-F-805 去重口径；不归并子域、不猜测）。
pub fn plan_save_login(
    summaries: &[(String, Option<String>, Option<String>)], // (title, username, url)
    host: &str,
    username: &str,
) -> SavePlan {
    for (title, u, url) in summaries {
        let entry_host = url
            .as_deref()
            .and_then(match_url::parse_url)
            .map(|p| match p.port {
                Some(port) => format!("{}:{}", p.host, port),
                None => p.host,
            });
        if entry_host.as_deref() == Some(host) && u.as_deref() == Some(username) {
            return SavePlan::Update {
                title: title.clone(),
            };
        }
    }
    let taken: std::collections::HashSet<&str> =
        summaries.iter().map(|(t, _, _)| t.as_str()).collect();
    let mut title = host.to_string();
    if taken.contains(title.as_str()) {
        title = format!("{host} ({username})");
        let mut n = 2;
        while taken.contains(title.as_str()) {
            title = format!("{host} ({username}) {n}");
            n += 1;
        }
    }
    SavePlan::Insert { title }
}

impl From<sp_core::SpError> for ActionError {
    fn from(e: sp_core::SpError) -> Self {
        let code = match e {
            sp_core::SpError::Credentials => "credentials",
            sp_core::SpError::NotFound(_) => "notfound",
            sp_core::SpError::Parameter(_) => "badrequest",
            _ => "internal",
        };
        Self::new(code, e.to_string())
    }
}

/// 处理一个请求 payload，返回响应 data；错误统一由调用方包装为 error 响应。
pub fn handle<S: NmActionSession + ?Sized>(
    session: &S,
    payload: &Value,
) -> Result<Value, ActionError> {
    let action = payload
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| ActionError::bad_request("missing action"))?;
    let data = match action {
        "sp.ping" => {
            json!({ "locked": session.is_locked() })
        }
        "sp.status" => {
            let path = session.db_path();
            json!({
                "locked": session.is_locked(),
                "dbPath": path.as_ref().map(|p| p.display().to_string()),
                "sessionTimeoutSecs": session.session_timeout_secs(),
            })
        }
        "sp.unlock" => {
            let password = payload
                .get("password")
                .and_then(Value::as_str)
                .ok_or_else(|| ActionError::bad_request("missing password"))?;
            // keyfile 不经管道：由 host 启动参数/config 解析（design_docs/10 §3）
            let count = session.unlock(password)?;
            json!({ "entryCount": count })
        }
        "sp.lock" => {
            session.lock();
            json!({})
        }
        "sp.get-logins" => {
            let url = payload
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| ActionError::bad_request("missing url"))?;
            let summaries = session.entry_summaries()?;
            let urls: Vec<Option<&str>> = summaries.iter().map(|s| s.url.as_deref()).collect();
            let matches: Vec<Value> = match_url::rank(url, &urls)
                .into_iter()
                .map(|i| {
                    let s = &summaries[i];
                    json!({
                        "id": s.title,
                        "title": s.title,
                        "username": s.username,
                        "url": s.url,
                        "hasOtp": s.has_otp,
                    })
                })
                .collect();
            json!({ "matches": matches })
        }
        "sp.get-secret" => {
            let id = payload
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| ActionError::bad_request("missing id"))?;
            let field = payload
                .get("field")
                .and_then(Value::as_str)
                .ok_or_else(|| ActionError::bad_request("missing field"))?;
            // 锁定优先于字段白名单（与既有错误优先级一致：锁定态一律 locked）
            if session.is_locked() {
                return Err(ActionError::locked());
            }
            let value = match field {
                "password" | "totp" => session.entry_secret(id, field)?,
                other => {
                    return Err(ActionError::bad_request(format!(
                        "unsupported field '{other}'"
                    )))
                }
            };
            json!({ "value": value })
        }
        "sp.save-login" => {
            let url = payload
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| ActionError::bad_request("missing url"))?;
            let username = payload
                .get("username")
                .and_then(Value::as_str)
                .ok_or_else(|| ActionError::bad_request("missing username"))?;
            let password = payload
                .get("password")
                .and_then(Value::as_str)
                .ok_or_else(|| ActionError::bad_request("missing password"))?;
            if username.is_empty() || password.is_empty() {
                return Err(ActionError::bad_request(
                    "username and password must be non-empty",
                ));
            }
            let host = login_host(url)?;
            let outcome = session.save_login(url, username, password, &host)?;
            json!({ "saved": outcome.action, "title": outcome.title })
        }
        "sp.event" => {
            return Err(ActionError::bad_request(
                "sp.event is host-to-extension only",
            ));
        }
        other => {
            return Err(ActionError::bad_request(format!(
                "unknown action '{other}'"
            )))
        }
    };
    // ping/status 不依赖解锁态，也参与空闲计时（扩展保活语义，design_docs/11 §7）
    session.touch();
    Ok(data)
}

pub fn default_session_timeout_secs() -> u64 {
    DEFAULT_SESSION_TIMEOUT_SECS
}

// 测试用库口令/值均为 SRS §5 认可的 fixture 常量（sp-test-only 惯例，仅测试用途，可入仓）
#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    use serde_json::json;
    use sp_core::{CredentialFactors, KdfParams, SecureString};
    use std::path::PathBuf;

    const TEST_DB_PASSWORD: &str = "sp-test-only";
    const TEST_ENTRY_PASSWORD: &str = "sp-nmhost-test-secret-value";

    struct TempDb(PathBuf);

    impl TempDb {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("sp-nmhost-act-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join(format!(
                "t{}.kdbx",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));
            sp_kdbx::create_db(
                &path,
                &CredentialFactors::new(SecureString::new(TEST_DB_PASSWORD.into()), None),
                &KdfParams {
                    memory_kib: 64 * 1024,
                    iterations: 2,
                    parallelism: 1,
                },
            )
            .unwrap();
            Self(path)
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn req(action: &str, extra: Value) -> Value {
        let mut o = json!({"action": action, "requestID": "test1234"});
        if let (Some(base), Some(add)) = (o.as_object_mut(), extra.as_object()) {
            for (k, v) in add {
                base.insert(k.clone(), v.clone());
            }
        }
        o
    }

    #[test]
    fn ping_and_status_work_while_locked() {
        let s = Session::new(300, None);
        let r = handle(&s, &req("sp.ping", json!({}))).unwrap();
        assert_eq!(r["locked"], true);
        let r = handle(&s, &req("sp.status", json!({}))).unwrap();
        assert_eq!(r["locked"], true);
        assert_eq!(r["sessionTimeoutSecs"], 300);
        assert_eq!(r["dbPath"], Value::Null);
    }

    #[test]
    fn unknown_action_and_event_rejected() {
        let s = Session::new(300, None);
        assert_eq!(
            handle(&s, &req("sp.nope", json!({}))).unwrap_err().code,
            "badrequest"
        );
        assert_eq!(
            handle(&s, &req("sp.event", json!({}))).unwrap_err().code,
            "badrequest"
        );
        assert_eq!(
            handle(&s, &json!({"requestID": "x"})).unwrap_err().code,
            "badrequest"
        );
    }

    #[test]
    fn secrets_blocked_while_locked() {
        let s = Session::new(300, None);
        assert_eq!(
            handle(
                &s,
                &req("sp.get-logins", json!({"url": "https://example.com"}))
            )
            .unwrap_err()
            .code,
            "locked"
        );
        assert_eq!(
            handle(
                &s,
                &req("sp.get-secret", json!({"id": "a", "field": "password"}))
            )
            .unwrap_err()
            .code,
            "locked"
        );
    }

    #[test]
    fn unlock_validation_errors() {
        let s = Session::new(300, None);
        assert_eq!(
            handle(&s, &req("sp.unlock", json!({}))).unwrap_err().code,
            "badrequest"
        );
        // 未配置库路径 → config 错误映射为 internal
        assert_eq!(
            handle(&s, &req("sp.unlock", json!({"password": "x"})))
                .unwrap_err()
                .code,
            "internal"
        );
    }

    #[test]
    fn full_flow_unlock_get_logins_empty_get_secret_notfound() {
        let db = TempDb::new();
        let s = Session::new(300, None);
        s.set_db_path(db.0.clone());
        let r = handle(&s, &req("sp.unlock", json!({"password": TEST_DB_PASSWORD}))).unwrap();
        assert_eq!(r["entryCount"], 0);
        let r = handle(
            &s,
            &req("sp.get-logins", json!({"url": "https://example.com/login"})),
        )
        .unwrap();
        assert_eq!(r["matches"].as_array().unwrap().len(), 0);
        assert_eq!(
            handle(
                &s,
                &req("sp.get-secret", json!({"id": "nope", "field": "password"}))
            )
            .unwrap_err()
            .code,
            "notfound"
        );
        assert_eq!(
            handle(
                &s,
                &req("sp.get-secret", json!({"id": "a", "field": "notes"}))
            )
            .unwrap_err()
            .code,
            "badrequest"
        );
        // 锁定后再取 → locked
        handle(&s, &req("sp.lock", json!({}))).unwrap();
        assert_eq!(
            handle(
                &s,
                &req("sp.get-logins", json!({"url": "https://example.com"}))
            )
            .unwrap_err()
            .code,
            "locked"
        );
    }

    #[test]
    fn get_logins_matches_url_and_requires_unlock() {
        let db = TempDb::new();
        let s = Session::new(300, None);
        s.set_db_path(db.0.clone());
        handle(&s, &req("sp.unlock", json!({"password": TEST_DB_PASSWORD}))).unwrap();
        // 写入一个带 url 的条目再匹配
        let factors = s.active_factors().unwrap();
        let mut dbh = sp_kdbx::open_db(&db.0, &factors).unwrap();
        sp_kdbx::add_entry(
            &mut dbh,
            &sp_core::EntryModel {
                title: "sp-nmhost-test-entry".into(),
                username: Some("sp-test-user".into()),
                password: Some(zeroize::Zeroizing::new(TEST_ENTRY_PASSWORD.into())),
                url: Some("https://example.com".into()),
                ..Default::default()
            },
        )
        .unwrap();
        sp_kdbx::save_atomic(&dbh, &db.0, &factors).unwrap();
        drop(dbh);
        let r = handle(
            &s,
            &req(
                "sp.get-logins",
                json!({"url": "https://login.example.com/x"}),
            ),
        )
        .unwrap();
        let matches = r["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["id"], "sp-nmhost-test-entry");
        assert_eq!(matches[0]["username"], "sp-test-user");
        assert!(
            matches[0].get("password").is_none(),
            "get-logins 不得含密码"
        );
        // 反例：子串误配
        let r = handle(
            &s,
            &req("sp.get-logins", json!({"url": "https://evilexample.com"})),
        )
        .unwrap();
        assert_eq!(r["matches"].as_array().unwrap().len(), 0);
        // 单条显式取回
        let r = handle(
            &s,
            &req(
                "sp.get-secret",
                json!({"id": "sp-nmhost-test-entry", "field": "password"}),
            ),
        )
        .unwrap();
        assert_eq!(r["value"], TEST_ENTRY_PASSWORD);
    }

    /// SP-F-708：save-login 的 handle 层校验（缺字段/空值/非 http(s)/坏 URL → badrequest；
    /// 机密值不进错误链）
    #[test]
    fn save_login_handle_validation() {
        let db = TempDb::new();
        let s = Session::new(300, None);
        s.set_db_path(db.0.clone());
        handle(&s, &req("sp.unlock", json!({"password": TEST_DB_PASSWORD}))).unwrap();

        // 缺 url/username/password → badrequest
        assert_eq!(
            handle(
                &s,
                &req("sp.save-login", json!({"username": "u", "password": "p"}))
            )
            .unwrap_err()
            .code,
            "badrequest"
        );
        assert_eq!(
            handle(
                &s,
                &req(
                    "sp.save-login",
                    json!({"url": "https://ex.com", "password": "p"})
                )
            )
            .unwrap_err()
            .code,
            "badrequest"
        );
        // 空值 → badrequest
        assert_eq!(
            handle(
                &s,
                &req(
                    "sp.save-login",
                    json!({"url": "https://ex.com", "username": "", "password": "p"})
                )
            )
            .unwrap_err()
            .code,
            "badrequest"
        );
        // 非 http/https → badrequest（错误信息不含密码值）
        let e = handle(
            &s,
            &req(
                "sp.save-login",
                json!({"url": "ftp://ex.com", "username": "u", "password": "SECRET-PW-VALUE"}),
            ),
        )
        .unwrap_err();
        assert_eq!(e.code, "badrequest");
        assert!(
            !e.message.contains("SECRET-PW-VALUE"),
            "机密值不得进错误链：{e:?}"
        );
        // 坏 URL → badrequest
        assert_eq!(
            handle(
                &s,
                &req(
                    "sp.save-login",
                    json!({"url": "not-a-url", "username": "u", "password": "p"})
                )
            )
            .unwrap_err()
            .code,
            "badrequest"
        );
    }

    /// SP-F-708：login_host 白名单与 host[:port] 归一
    #[test]
    fn login_host_scheme_and_port() {
        assert_eq!(login_host("https://ex.com/x").unwrap(), "ex.com");
        assert_eq!(
            login_host("http://localhost:8081/a").unwrap(),
            "localhost:8081"
        );
        assert!(login_host("ftp://ex.com").is_err());
        assert!(login_host("javascript:alert(1)").is_err());
        assert!(login_host("not-a-url").is_err());
    }

    /// SP-F-708：去重判定纯函数——精确 host+username、不归并子域、端口参与、撞题消歧
    #[test]
    fn plan_save_login_dedupe_and_disambiguation() {
        let sums = vec![
            (
                "ex.com".to_string(),
                Some("u1".to_string()),
                Some("https://ex.com".to_string()),
            ),
            ("other.com".to_string(), None, None),
        ];
        // 命中 → 更新
        assert_eq!(
            plan_save_login(&sums, "ex.com", "u1"),
            SavePlan::Update {
                title: "ex.com".into()
            }
        );
        // 同 host 异 user → 撞题消歧
        assert_eq!(
            plan_save_login(&sums, "ex.com", "u2"),
            SavePlan::Insert {
                title: "ex.com (u2)".into()
            }
        );
        // 新 host → 直接用 host
        assert_eq!(
            plan_save_login(&sums, "other.org", "u"),
            SavePlan::Insert {
                title: "other.org".into()
            }
        );
        // 子域不归并（精确匹配）
        assert_eq!(
            plan_save_login(&sums, "sub.ex.com", "u1"),
            SavePlan::Insert {
                title: "sub.ex.com".into()
            }
        );
        // 端口参与匹配；无端口条目与带端口页面互不命中
        let sums2 = vec![(
            "localhost:8081".to_string(),
            Some("u".to_string()),
            Some("http://localhost:8081".to_string()),
        )];
        assert_eq!(
            plan_save_login(&sums2, "localhost:8081", "u"),
            SavePlan::Update {
                title: "localhost:8081".into()
            }
        );
        assert_eq!(
            plan_save_login(&sums2, "localhost", "u"),
            SavePlan::Insert {
                title: "localhost".into()
            }
        );
        // 二级撞题：多级消歧（更新匹配要求条目 url 可解析出 host）
        let sums3 = vec![
            (
                "ex.com".to_string(),
                Some("u1".to_string()),
                Some("https://ex.com".to_string()),
            ),
            (
                "ex.com (u2)".to_string(),
                Some("u2".to_string()),
                Some("https://ex.com".to_string()),
            ),
        ];
        assert_eq!(
            plan_save_login(&sums3, "ex.com", "u3"),
            SavePlan::Insert {
                title: "ex.com (u3)".into()
            }
        );
        assert_eq!(
            plan_save_login(&sums3, "ex.com", "u2"),
            SavePlan::Update {
                title: "ex.com (u2)".into()
            }
        );
    }
}
