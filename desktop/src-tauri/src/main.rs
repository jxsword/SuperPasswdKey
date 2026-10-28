//! sp-desktop：SuperPasswdKey 桌面壳（DR-013 批次 1，任务书_step7）。
//!
//! 形态：Tauri 托盘常驻 + 解锁窗（后续增量：NM host 单进程嵌入 / HKCU 自启 / 设置持久化）。
//! 会话语义见 session.rs（复用 sp-kffi actor + 双超时）。机密纪律：主密码只在
//! Tauri command 调用栈内存在（传给 sp_kffi::open_db 后即弃），前端 JS 不接触明文凭据。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

mod session;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};

use session::{describe, Session, SessionStatus};

/// 会话锁（Mutex<Session>；Vault Send+!Sync 由 Mutex 补足 Sync）
struct SessionState(Mutex<Session>);

#[tauri::command]
fn session_status(state: State<SessionState>) -> SessionStatus {
    state.0.lock().expect("session poisoned").status()
}

/// 解锁（password/keyfile 即用即弃：仅本次调用栈，传给 FFI 后随栈帧释放）
#[tauri::command]
fn unlock(
    state: State<SessionState>,
    password: String,
    keyfile: Option<String>,
) -> Result<SessionStatus, String> {
    let mut session = state.0.lock().expect("session poisoned");
    let db_path = Session::default_vault_path();
    session
        .unlock(&db_path, &password, keyfile.as_deref())
        .map_err(|e| describe(&e))?;
    Ok(session.status())
}

#[tauri::command]
fn lock_now(state: State<SessionState>) -> SessionStatus {
    let mut session = state.0.lock().expect("session poisoned");
    session.lock_now();
    session.status()
}

#[tauri::command]
fn touch(state: State<SessionState>) {
    state.0.lock().expect("session poisoned").touch();
}

fn main() {
    tauri::Builder::default()
        .manage(SessionState(Mutex::new(Session::new(Default::default()))))
        .invoke_handler(tauri::generate_handler![
            session_status,
            unlock,
            lock_now,
            touch
        ])
        .setup(|app| {
            // 托盘菜单：显示/解锁、立即锁定、退出（自启/设置项为任务 2 增量）
            let show = MenuItem::with_id(app, "show", "显示 / 解锁", true, None::<&str>)?;
            let lock = MenuItem::with_id(app, "lock", "立即锁定", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &lock, &quit])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("window icon").clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("SuperPasswdKey")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "lock" => {
                        if let Some(state) = app.try_state::<SessionState>() {
                            state.0.lock().expect("session poisoned").lock_now();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    // 左键单击托盘 = 唤出解锁/状态窗
                    if matches!(event, TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }) {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // 托盘应用语义：关窗 = 隐藏（进程与会话保留，超时策略照常生效）
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running sp-desktop");
}
