//! NM host 单进程嵌入（任务书_step7 任务 3，DR-013 单进程架构，design_docs/15）：
//! sp-desktop.exe 同时是托盘 GUI 进程与 native messaging host——浏览器经 manifest
//! 拉起本进程（argv[1] = 扩展 origin）时，NM 请求循环跑在专用线程，经
//! `DesktopNmSession` 适配器与会话管理器同进程共享同一 Session（会话进程内闭环，
//! 免跨进程会话共享/IPC；协议编解码/动作分发复用 sp-nmhost 库，零复制）。
//!
//! 生命周期：浏览器 connectNative → 拉起新的 sp-desktop 进程 → NM 线程 serve →
//! 端口关闭（stdin EOF）→ 进程退出（与 sp-nmhost bin 同语义）。每进程一次首次
//! 主密码（DR-013 桌面口径"每次进程启动首次主密码"）：锁定态收到机密请求 →
//! 回 locked 语义 + 弹解锁窗，主密码在窗口输入、不经 NM 管道。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::Manager;

use sp_nmhost::actions::{ActionError, EntryDigest, NmActionSession};
use sp_nmhost::nm_loop::{self, ServeEnd};
use sp_nmhost::origin;
use sp_nmhost::protocol::{encode_envelope, write_frame, Envelope};

use crate::session::{describe, Session};

/// 桌面 Vault actor 会话 → sp-nmhost 动作分发面适配器。
/// NM 侧解锁目标 = 桌面单库（%APPDATA%/SuperPasswdKey/vault.kdbx）；keyfile 仅来自
/// nm-host-config.json（不经管道，design_docs/10 §3），未配置时 NM 解锁等价无 keyfile。
struct DesktopNmSession {
    session: Arc<Mutex<Session>>,
    keyfile: Option<PathBuf>,
    db_path: PathBuf,
}

impl NmActionSession for DesktopNmSession {
    fn is_locked(&self) -> bool {
        // status() 内含惰性双超时检查（过期即锁库）
        !self
            .session
            .lock()
            .expect("session poisoned")
            .status()
            .unlocked
    }

    fn db_path(&self) -> Option<PathBuf> {
        Some(self.db_path.clone())
    }

    fn session_timeout_secs(&self) -> u64 {
        self.session
            .lock()
            .expect("session poisoned")
            .idle_timeout_secs()
    }

    fn unlock(&self, password: &str) -> Result<usize, ActionError> {
        let mut s = self.session.lock().expect("session poisoned");
        let keyfile = self
            .keyfile
            .as_deref()
            .map(|p| p.to_string_lossy().to_string());
        s.unlock(&self.db_path, password, keyfile.as_deref())
            .map_err(|e| match e {
                sp_kffi::FfiError::Credentials => {
                    ActionError::new("credentials", "wrong password or keyfile")
                }
                other => ActionError::new("internal", describe(&other)),
            })?;
        let count = s
            .active_vault()
            .and_then(|v| v.list_entry_summaries().ok())
            .map(|l| l.len())
            .unwrap_or(0);
        Ok(count)
    }

    fn lock(&self) {
        self.session.lock().expect("session poisoned").lock_now();
    }

    fn entry_summaries(&self) -> Result<Vec<EntryDigest>, ActionError> {
        let mut s = self.session.lock().expect("session poisoned");
        let v = s.active_vault().ok_or_else(ActionError::locked)?;
        let list = v
            .list_entry_summaries()
            .map_err(|e| ActionError::new("internal", describe(&e)))?;
        Ok(list
            .into_iter()
            .map(|e| EntryDigest {
                title: e.title,
                username: e.username,
                url: e.url,
                has_otp: e.has_otp,
            })
            .collect())
    }

    fn entry_secret(&self, id: &str, field: &str) -> Result<String, ActionError> {
        let mut s = self.session.lock().expect("session poisoned");
        let v = s.active_vault().ok_or_else(ActionError::locked)?;
        match field {
            "password" => v
                .find_entry(id.to_string())
                .map_err(|e| ActionError::new("internal", describe(&e)))?
                .password
                .ok_or_else(|| {
                    ActionError::new("notfound", format!("entry '{id}' has no password"))
                }),
            "totp" => v
                .entry_totp_now(id.to_string())
                .map_err(|e| ActionError::new("internal", describe(&e))),
            other => Err(ActionError::bad_request(format!(
                "unsupported field '{other}'"
            ))),
        }
    }

