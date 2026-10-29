# SuperPasswdKey

Local-first, zero-knowledge password/secret manager CLI with KDBX4 interoperability.

本地优先、零知识的密码/机密管理器 CLI，KDBX4 全兼容。

> Status: develop @ 2026-10-30（CLI 全功能内核 + 浏览器扩展 F1（native messaging host + 本填充引擎 + **登录自动保存** SP-F-708）+ 桌面 Windows 壳（DR-013 批次 1：托盘/双超时会话/自启/NM 单进程嵌入）+ Android M1/M2 已验证未发布；服务器轨见 [docs/SRS.md](docs/SRS.md) §10 路线）

## Security model / 安全模型（务必阅读）

- All encryption/decryption happens locally; databases are KDBX4 (Argon2id + AES-256/ChaCha20, HMAC-SHA-256 authenticated blocks). 加解密全部本地完成；库为 KDBX4 格式。
- Your master password and keyfile never leave this process. 主密码与密钥文件永不离开本进程、不上传任何服务器。
- The app performs **zero network I/O** (enforced in CI via dependency bans). 应用零网络请求（CI 依赖禁入强制）。
- **Threat boundary / 威胁边界**（honest limits / 如实声明）：
  - Other processes of the same OS user may read this process's memory (injected env vars included). 同操作系统用户的进程可能读取本进程内存（含注入的环境变量）——这是注入式方案的固有边界。
  - Memory pages are **not locked** (VirtualLock/SetProcessMitigationPolicy/anti-debug from design_docs/09§4 are designed but **not implemented yet** — backlog 2026-10-30 audit); sensitive material lives in process memory while the vault is unlocked — use full-disk encryption (BitLocker) for hibernation/pagefile residue. 内存页锁定/进程加固/反调试**设计有但尚未实现**（2026-10-30 回填核查，列 backlog）；解锁期间敏感材料驻留进程内存——休眠/页面文件残留建议全盘加密。
  - Database plaintext in memory (XML DOM) is not fully zeroized by the format library. 库明文（XML DOM）不被格式库全量零化——与 KeePassXC 同级。
  - This app cannot stop malicious software from reading/uploading/deleting your database file; it provides integrity detection (baseline hash) + automatic backups (rolling 10). 应用无法阻止恶意软件读写/删除库文件；提供完整性检测 + 自动备份（滚动 10 份）。

## Development / 开发

- Requirements: Rust 1.99.1 (via `rust-toolchain.toml`), Node 22 + pnpm 11 (extension only), KeePassXC ≥2.7 (tests only).
- Build: `cargo build --release` ｜ Test: `cargo test` ｜ Lint: `cargo clippy -- -D warnings`
- Extension: `cd extension && pnpm install && pnpm build` ｜ Desktop: `cd desktop/src-tauri && cargo tauri build` ｜ Design docs: [design_docs/](design_docs/) (architecture `00`, crypto scheme `09`, extension `10`-`12`, Android `13`-`14`, desktop `15`, tests `07`, git flow `08`) ｜ CLI 手册: [docs/manual/sp_cli_manual.md](docs/manual/sp_cli_manual.md)

## Android（M1 原型已验证，DR-009/DR-010，未发布）

Rust 内核经 UniFFI（`crates/sp-kffi`）复用于 Android：UniFFI 绑定 + 线程封闭 actor（keepass_ng `Database` 非 Send）+ 最小 Autofill 壳。已验证：模拟器与真机（Redmi Turbo 3/HyperOS）原生路径 Autofill 端到端填充全通；KDF 64MiB Argon2id 真机 2.2–2.5s（HyperOS 夜间降频可达 8–18s，实测须记录电源状态）；移动端 KDF 默认参数定为 init 校准终态（DR-010，过渡 t=4/p=4）。**未提供发布物**；web 字段 autofill 委派受浏览器/ROM 策略限制（R14-1/3，待非 MIUI 真机 + Chrome 复核）。设计：[design_docs/13](design_docs/13_Android_UniFFI.md) ｜ [14](design_docs/14_Android_Autofill.md) ｜ [spike 结论](docs/reviews/android_spike_结论报告_2026-10-27.md) ｜ [真机测试方案与记录](docs/reviews/android_真机安装测试方案_2026-10-27.md) ｜ 构建：[android/README.md](android/README.md)。

