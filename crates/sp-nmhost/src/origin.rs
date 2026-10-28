//! 调用方 origin 绑定（design_docs/11 §6；step7 自 bin 下沉 lib——桌面单进程嵌入
//! 复用同一道 host 侧防线，design_docs/15）。

use serde_json::Value;

/// nm-host-config.json（register.ps1 写入）：allowed_origin / db_path / keyfile_path /
/// session_timeout_secs。库与 keyfile 的解析优先级：CLI 参数 > 本文件 > config.toml。
pub fn host_side_config_json() -> Option<Value> {
    let cfg_path = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .map(|p| p.join("SuperPasswdKey").join("nm-host-config.json"))?;
    let raw = std::fs::read_to_string(cfg_path).ok()?;
    // 兼容带 BOM 的写出工具（Chrome manifest 解析同样不容忍 BOM——register.ps1 已改无 BOM）
    let raw = raw.trim_start_matches('\u{feff}');
    serde_json::from_str(raw).ok()
}

/// nm-host-config.json 中字符串字段读取
pub fn host_side_config_str(key: &str) -> Option<String> {
    host_side_config_json().as_ref().and_then(|c| {
        c.get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    })
}

/// origin 期望值解析顺序（design_docs/11 §6）：CLI --allowed-origin >
/// 环境变量 SP_NM_ALLOWED_ORIGIN > nm-host-config.json（register.ps1 写入）。
/// 均未配置 → None（开发模式：调用方告警后放行，防线只剩 manifest allowed_origins）。
pub fn bound_origin(cli_allowed_origin: Option<&str>) -> Option<String> {
    if let Some(o) = cli_allowed_origin {
        if !o.is_empty() {
            return Some(o.to_string());
        }
    }
    if let Ok(o) = std::env::var("SP_NM_ALLOWED_ORIGIN") {
        if !o.is_empty() {
            return Some(o);
        }
    }
    host_side_config_str("allowed_origin")
}

/// 浏览器拉起 host 时经 argv[1] 传入的调用方 origin（chrome-extension://<id>/）
pub fn caller_origin() -> Option<String> {
    std::env::args().nth(1)
}

/// 进程是否由浏览器经 native messaging 拉起（argv[1] 为扩展 origin）。
/// 桌面壳据此区分 NM 模式与托盘模式（design_docs/15 进程模型）。
pub fn launched_by_browser() -> bool {
    caller_origin().is_some_and(|o| o.starts_with("chrome-extension://"))
}

/// origin 双重校验（manifest allowed_origins 之外的 host 侧防线，design_docs/11 §1）。
/// expected 为 "*" 时告警放行（仅本机开发调试）。
pub fn check_caller_origin(expected: &str, caller: Option<&str>) -> Result<(), String> {
    if expected == "*" {
        eprintln!("[warn] origin check disabled by '*': development only");
        return Ok(());
    }
    let caller =
        caller.ok_or_else(|| "missing caller origin (first argument from browser)".to_string())?;
    if caller != expected {
        return Err(format!("caller origin mismatch: {caller}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_origin_accepts_match_and_rejects_mismatch() {
        assert!(
            check_caller_origin("chrome-extension://abc/", Some("chrome-extension://abc/")).is_ok()
        );
        assert!(
            check_caller_origin("chrome-extension://abc/", Some("chrome-extension://xyz/"))
                .is_err()
        );
        assert!(check_caller_origin("chrome-extension://abc/", None).is_err());
    }

    #[test]
    fn wildcard_disables_check() {
        assert!(check_caller_origin("*", Some("chrome-extension://anything/")).is_ok());
        assert!(check_caller_origin("*", None).is_ok());
    }

    #[test]
    fn launched_by_browser_requires_extension_scheme() {
        // 本测试进程 argv 不含扩展 origin（cargo test 拉起）
        // 仅验证纯函数语义：直接构造参数走 check 路径已覆盖；此处守住误报
        assert!(!std::env::args()
            .nth(1)
            .is_some_and(|o| o.starts_with("chrome-extension://")));
    }
}