    fn touch(&self) {
        self.session.lock().expect("session poisoned").touch();
    }
}

/// NM 模式装配（进程由浏览器拉起时从 setup 调用）：
/// ① origin 双重校验（期望值 = SP_NM_ALLOWED_ORIGIN > nm-host-config.json，
///    与 sp-nmhost bin 同序；浏览器 manifest 无启动参数，CLI 口不可用）；
/// ② 双超时 watcher：任一超时锁库（status() 惰性检查兜底）后向扩展发 sp.event locked；
/// ③ 请求循环线程：分发经 DesktopNmSession；locked 错误同步弹解锁窗；
/// ④ serve 结束（EOF/协议错误）→ 进程退出（浏览器端口生命周期语义）。
pub fn spawn(app: tauri::AppHandle, session: Arc<Mutex<Session>>) {
    let expected = origin::bound_origin(None);
    match expected.as_deref() {
        Some(o) => {
            if let Err(e) = origin::check_caller_origin(o, origin::caller_origin().as_deref()) {
                eprintln!("sp-desktop nm: {e}");
                app.exit(4);
                return;
            }
        }
        None => eprintln!(
            "[warn] sp-desktop nm: no bound origin configured (nm-host-config.json); \
             relying on manifest allowed_origins only"
        ),
    }

    let dispatcher = DesktopNmSession {
        keyfile: origin::host_side_config_str("keyfile_path").map(PathBuf::from),
        db_path: Session::default_vault_path(),
        session: session.clone(),
    };

    let stdout = Arc::new(Mutex::new(std::io::stdout()));

    // 双超时 watcher：解锁态翻转为锁定（双超时到期 / 显式锁定）→ 通知扩展
    {
        let session = Arc::clone(&session);
        let stdout = Arc::clone(&stdout);
        std::thread::spawn(move || {
            let mut was_unlocked = false;
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let now_unlocked = session.lock().expect("session poisoned").status().unlocked;
                if was_unlocked && !now_unlocked {
                    let env = Envelope::plain(serde_json::json!({
                        "action": "sp.event",
                        "requestID": null,
                        "event": "locked",
                    }));
                    if let Ok(bytes) = encode_envelope(&env) {
                        let mut w = stdout
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        let _ = write_frame(&mut *w, &bytes);
                    }
                }
                was_unlocked = now_unlocked;
            }
        });
    }

    // 请求循环（专用线程；setup 回调本身在主线程，阻塞会卡窗口创建——故线程化）
    let app_for_loop = app.clone();
    std::thread::spawn(move || {
        let app = app_for_loop;
        let app_for_popup = app.clone();
        let end = nm_loop::serve(std::io::stdin(), &stdout, move |payload| {
            let result = sp_nmhost::actions::handle(&dispatcher, payload);
            // 任务书交互口径：扩展请求到达且会话锁定 → NM 回 locked 语义 → 弹解锁窗
            if let Err(e) = &result {
                if e.code == "locked" {
                    if let Some(w) = app_for_popup.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
            result
        });
        if let ServeEnd::Protocol(e) = &end {
            eprintln!("sp-desktop nm: protocol error: {e}");
        }
        // 端口关闭/协议错误 → NM 实例退出（托盘实例不经过此路径）
        app.exit(0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::{CredentialFactors, EntryModel, KdfParams, SecureString};
    use sp_nmhost::actions;
    use std::io::Cursor;

    const TEST_DB_PASSWORD: &str = "test-db-pass";
    const TEST_ENTRY_PASSWORD: &str = "nm-embed-secret-value";

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("sp-desktop-nm-{tag}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("mkdir");
            let path = dir.join("test.kdbx");
            let factors = CredentialFactors::new(SecureString::new(TEST_DB_PASSWORD.into()), None);
            sp_kdbx::create_db(&path, &factors, &KdfParams::default()).expect("create_db");
            let mut db = sp_kdbx::open_db(&path, &factors).expect("seed open");
            sp_kdbx::add_entry(
                &mut db,
                &EntryModel {
                    title: "NmDemo".into(),
                    username: Some("nm-user".into()),
                    password: Some(SecureString::new(TEST_ENTRY_PASSWORD.into())),
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

    fn bridge(f: &Fixture) -> DesktopNmSession {
        DesktopNmSession {
            session: Arc::new(Mutex::new(Session::new(Default::default()))),
            keyfile: None,
            db_path: f.0.clone(),
        }
    }

    fn framed(payload: serde_json::Value) -> Vec<u8> {
        let env = sp_nmhost::protocol::Envelope::plain(payload);
        let body = serde_json::to_vec(&env).expect("json");
        let mut out = (body.len() as u32).to_le_bytes().to_vec();
        out.extend_from_slice(&body);
        out
    }

    /// 模拟 NM 握手：把一帧请求喂进 serve 循环（适配器直连），取回响应 payload。
    fn roundtrip(bridge: &DesktopNmSession, request: serde_json::Value) -> serde_json::Value {
        let out: Mutex<Vec<u8>> = Mutex::new(Vec::new());
        let b = bridge;
        let end = nm_loop::serve(Cursor::new(framed(request)), &out, move |p| {
            actions::handle(b, p)
        });
        assert!(matches!(end, ServeEnd::Eof));
        let buf = out
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(!buf.is_empty(), "response frame must be written");
        let mut cursor = Cursor::new(&*buf);
        let frame = sp_nmhost::protocol::read_frame(&mut cursor)
            .expect("frame read")
            .expect("frame present");
        sp_nmhost::protocol::decode_frame(&frame)
            .expect("decode")
            .payload
    }

    #[test]
    fn simulated_handshake_full_fill_flow() {
        // 模拟 NM 握手测试（任务书任务 3 出口）：ping → unlock → get-logins →
        // get-secret → lock → 再取即 locked，全链路走真实 serve 循环与动作分发。
        let f = Fixture::new("handshake");
        let b = bridge(&f);

        let resp = roundtrip(
            &b,
            serde_json::json!({"action": "sp.ping", "requestID": "1"}),
        );
        assert_eq!(resp["data"]["locked"], true, "初始锁定");

        let resp = roundtrip(
            &b,
            serde_json::json!({"action": "sp.unlock", "requestID": "2", "password": "wrong"}),
        );
        assert_eq!(resp["success"], false);
        assert_eq!(resp["error"]["code"], "credentials");

        let resp = roundtrip(
            &b,
            serde_json::json!({"action": "sp.unlock", "requestID": "3", "password": TEST_DB_PASSWORD}),
        );
        assert_eq!(resp["success"], true);
        assert_eq!(resp["data"]["entryCount"], 1);

        let resp = roundtrip(
            &b,
            serde_json::json!({"action": "sp.get-logins", "requestID": "4", "url": "https://login.demo.local/x"}),
        );
        let matches = resp["data"]["matches"].as_array().expect("matches");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["id"], "NmDemo");
        assert_eq!(matches[0]["username"], "nm-user");
        assert!(
            matches[0].get("password").is_none(),
            "get-logins 不得含密码"
        );

        let resp = roundtrip(
            &b,
            serde_json::json!({"action": "sp.get-secret", "requestID": "5", "id": "NmDemo", "field": "password"}),
        );
        assert_eq!(resp["data"]["value"], TEST_ENTRY_PASSWORD);

        roundtrip(
            &b,
            serde_json::json!({"action": "sp.lock", "requestID": "6"}),
        );
        let resp = roundtrip(
            &b,
            serde_json::json!({"action": "sp.get-logins", "requestID": "7", "url": "https://demo.local"}),
        );
        assert_eq!(resp["error"]["code"], "locked", "锁定后机密操作拒绝");
    }

    #[test]
    fn status_reports_desktop_vault_path_and_idle_timeout() {
        let f = Fixture::new("status");
        let b = bridge(&f);
        let resp = roundtrip(
            &b,
            serde_json::json!({"action": "sp.status", "requestID": "1"}),
        );
        assert_eq!(resp["data"]["locked"], true);
        assert_eq!(
            resp["data"]["dbPath"].as_str().expect("dbPath"),
            f.0.to_string_lossy()
        );
        assert_eq!(resp["data"]["sessionTimeoutSecs"], 300);
    }
}