## Desktop（Windows 桌面壳，DR-013 批次 1）

`sp-desktop.exe` 同时是**托盘 GUI**（解锁/会话状态/设置/退出）与 **native messaging host**：浏览器经 `connectNative` 拉起的实例与 GUI 共享进程内会话语义（每进程启动首次主密码 + idle/absolute 双超时，DR-013 桌面口径）。

1. 构建：`cd desktop/src-tauri && cargo tauri build`（产物 `target/release/sp-desktop.exe` + NSIS 安装包）
2. 库路径固定 `%APPDATA%\SuperPasswdKey\vault.kdbx`（桌面单库；`sp init --db` 指向它创建）
3. 注册浏览器填充（与 sp-nmhost 双轨、重跑即切换）：`powershell -ExecutionPolicy Bypass -File native-host/register.ps1 -Target desktop -HostExe <sp-desktop.exe> -ExtensionId <扩展ID> [-KeyfilePath <keyfile>]`
4. 开机自启：桌面设置窗勾选（写 HKCU Run，免管理员）；超时/自启配置落 `%APPDATA%\SuperPasswdKey\settings.json`（仅非机密）
5. 锁定态收到填充请求 → 自动弹出解锁窗（每进程一次首次主密码）；浏览器断开该实例即退出

设计：[design_docs/15](design_docs/15_桌面形态.md)（进程模型/机密纪律/供应链差异披露）。

## Browser extension / 浏览器扩展（v0.2.0-ext + SP-F-708）

Chrome/Edge MV3 扩展经 native messaging 驱动本地 host（`sp-nmhost`）完成解锁与填充；加密解密全部本地，扩展端不落任何存储。

1. 构建 host：`cargo build --release -p sp-nmhost`
2. 加载扩展：chrome://extensions → 开发者模式 → 加载 `extension/dist`（或解压发布包内 `extension/` 构建产物）
3. 注册 host（免管理员，HKCU）：
   `powershell -ExecutionPolicy Bypass -File native-host/register.ps1 -HostExe <sp-nmhost.exe> -ExtensionId <chrome://extensions 显示的 32 位 ID> -DbPath <你的 .kdbx>`
4. popup 输入主密码解锁后即可在登录页一键填充（URL 匹配规则与协议见 [design_docs/11](design_docs/11_native_messaging协议.md) / [12](design_docs/12_填充引擎与表单匹配.md)）
5. **登录自动保存（SP-F-708）**：登录表单提交后页内浮层确认「保存/忽略」→ 确认才写库（新凭据新增、同站同用户名更新且旧密码入历史）；host 需为含 `sp.save-login` 的版本（PR #35 之后）
6. 注销：`native-host/unregister.ps1`

威胁边界延伸（如实在声明）：扩展与 host 间为**同用户本机管道**（P1 明文帧，`allowed_origins` 绑定扩展 ID + host 侧 origin 校验双重防线；协议层加密 P2 为预适配位）；同 OS 用户恶意软件的内存读取边界与 CLI 一致。详见 [design_docs/11 §5-6](design_docs/11_native_messaging协议.md)。

## Install / 安装与发布（Windows，v0.3.0 起）

Release 资产（tag `v*` 触发 CI 构建 **draft** Release 并上传；维护者本地 `scripts/sign-release.ps1` 签名后发布——DR-007 私钥不经 CI）：

| 资产 | 形态 | 适用 |
|---|---|---|
| `sp-<tag>-x86_64-pc-windows-msvc.zip` | **免安装包** | 解压 → 运行 `install.ps1`（复制到 `%LOCALAPPDATA%\SuperPasswdKey\bin` + **自动加入用户 PATH**，免管理员）→ 随包含 CLI 手册与 native-host 注册脚本 |
| `sp-cli-<tag>-x64-setup.exe` | **CLI 单安装包** | 向导安装 sp.exe/sp-nmhost.exe/手册 + 可选"加入用户 PATH"勾选 + 控制面板卸载器（仅 HKCU，免管理员） |
| `SuperPasswdKey_<tag>_x64-setup.exe` | **桌面壳安装包** | 托盘 + 会话 + 浏览器填充（NM host）一体 |
| `superpasswdkey-extension-<tag>.zip` | **浏览器扩展** | 商店格式 zip（dist 内容为根）：上架 Chrome Web Store 用；或解压后开发者模式加载 |
| `SHA256SUMS.txt(.minisig)` | 校验/验签 | 见下方验签流程 |

