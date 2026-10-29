//! sp-desktop：SuperPasswdKey 桌面壳（DR-013 批次 1，任务书_step7）。
//!
//! 形态：Tauri 托盘常驻 + 解锁窗；NM host 单进程嵌入（任务 3，nm.rs）——浏览器经
//! manifest 拉起本进程时 argv[1] 为扩展 origin，请求循环跑专用线程共享进程内会话。
//! 会话语义见 session.rs（复用 sp-kffi actor + 双超时）；设置持久化见 settings.rs；
//! 开机自启见 autostart.rs。机密纪律：主密码只在 Tauri command 调用栈内存在（传给
//! sp_kffi::open_db 后即弃），前端 JS 不接触明文凭据；设置文件只存非机密。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

mod autostart;
mod nm;
mod session;
mod settings;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};

use session::{describe, Session, SessionStatus};
use settings::SettingsFile;

/// 会话锁（Mutex<Session>；Vault Send+!Sync 由 Mutex 补足 Sync）。
/// Arc 包装供 NM 线程共享同一会话（单进程闭环，nm.rs）。
struct SessionState(Arc<Mutex<Session>>);

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

/// 读取设置文件（损坏/缺失已回落默认值；仅非机密字段）
#[tauri::command]
fn get_settings() -> SettingsFile {
    settings::load_or_default()
}

/// 保存设置：写 settings.json（超时配置重启后生效）+ 即时对账自启注册表值。
#[tauri::command]
fn save_settings(s: SettingsFile) -> Result<SettingsFile, String> {
    settings::save_to(&settings::settings_path(), &s)?;
    let exe = std::env::current_exe()
        .map_err(|e| format!("resolve current exe: {e}"))?
        .to_string_lossy()
        .to_string();
    autostart::reconcile(s.autostart, &exe)?;
    Ok(s)
}

fn main() {
    // 设置先于会话装配：双超时取自 settings.json（损坏回落默认值）
    let cfg = settings::load_or_default();
    // 自启对账：注册表实际状态向用户偏好收敛（失败不阻断启动）
    if let Ok(exe) = std::env::current_exe() {
        if let Err(e) = autostart::reconcile(cfg.autostart, &exe.to_string_lossy()) {
            eprintln!("[warn] autostart reconcile: {e}");
        }
    }
    let session = Arc::new(Mutex::new(Session::new(cfg.session_settings())));
    let nm_mode = sp_nmhost::origin::launched_by_browser();

    tauri::Builder::default()
        .manage(SessionState(Arc::clone(&session)))
        .invoke_handler(tauri::generate_handler![
            session_status,
            unlock,
            lock_now,
            touch,
            get_settings,
            save_settings
        ])
        .setup(move |app| {
            // 窗口策略：托盘实例启动即弹解锁窗（DR-013"每次进程启动首次主密码"）；
            // NM 实例初始隐藏，锁定态请求到达时由 nm.rs 弹出。
            if let Some(w) = app.get_webview_window("main") {
                if nm_mode {
                    let _ = w.hide();
                } else {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }

            // 托盘仅托盘实例创建。NM 实例是服务型进程：解锁窗由 locked 请求拉起、
            // 生命周期由端口关闭（EOF）驱动——不建托盘，避免连接期出现第二组图标。
            // （tauri.conf.json 不再声明 trayIcon：Tauri 2 会按配置自动再建一个
            //   无菜单托盘，与代码托盘叠成"双图标/一个无菜单"——实测翻车，2026-10-29）
            if !nm_mode {
                let show = MenuItem::with_id(app, "show", "显示 / 解锁", true, None::<&str>)?;
                let lock = MenuItem::with_id(app, "lock", "立即锁定", true, None::<&str>)?;
                let conf = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &lock, &conf, &quit])?;

                TrayIconBuilder::with_id("main")
                    .icon(app.default_window_icon().expect("window icon").clone())
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .tooltip("SuperPasswdKey")
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" | "settings" => {
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
                        if matches!(
                            event,
                            TrayIconEvent::Click {
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            }
                        ) {
                            if let Some(w) = tray.app_handle().get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            // NM host 单进程嵌入：仅浏览器拉起（argv[1]=扩展 origin）时启用
            if nm_mode {
                nm::spawn(app.handle().clone(), Arc::clone(&session));
            }
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
