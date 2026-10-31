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

use session::{describe, EntryDetailDto, EntryDraftDto, EntrySummaryDto, Session, SessionStatus};
use settings::SettingsFile;

/// 会话锁（Mutex<Session>；Vault Send+!Sync 由 Mutex 补足 Sync）。
/// Arc 包装供 NM 线程共享同一会话（单进程闭环，nm.rs）。
struct SessionState(Arc<Mutex<Session>>);

#[tauri::command]
fn session_status(state: State<SessionState>) -> SessionStatus {
    state.0.lock().expect("session poisoned").status()
}

/// 解锁（password/keyfile 即用即弃：仅本次调用栈，传给 FFI 后随栈帧释放）。
/// 成功后记忆 keyfile 路径到 settings.json（2026-10-31 验收反馈：免每次手输；
/// 路径非机密值，design_docs/15 §6 同口径）。
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
    if let Some(kf) = keyfile.as_deref().filter(|s| !s.is_empty()) {
        let mut s = settings::load_or_default();
        s.keyfile_path = kf.to_string();
        if let Err(e) = settings::save_to(&settings::settings_path(), &s) {
            eprintln!("[warn] persist keyfile path: {e}"); // 记忆失败不阻断解锁
        }
    }
    Ok(session.status())
}

/// 编辑态开关（EntryFormModal 打开/关闭）：暂停 idle 超时，absolute 保留。
#[tauri::command]
fn set_editing(state: State<SessionState>, active: bool) {
    state
        .0
        .lock()
        .expect("session poisoned")
        .set_editing(active);
}

