# SuperPasswdKey

Local-first, zero-knowledge password/secret manager CLI with KDBX4 interoperability.

本地优先、零知识的密码/机密管理器 CLI，KDBX4 全兼容。

> Status: v0.2.0-ext（CLI 全功能内核 + 浏览器扩展 F1：native messaging host + 本填充引擎；服务器轨见 [docs/SRS.md](docs/SRS.md) §10 路线）

## Security model / 安全模型（务必阅读）

- All encryption/decryption happens locally; databases are KDBX4 (Argon2id + AES-256/ChaCha20, HMAC-SHA-256 authenticated blocks). 加解密全部本地完成；库为 KDBX4 格式。
- Your master password and keyfile never leave this process. 主密码与密钥文件永不离开本进程、不上传任何服务器。
- The app performs **zero network I/O** (enforced in CI via dependency bans). 应用零网络请求（CI 依赖禁入强制）。
- **Threat boundary / 威胁边界**（honest limits / 如实声明）：
  - Other processes of the same OS user may read this process's memory (injected env vars included). 同操作系统用户的进程可能读取本进程内存（含注入的环境变量）——这是注入式方案的固有边界。
  - Keyfile-protected memory pages are locked (`VirtualLock`) but paging under memory pressure is best-effort; use full-disk encryption (BitLocker) for hibernation/pagefile residue. 内存页锁定为最佳努力；休眠/页面文件残留建议全盘加密。
  - Database plaintext in memory (XML DOM) is not fully zeroized by the format library. 库明文（XML DOM）不被格式库全量零化——与 KeePassXC 同级。
  - This app cannot stop malicious software from reading/uploading/deleting your database file; it provides integrity detection (baseline hash) + automatic backups (rolling 10). 应用无法阻止恶意软件读写/删除库文件；提供完整性检测 + 自动备份（滚动 10 份）。

## Development / 开发

- Requirements: Rust 1.99.1 (via `rust-toolchain.toml`), Node 22 + pnpm 11 (extension only), KeePassXC ≥2.7 (tests only).
- Build: `cargo build --release` ｜ Test: `cargo test` ｜ Lint: `cargo clippy -- -D warnings`
- Extension: `cd extension && pnpm install && pnpm build` ｜ Design docs: [design_docs/](design_docs/) (architecture, crypto scheme `09`, extension `10`-`12`, tests `07`, git flow `08`)

## Browser extension / 浏览器扩展（v0.2.0-ext）

Chrome/Edge MV3 扩展经 native messaging 驱动本地 host（`sp-nmhost`）完成解锁与填充；加密解密全部本地，扩展端不落任何存储。

1. 构建 host：`cargo build --release -p sp-nmhost`
2. 加载扩展：chrome://extensions → 开发者模式 → 加载 `extension/dist`（或解压发布包内 `extension/` 构建产物）
3. 注册 host（免管理员，HKCU）：
   `powershell -ExecutionPolicy Bypass -File native-host/register.ps1 -HostExe <sp-nmhost.exe> -ExtensionId <chrome://extensions 显示的 32 位 ID> -DbPath <你的 .kdbx>`
4. popup 输入主密码解锁后即可在登录页一键填充（URL 匹配规则与协议见 [design_docs/11](design_docs/11_native_messaging协议.md) / [12](design_docs/12_填充引擎与表单匹配.md)）
5. 注销：`native-host/unregister.ps1`

威胁边界延伸（如实在声明）：扩展与 host 间为**同用户本机管道**（P1 明文帧，`allowed_origins` 绑定扩展 ID + host 侧 origin 校验双重防线；协议层加密 P2 为预适配位）；同 OS 用户恶意软件的内存读取边界与 CLI 一致。详见 [design_docs/11 §5-6](design_docs/11_native_messaging协议.md)。

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
sp init --db C:\Dataault.kdbx

# 条目管理
sp item create demo -u alice --url https://x.io --field env=prod --totp "otpauth://totp/demo?secret=..." -p
sp item list
sp item get demo            # 全字段
sp item get demo --totp     # 当前 TOTP 码
sp item delete demo

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
