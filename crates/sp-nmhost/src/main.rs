//! sp-nmhost：浏览器扩展 native messaging host（F1，SP-F-701/702/703）。
//!
//! 生命周期：浏览器 connectNative 拉起 → 校验调用方 origin → 请求循环（stdin/stdout
//! 长度前缀 JSON）→ stdin EOF（端口关闭）退出。空闲超时由后台线程主动锁库并通知扩展。
//!
//! 用法：sp-nmhost --allowed-origin chrome-extension://<扩展ID>/ [--db <路径>]
//!           [--keyfile <路径>] [--session-timeout <秒>]
//! dev 场景：--allowed-origin '*' 关闭校验（stderr 告警，仅本机调试）。
//!
//! step7 起：协议编解码/动作分发/请求循环/origin 校验均在库目标（sp_nmhost::），
//! bin 仅保留参数解析与装配（sp-desktop 单进程嵌入复用同一库面，design_docs/15）。

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use sp_nmhost::actions;
use sp_nmhost::config::{self, HostConfig};
use sp_nmhost::nm_loop::{self, ServeEnd};
use sp_nmhost::origin;
use sp_nmhost::protocol::{encode_envelope, write_frame, Envelope};
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

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("sp-nmhost: {e}");
            return ExitCode::from(4);
        }
    };
    let expected_origin = origin::bound_origin(args.allowed_origin.as_deref());
    match expected_origin.as_deref() {
        Some(o) => {
            if let Err(e) = origin::check_caller_origin(o, origin::caller_origin().as_deref()) {
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
    let db_path = args
        .db
        .or_else(|| origin::host_side_config_str("db_path").map(std::path::PathBuf::from))
        .or(cfg.default_db);
    let keyfile = args
        .keyfile
        .or_else(|| origin::host_side_config_str("keyfile_path").map(std::path::PathBuf::from))
        .or(cfg.keyfile);
    if db_path.is_none() {
        eprintln!("sp-nmhost: no database configured (--db or config.toml default_db)");
        return ExitCode::from(1);
    }

    let session = Arc::new(Session::new(args.session_timeout, keyfile));
    session.set_db_path(db_path.expect("checked above"));

    // stdout 供请求循环与空闲锁库事件线程共用（serve 按帧持锁，不交错）
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

    let session_for_dispatch = Arc::clone(&session);
    let end = nm_loop::serve(std::io::stdin(), &stdout, move |payload| {
        actions::handle(session_for_dispatch.as_ref(), payload)
    });
    if let ServeEnd::Protocol(e) = end {
        eprintln!("sp-nmhost: protocol error: {e}");
    }
    // 进程退出即零化（Session Drop：SecVec 语义；design_docs/03 §3 Zeroizing 迁移）
    ExitCode::SUCCESS
}
