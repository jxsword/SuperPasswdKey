# native-host：注册与部署

host 二进制由 Rust workspace 构建产出：`cargo build --release -p sp-nmhost` →
`target/release/sp-nmhost.exe`。

## Windows 注册（Chrome / Edge，HKCU 免管理员）

```powershell
powershell -ExecutionPolicy Bypass -File native-host/register.ps1 `
  -HostExe "target\release\sp-nmhost.exe" `
  -ExtensionId "<chrome://extensions 显示的 32 位 ID>"
# 注销
powershell -ExecutionPolicy Bypass -File native-host/register.ps1 -Unregister
```

- manifest 落位 `%LOCALAPPDATA%\SuperPasswdKey\nm-host-manifest.json`，
  `allowed_origins` 绑定扩展 ID（SP-F-701，无通配）；脚本同步写
  `nm-host-config.json`（host 侧 origin 第二道防线，Chrome manifest 不支持启动参数，
  design_docs/11 §6）。
- `sp-nmhost` 的库路径/keyfile：`~/.config/sp/config.toml`（default_db/keyfile）
  或 `--db`/`--keyfile`（手工调试时）；host 启动时读取。
- unpacked 扩展 ID 由加载目录路径派生（SHA-256 前 32 hex 的 a-p 映射）：
  移动扩展目录即变更 ID，需重新注册。
