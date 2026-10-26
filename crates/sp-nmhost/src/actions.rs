//! 动作处理（SP-F-702，design_docs/11 §3）：payload → data 映射。
//! 纪律：不提供任何"导出全库/全字段枚举"动作；密码/TOTP 仅 sp.get-secret 单条显式取回；
//! 每个成功请求刷新空闲计时；机密值不进错误链。

use serde_json::{json, Value};

use crate::match_url;
use crate::session::{Session, DEFAULT_SESSION_TIMEOUT_SECS};

#[derive(Debug)]
pub struct ActionError {
    pub code: &'static str,
    pub message: String,
}

impl ActionError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn bad_request(msg: impl Into<String>) -> Self {
        Self::new("badrequest", msg)
    }

    fn locked() -> Self {
        Self::new("locked", "database is locked")
    }
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
pub fn handle(session: &Session, payload: &Value) -> Result<Value, ActionError> {
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
                "sessionTimeoutSecs": session.timeout_secs(),
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
            let factors = session
                .active_factors()
                .map_err(|_| ActionError::locked())?;
            let db_path = session.db_path().ok_or_else(ActionError::locked)?;
            let db = sp_kdbx::open_db(&db_path, &factors)?;
            let summaries = sp_kdbx::list_entry_summaries(&db);
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
            let factors = session
                .active_factors()
                .map_err(|_| ActionError::locked())?;
            let db_path = session.db_path().ok_or_else(ActionError::locked)?;
            let db = sp_kdbx::open_db(&db_path, &factors)?;
            let value = match field {
                "password" => sp_kdbx::find_entry(&db, id)?
                    .password
                    .map(|p| p.to_string())
                    .ok_or_else(|| {
                        ActionError::new("notfound", format!("entry '{id}' has no password"))
                    })?,
                "totp" => sp_kdbx::entry_totp_now(&db, id)?,
                other => {
                    return Err(ActionError::bad_request(format!(
                        "unsupported field '{other}'"
                    )))
                }
            };
            json!({ "value": value })
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
}