/// keyfile 文件浏览（系统打开对话框，Rust 侧调用 tauri-plugin-dialog）；
/// 取消返回空串。
#[tauri::command]
fn keyfile_pick(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(target) = app
        .dialog()
        .file()
        .add_filter("Keyfile", &["keyfile", "keyx", "*"])
        .blocking_pick_file()
    else {
        return Ok(String::new());
    };
    match target {
        tauri_plugin_dialog::FilePath::Path(p) => Ok(p.to_string_lossy().to_string()),
        other => Err(format!("unsupported dialog target: {other:?}")),
    }
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

/// 读取设置文件（损坏/缺失已回落默认值；仅非机密字段）。
/// keyfile_path 指向的文件不存在时返回空串（本次不预填；配置保留——
/// U 盘 keyfile 临时拔出后恢复即复用）。
#[tauri::command]
fn get_settings() -> SettingsFile {
    let mut s = settings::load_or_default();
    if !s.keyfile_path.is_empty() && !std::path::Path::new(&s.keyfile_path).exists() {
        s.keyfile_path = String::new();
    }
    s
}

/// 保存设置：写 settings.json（超时配置重启后生效）+ 即时对账自启注册表值。
/// keyfile_path 由 unlock 流程维护——设置面板不持有该字段，落盘前强制保留现值。
#[tauri::command]
fn save_settings(s: SettingsFile) -> Result<SettingsFile, String> {
    let mut s = s;
    s.keyfile_path = settings::load_from(&settings::settings_path()).keyfile_path;
    settings::save_to(&settings::settings_path(), &s)?;
    let exe = std::env::current_exe()
        .map_err(|e| format!("resolve current exe: {e}"))?
        .to_string_lossy()
        .to_string();
    autostart::reconcile(s.autostart, &exe)?;
    Ok(s)
}

// ---------- GUI 条目浏览（S1a MVP；design_docs/16 §4 command 面映射） ----------

/// 条目摘要列表（惰性超时检查；锁定态返回 locked 语义供前端提示重新解锁）
#[tauri::command]
fn entry_list(state: State<SessionState>) -> Result<Vec<EntrySummaryDto>, String> {
    state.0.lock().expect("session poisoned").entry_list()
}

/// 条目详情（含密码——仅详情视图瞬态展示，不落前端存储，SP-F-707 口径）
#[tauri::command]
fn entry_get(state: State<SessionState>, title: String) -> Result<EntryDetailDto, String> {
    state.0.lock().expect("session poisoned").entry_get(&title)
}

/// 条目当前 TOTP 码（RFC 6238 本地生成）
#[tauri::command]
fn entry_totp(state: State<SessionState>, title: String) -> Result<String, String> {
    state.0.lock().expect("session poisoned").entry_totp(&title)
}

/// 新建条目（复用 Vault::add_entry + save_session 零凭据保存；同名允许，对齐 CLI 口径）
#[tauri::command]
fn entry_create(state: State<SessionState>, draft: EntryDraftDto) -> Result<(), String> {
    state
        .0
        .lock()
        .expect("session poisoned")
        .entry_create(draft)
}

/// 编辑条目（按 old_title 定位，标题可经 draft 改写；旧快照进 history，SP-F-103 单源）
#[tauri::command]
fn entry_edit(
    state: State<SessionState>,
    old_title: String,
    draft: EntryDraftDto,
) -> Result<(), String> {
    state
        .0
        .lock()
        .expect("session poisoned")
        .entry_edit(&old_title, draft)
}

/// 删除条目（复用 Vault::delete_entry + save_session）
#[tauri::command]
fn entry_delete(state: State<SessionState>, title: String) -> Result<(), String> {
    state
        .0
        .lock()
        .expect("session poisoned")
        .entry_delete(&title)
}

/// 列条目附件名（S1b-2b）
#[tauri::command]
fn attachment_list(state: State<SessionState>, title: String) -> Result<Vec<String>, String> {
    state
        .0
        .lock()
        .expect("session poisoned")
        .attachment_list(&title)
}

/// 添加附件（同名覆盖，语义对齐 CLI sp attachment add；保存经 save_session 零凭据）
#[tauri::command]
fn attachment_add(
    state: State<SessionState>,
    title: String,
    name: String,
    data: Vec<u8>,
) -> Result<(), String> {
    state
        .0
        .lock()
        .expect("session poisoned")
        .attachment_add(&title, &name, data)
}

/// 下载附件：弹出系统保存对话框 → Rust 落盘 + 限权收敛（icacls/0600，
/// 复用 sp-core::fsperms 单源——与 CLI attachment_get 同语义）。返回落盘路径；
/// 用户取消返回空串。
#[tauri::command]
fn attachment_download(
    app: tauri::AppHandle,
    state: State<SessionState>,
    title: String,
    name: String,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(target) = app
        .dialog()
        .file()
        .set_file_name(&name)
        .blocking_save_file()
    else {
        return Ok(String::new());
    };
    let path = match target {
        tauri_plugin_dialog::FilePath::Path(p) => p,
        other => return Err(format!("unsupported dialog target: {other:?}")),
    };
    let data = state
        .0
        .lock()
        .expect("session poisoned")
        .attachment_get(&title, &name)?;
    std::fs::write(&path, &data).map_err(|e| format!("write '{}': {e}", path.display()))?;
    sp_core::fsperms::restrict(&path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// 复制到剪贴板（arboard 写入 + 30s 超时清空，对齐 CLI sp clip 语义；
/// 后台线程清空不阻塞 UI；Win+V 历史不受 sp 控制——提示文案与 CLI 一致）
#[tauri::command]
fn clip_copy(text: String) -> Result<(), String> {
    let mut ctx = arboard::Clipboard::new().map_err(|e| format!("初始化剪贴板失败：{e}"))?;
    ctx.set_text(text)
        .map_err(|e| format!("写入剪贴板失败：{e}"))?;
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(30));
        if let Ok(mut c) = arboard::Clipboard::new() {
            let _ = c.set_text(String::new());
        }
    });
    Ok(())
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            session_status,
            unlock,
            lock_now,
            touch,
            set_editing,
            keyfile_pick,
            get_settings,
            save_settings,
            entry_list,
            entry_get,
            entry_totp,
            entry_create,
            entry_edit,
            entry_delete,
            attachment_list,
            attachment_add,
            attachment_download,
            clip_copy
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
