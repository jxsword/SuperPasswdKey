//! sp-nmhost：浏览器扩展 native messaging host（F1，SP-F-701/702/703）。
//!
//! 生命周期：浏览器 connectNative 拉起 → 校验调用方 origin → 请求循环（stdin/stdout
//! 长度前缀 JSON）→ stdin EOF（端口关闭）退出。空闲超时由后台线程主动锁库并通知扩展。
//!
//! 用法：sp-nmhost --allowed-origin chrome-extension://<扩展ID>/ [--db <路径>]
//!           [--keyfile <路径>] [--session-timeout <秒>]
//! dev 场景：--allowed-origin '*' 关闭校验（stderr 告警，仅本机调试）。

use std::io::BufReader;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use sp_nmhost::actions;
use sp_nmhost::config::{self, HostConfig};
use sp_nmhost::protocol::{decode_frame, encode_envelope, read_frame, write_frame, Envelope};
use sp_nmhost::session::Session;

struct Args {
    db: Option<std::path::PathBuf>,
    keyfile: Option<std::path::PathBuf>,
    session_timeout: u64,
    allowed_origin: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        db: None,
        keyfile: None,
        session_timeout: sp_nmhost::actions::default_session_timeout_secs(),
        allowed_origin: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--db" => args.db = it.next().map(std::path::PathBuf::from),
            "--keyfile" => args.keyfile = it.next().map(std::path::PathBuf::from),
            "--session-timeout" => {
                args.session_timeout = it
                    .next()
                    .ok_or("--session-timeout requires a value")?
                    .parse()
                    .map_err(|_| "--session-timeout must be an integer (seconds)")?;
            }
            "--allowed-origin" => {
                args.allowed_origin = Some(it.next().ok_or("--allowed-origin requires a value")?);
            }
            // 浏览器传入的首参（chrome-extension://<id>/）不是本程序 flag，跳过
            _ => {}
        }
    }
    Ok(args)
}

/// origin 期望值解析顺序（design_docs/11 §6）：--allowed-origin >
/// 环境变量 SP_NM_ALLOWED_ORIGIN > %LOCALAPPDATA%\SuperPasswdKey\nm-host-config.json
/// （register.ps1 写入）。均未配置 → None（开发模式：stderr 告警后放行，
/// 调用方限制完全依赖 manifest allowed_origins）。
fn resolve_expected_origin(args: &Args) -> Option<String> {
    if let Some(o) = &args.allowed_origin {
        return Some(o.clone());
    }
    if let Ok(o) = std::env::var("SP_NM_ALLOWED_ORIGIN") {
        if !o.is_empty() {
            return Some(o);
        }
    }
    host_side_config().as_ref().and_then(|c| {
        c.get("allowed_origin")
            .and_then(serde_json::Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    })
}

/// nm-host-config.json（register.ps1 写入，design_docs/11 §6）：
/// allowed_origin / db_path / keyfile_path / session_timeout_secs。
/// 库与 keyfile 的优先级：--db/--keyfile > 本文件 > config.toml（与 sp 语义兼容）。
fn host_side_config() -> Option<serde_json::Value> {
    let cfg_path = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .map(|p| p.join("SuperPasswdKey").join("nm-host-config.json"))?;
    let raw = std::fs::read_to_string(cfg_path).ok()?;
    // 兼容带 BOM 的写出工具（Chrome manifest 解析同样不容忍 BOM——register.ps1 已改无 BOM）
    let raw = raw.trim_start_matches('\u{feff}');
    serde_json::from_str(raw).ok()
}

/// origin 双重校验（manifest allowed_origins 之外的 host 侧防线，design_docs/11 §1）
fn check_caller_origin(expected: &str) -> Result<(), String> {
    if expected == "*" {
        eprintln!("[warn] origin check disabled by '*': development only");
        return Ok(());
    }
    let caller = std::env::args()
        .nth(1)
        .ok_or_else(|| "missing caller origin (first argument from browser)".to_string())?;
    if caller != expected {
        return Err(format!("caller origin mismatch: {caller}"));
    }
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("sp-nmhost: {e}");
            return ExitCode::from(4);
        }
    };
    let expected_origin = resolve_expected_origin(&args);
    match expected_origin.as_deref() {
        Some(o) => {
            if let Err(e) = check_caller_origin(o) {
                eprintln!("sp-nmhost: {e}");
                return ExitCode::from(4);
            }
        }
        None => {
            eprintln!(
                "[warn] no bound origin configured (nm-host-config.json/--allowed-origin); \
                 relying on manifest allowed_origins only"
            );
        }
    }

    // 库与 keyfile 解析：CLI 参数 > nm-host-config.json > config.toml（design_docs/11 §6）
    let cfg: HostConfig = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sp-nmhost: {e}");
            return ExitCode::from(1);
        }
    };
    let host_cfg = host_side_config();
    let json_path = |key: &str| -> Option<std::path::PathBuf> {
        host_cfg
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(serde_json::Value::as_str)
            .filter(|s| !s.is_empty())
            .map(std::path::PathBuf::from)
    };
    let db_path = args.db.or_else(|| json_path("db_path")).or(cfg.default_db);
    let keyfile = args
        .keyfile
        .or_else(|| json_path("keyfile_path"))
        .or(cfg.keyfile);
    if db_path.is_none() {
        eprintln!("sp-nmhost: no database configured (--db or config.toml default_db)");
        return ExitCode::from(1);
    }

    let session = Arc::new(Session::new(args.session_timeout, keyfile));
    session.set_db_path(db_path.expect("checked above"));

    // stdout 供请求循环与空闲锁库事件线程共用
    let stdout = Arc::new(std::sync::Mutex::new(std::io::stdout()));

    // 空闲超时 watcher：主动锁库 + 通知扩展（design_docs/11 §7）
    {
        let session = Arc::clone(&session);
        let stdout = Arc::clone(&stdout);
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            if !session.is_locked() && session.idle_expired() {
                session.lock();
                let env = Envelope::plain(json!({
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
        });
    }

    let mut stdin = BufReader::new(std::io::stdin());
    loop {
        let frame = match read_frame(&mut stdin) {
            Ok(Some(f)) => f,
            Ok(None) => break, // 端口关闭
            Err(e) => {
                eprintln!("sp-nmhost: protocol error: {e}");
                break;
            }
        };
        let env = match decode_frame(&frame) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("sp-nmhost: bad frame: {e}");
                break;
            }
        };
        let request_id = env.payload.get("requestID").cloned().unwrap_or(Value::Null);
        let action = env
            .payload
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        let response = match actions::handle(&session, &env.payload) {
            Ok(data) => json!({
                "action": action,
                "requestID": request_id,
                "success": true,
                "data": data,
            }),
            Err(e) => json!({
                "action": action,
                "requestID": request_id,
                "success": false,
                "error": {"code": e.code, "message": e.message},
            }),
        };
        let bytes = match encode_envelope(&Envelope::plain(response)) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("sp-nmhost: encode response: {e}");
                break;
            }
        };
        let mut w = stdout
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if write_frame(&mut *w, &bytes).is_err() {
            break;
        }
    }
    // 进程退出即零化（Session Drop：SecVec 语义；design_docs/03 §3 Zeroizing 迁移）
    ExitCode::SUCCESS
}
