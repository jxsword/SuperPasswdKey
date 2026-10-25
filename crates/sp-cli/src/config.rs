//! 配置文件（SP-F-106）：`%USERPROFILE%\.config\sp\config.toml`
//! 不含任何密钥材料（红线）；完整性基线哈希记录于此（SP-N-311）。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use sp_core::SpError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_db: Option<PathBuf>,
    pub keyfile: Option<PathBuf>,
    #[serde(default = "default_clip_timeout")]
    pub clip_timeout_secs: u64,
    /// 上次成功打开后的库 SHA256（SP-N-311 变更检测基线）
    pub baseline_sha256: Option<String>,
}

fn default_clip_timeout() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_db: None,
            keyfile: None,
            clip_timeout_secs: default_clip_timeout(),
            baseline_sha256: None,
        }
    }
}

pub fn config_path() -> Result<PathBuf, SpError> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| SpError::Config("no home directory".into()))?;
    Ok(home.join(".config").join("sp").join("config.toml"))
}

pub fn load() -> Result<Config, SpError> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| SpError::Config(format!("read config: {e}")))?;
    toml::from_str(&raw).map_err(|e| SpError::Config(format!("parse config: {e}")))
}

pub fn save(cfg: &Config) -> Result<(), SpError> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| SpError::Config(format!("create config dir: {e}")))?;
    }
    let raw =
        toml::to_string_pretty(cfg).map_err(|e| SpError::Config(format!("serialize: {e}")))?;
    std::fs::write(&path, raw).map_err(|e| SpError::Config(format!("write config: {e}")))
}
