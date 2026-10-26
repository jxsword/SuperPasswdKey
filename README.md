# SuperPasswdKey

Local-first, zero-knowledge password/secret manager CLI with KDBX4 interoperability.

本地优先、零知识的密码/机密管理器 CLI，KDBX4 全兼容。

> Status: v0.1.0-mvp（CLI 全功能内核；F8 run/inject 编排接线、浏览器扩展、服务器轨见 [docs/SRS.md](docs/SRS.md) §10 路线）

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

- Requirements: Rust 1.99.1 (via `rust-toolchain.toml`), KeePassXC ≥2.7 (tests only).
- Build: `cargo build --release` ｜ Test: `cargo test` ｜ Lint: `cargo clippy -- -D warnings`
- Design docs: [design_docs/](design_docs/) (architecture, crypto scheme `09`, tests `07`, git flow `08`)

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