> 所有 exe 经 **CRT 静态链接**（crt-static）构建——自包含、零 VC 运行库依赖。

**首次使用（CLI）**：解压 zip → 运行 `install.ps1` → 重开终端 → `sp init --db "%APPDATA%\SuperPasswdKey\vault.kdbx"` → **立即离线备份 keyfile** → 完整用法见 [docs/manual/sp_cli_manual.md](docs/manual/sp_cli_manual.md)。

**浏览器扩展安装**（平台约束：Chrome 仅允许商店或企业策略两条合规途径，无法技术绕过）：
- 正式用户：扩展上架 Chrome Web Store 后从商店安装（上架动作：开发者账号上传 `superpasswdkey-extension-<tag>.zip`，发布流程见下）；
- 开发/试用：开发者模式加载 `extension/dist`（unpacked ID 随目录路径派生，移动目录需重新注册 NM host）。

## Release verification / 发布验签（v0.2 起）

发布物附 `SHA256SUMS.txt` 与其 minisign 签名 `SHA256SUMS.txt.minisig`（Ed25519，DR-007）：

```bash
# 1) 校验文件哈希
sha256sum -c SHA256SUMS.txt          # Windows: Get-FileHash 对比
# 2) 验证签名（minisign.pub 渠道：本仓库根 / Release 资产 / 维护者 Pages 页 / Web Archive 快照）
minisign -V -p minisign.pub -m SHA256SUMS.txt
```

维护者签名流程见 `scripts/sign-release.ps1`（本地离线签名，私钥不经 CI）。**本仓库签名公钥 minisign.pub 指纹（key ID）：`52113F262DA49B62`**——公钥四渠道分发（本仓库根 / Release 资产 / [jxsword.github.io](https://jxsword.github.io/) / Web Archive 首发快照），若各渠道公钥不一致，以 Pages 页为准并视为安全事故。

## License

AGPL-3.0-only. See [LICENSE](LICENSE).

## Usage / 用法

```bash
# 初始化（Argon2id 校准至 ~700ms + XML keyfile 第二因子）
sp init --db C:\vault\main.kdbx

# 条目管理（密码仅交互输入，无 -p 参数）
sp item create demo -u alice --url https://x.io --field env=prod --totp "otpauth://totp/demo?secret=..."
sp item list
sp item get demo            # 全字段
sp item get demo --totp     # 当前 TOTP 码
sp item delete demo

# 附件（任意文件随库加密：SSH 私钥/证书/文档）
sp item create server-key --attach C:\keys\id_ed25519   # 建条目时附加
sp attachment add server-key C:\keys\id_ed25519 --name id_ed25519   # 给已有条目增补/覆盖
sp attachment list server-key
sp attachment get server-key id_ed25519 --out C:\tmp\key   # 取回（ACL 限当前用户）

# 单值读取（stdout / ACL 限定文件）
sp read demo/Password
sp read demo/Password -n --out-file out.txt

# 剪贴板（默认 30s 自动清除；Win+V 历史不受控）
sp clip demo/Password

# F8 临时环境变量注入（引用文件可安全入版本库；值不落盘、进程退出即销毁）
cat > prod.env << 'EOF'
DB_PASSWORD=sp://demo/Password
API_KEY=sp://demo/env
EOF
sp run --env-file prod.env -- your-command args     # 输出自动遮蔽
sp run --no-masking --env-file prod.env -- printenv DB_PASSWORD
sp inject -i config.tpl -o config.yml               # {{ sp://... }} 渲染，产物限当前用户

# 批量导入（F9：dotenv / csv / bw-json / env 快照 / files 扫描）
sp import dotenv ./secrets.env --dry-run
sp import files D:\projects

# 批量导出（F10：默认 age 加密归档；明文需重输主密码 + stdout）
sp export --out backup.age
sp export --plaintext --stdout
```

## Password input / 密码输入

Interactive prompts hide echo (rpassword). When stdin is not a TTY (scripts/CI),
passwords are read line-by-line from stdin — never pass real secrets through
pipes you don't control. 交互提示隐藏回显；非终端（脚本/CI）从 stdin 按行读取。
