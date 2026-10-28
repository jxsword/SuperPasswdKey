//! 开机自启（任务书_step7 任务 2，design_docs/15）：
//! HKCU\Software\Microsoft\Windows\CurrentVersion\Run 写/删用户级注册表值
//! （免管理员；手写 registry，少一个依赖面——任务书选型）。
//! 值名 SuperPasswdKey；值 = 带引号的可执行文件全路径。
//! settings.json 的 autostart 字段为用户偏好单一来源，启动时按其对账。

use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const AUTOSTART_VALUE_NAME: &str = "SuperPasswdKey";

/// 查询自启注册表值；未设置 → None（value_name 可注入以便测试隔离）
pub fn query_named(value_name: &str) -> Result<Option<String>, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(RUN_KEY)
        .map_err(|e| format!("open Run key: {e}"))?;
    match key.get_value::<String, _>(value_name) {
        Ok(v) => Ok(Some(v)),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("read Run value: {e}")),
    }
}

/// 写/删自启注册表值；enable=false 且值不存在时幂等成功（对账语义依赖此点）
pub fn set_named(value_name: &str, enable: bool, exe_path: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE)
        .map_err(|e| format!("open Run key: {e}"))?;
    if enable {
        key.set_value(value_name, &format!("\"{exe_path}\""))
            .map_err(|e| format!("set Run value: {e}"))
    } else {
        match key.delete_value(value_name) {
            Ok(()) => Ok(()),
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("delete Run value: {e}")),
        }
    }
}

/// 生产口径（值名固定 SuperPasswdKey）
pub fn set(enable: bool, exe_path: &str) -> Result<(), String> {
    set_named(AUTOSTART_VALUE_NAME, enable, exe_path)
}

/// 读取面：设置对账与注册表验证（reg query 口径的程序内等价；当前仅测试消费）
#[allow(dead_code)]
pub fn query() -> Result<Option<String>, String> {
    query_named(AUTOSTART_VALUE_NAME)
}

/// 启动时对账：注册表实际状态向 settings.json 偏好收敛（用户手删注册表值后
/// 重启即恢复；关掉偏好则清残留）。失败只报错不阻断启动（自启非关键路径）。
pub fn reconcile(enable: bool, exe_path: &str) -> Result<(), String> {
    set(enable, exe_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_VALUE_NAME: &str = "SuperPasswdKeySelfTest";

    /// 用独立值名走真实 API roundtrip（写 → 查 → 幂等删 → 查），结束清场。
    /// 仅写 HKCU 当前用户 Run 键的测试值，不触碰生产值名，无系统面影响。
    #[test]
    fn registry_roundtrip_with_dedicated_value_name() {
        let exe = r"C:\Program Files\SuperPasswdKey\sp-desktop.exe";
        set_named(TEST_VALUE_NAME, true, exe).expect("enable");
        let got = query_named(TEST_VALUE_NAME).expect("query");
        assert_eq!(got.as_deref(), Some(format!("\"{exe}\"").as_str()));

        set_named(TEST_VALUE_NAME, false, exe).expect("disable");
        assert_eq!(query_named(TEST_VALUE_NAME).expect("query"), None);
    }

    #[test]
    fn set_false_without_existing_value_is_ok() {
        set_named(TEST_VALUE_NAME, false, r"C:\anywhere.exe").expect("idempotent delete");
        assert_eq!(query_named(TEST_VALUE_NAME).expect("query"), None);
    }
}
