# native-host：注册与部署

host 二进制由 Rust workspace 构建产出，二选一（迁移期双轨，同一时刻注册其一，
重跑脚本即切换；extension 侧无感知）：

- `cargo build --release -p sp-nmhost` → `target/release/sp-nmhost.exe`
  （独立 host 进程，design_docs/11）
- `cargo tauri build`（desktop/src-tauri）→ `desktop/target/release/sp-desktop.exe`
  （托盘 GUI 与 NM host 同进程，DR-013 单进程架构，design_docs/15）

## Windows 注册（Chrome / Edge，HKCU 免管理员）

```powershell
# sp-nmhost 模式（独立 host 进程）
powershell -ExecutionPolicy Bypass -File native-host/register.ps1 `
  -HostExe "target\release\sp-nmhost.exe" `
  -ExtensionId "<chrome://extensions 显示的 32 位 ID>"
# sp-desktop 模式（托盘 + NM 同进程；库路径固定 %APPDATA%\SuperPasswdKey\vault.kdbx，
# 双超时经桌面设置窗配置）
powershell -ExecutionPolicy Bypass -File native-host/register.ps1 `
  -Target desktop `
  -HostExe "desktop\target\release\sp-desktop.exe" `
  -ExtensionId "<chrome://extensions 显示的 32 位 ID>" `
  [-KeyfilePath "<可选 keyfile 路径>"]
# 注销
powershell -ExecutionPolicy Bypass -File native-host/register.ps1 -Unregister
```

- manifest 落位 `%LOCALAPPDATA%\SuperPasswdKey\nm-host-manifest.json`，
  `allowed_origins` 绑定扩展 ID（SP-F-701，无通配）；脚本同步写
  `nm-host-config.json`（host 侧 origin 第二道防线，Chrome manifest 不支持启动参数，
  design_docs/11 §6）。
- `sp-nmhost` 的库路径/keyfile：`~/.config/sp/config.toml`（default_db/keyfile）
  或 `--db`/`--keyfile`（手工调试时）；host 启动时读取。sp-desktop 模式不写
  db_path/session_timeout_secs（归桌面单库与设置窗），keyfile 经 `-KeyfilePath` 写入。
- unpacked 扩展 ID 由加载目录路径派生（SHA-256 前 32 hex 的 a-p 映射）：
  移动扩展目录即变更 ID，需重新注册。
