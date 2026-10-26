//! host 侧 config.toml 只读最小解析（与 sp-cli 共享同一文件；不引入 sp-cli 依赖，
//! 避免 clap/arboard 等进入 host 进程——design_docs/10 §2 依赖纪律）。

use std::path::PathBuf;

use serde::Deserialize;
use sp_core::SpError;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HostConfig {
    pub default_db: Option<PathBuf>,
    pub keyfile: Option<PathBuf>,
}

pub fn config_path() -> Result<PathBuf, SpError> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| SpError::Config("no home directory".into()))?;
    Ok(home.join(".config").join("sp").join("config.toml"))
}

pub fn load() -> Result<HostConfig, SpError> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(HostConfig::default());
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| SpError::Config(format!("read config: {e}")))?;
    toml::from_str(&raw).map_err(|e| SpError::Config(format!("parse config: {e}")))
}
