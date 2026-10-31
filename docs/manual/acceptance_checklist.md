# 验收测试清单（GUI + CLI）— v1.2（2026-10-31）

> 用途：**逐项执行**的验收清单，覆盖 S1a/S1b 全部用户可见功能（SP-F-901/902/903/904/905/908/909/910/601/108）与 F8 注入（SP-F-301/3xx）。
> 用法：按序执行，在每行"实际结果"列填 ✅/❌；❌ 请附截图与复现步骤。命令与预期输出以 `sp_cli_manual.md` 为详注；若程序输出与手册不一致，以程序输出为准并如实记录。
> **Shell 口径（v1.1 修订）**：全部命令为 **PowerShell** 语法，路径变量统一 `$SP` / `$TESTDB` / `$VDB`（cmd 的 `%VAR%` 写法不再使用；A10 中子进程 `cmd /c echo %GH_TOKEN%` 的 `%` 属 cmd 变量展开，非本清单变量）。每节开头的变量赋值执行一次即可。
> 上游：design_docs/16 §8（启动指引与协作原理图）、S1a/S1b 交付说明（docs/plans/）。

---

## 0. 前置准备（必读）

### 0.1 ⚠️ 必须重建（旧版不含本轮功能）

PATH 中的 `sp.exe`（`E:\rust\cargo\bin`）与已安装的桌面版均为 **v0.3.0**，**不含 `--tag`、参数前置校验与 GUI 写路径（创建/编辑/删除/附件/放大显示等）**。测试前必须重建：

```powershell
# 仓库根目录执行
cargo build --release --workspace          # CLI：产出 target\release\sp.exe
cd desktop/src-tauri
cargo tauri build                          # GUI：产出 target\release\sp-desktop.exe + NSIS 安装包
```

### 0.2 变量准备（复制后续命令前先执行）

```powershell
$SP     = "E:\proj\SuperPasswdKey\target\release\sp.exe"   # 新构建的 CLI（全路径，规避同名撞车）
$TESTDB = "C:\vault\sp-test.kdbx"                          # CLI 测试库（与主库隔离）
$VDB    = "$env:APPDATA\SuperPasswdKey\vault.kdbx"         # GUI 主库（GUI 锁定单库，无法换库）
```

### 0.3 GUI 程序位置与启动方式

| 方式 | 位置 / 命令 | 说明 |
|---|---|---|
| **直接运行（本轮推荐）** | `desktop\src-tauri\target\release\sp-desktop.exe` | 双击或命令行启动；弹解锁窗 + 托盘常驻；关窗=隐藏 |
| NSIS 安装包 | `desktop\src-tauri\target\release\bundle\nsis\SuperPasswdKey_0.3.0_x64-setup.exe` | 安装后从开始菜单"SuperPasswdKey"启动 |
| 开发模式（排查问题用） | `cd desktop/src-tauri && cargo tauri dev` | Vite dev server 自动拉起；可看前端 console；Rust 改动触发重编 |
| 开机自启 | GUI 顶栏"设置"→ 勾选 | 写 HKCU Run（免管理员） |
| NM 实例 | 不手动启动 | 浏览器扩展 connectNative 经 manifest 拉起，随断开自动退出 |

### 0.4 同名撞车核对（sp.exe）

第三方程序可能与本 CLI 同名（已知案例：**win-pyvideotrans v4.17 项目的 GUI 应用亦为 sp.exe**）。开始前先核对：

```powershell
where.exe sp     # 列出 PATH 中全部同名命中（按优先级排序）
& $SP --version  # 新构建 CLI 的版本号
```

本轮命令统一用 `& $SP` 全路径调用（不受 PATH 顺序影响）；长期处置见 CLI 手册 §1（PATH 调序，推荐 `scripts/add-path.ps1`）。

### 0.5 测试库隔离与备份

- **CLI 部分**：独立测试库 `$TESTDB`（`--db` 指定），不碰主库。
- **GUI 部分**：桌面 GUI 锁定单库 `$VDB`——**测试前先备份**：

```powershell
Copy-Item $VDB "$VDB.bak-acceptance"
```

（测试完成后可用备份还原；注意备份的是 KDBX 文件本身，keyfile 如有也需一并备份。）

### 0.6 剪贴板验证环境

准备一个可粘贴的记事本/输入框，用于 B5 组（复制）与 CLI clip 的验证。

---

## A. CLI 逐项清单

> 通用约定：交互提示（`Enter password to unlock ...`、`Entry password (empty to skip):`）走 stderr，stdout 是干净数据通道；
> 测试主密码建议用一次性口令（如 `sp-acceptance-only`），不入任何真实库；条目密码同理用一次性测试值。

### A0. 环境

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A0.1 | `where.exe sp` | 列出 PATH 中全部同名命中（含本 CLI 与/或第三方 sp.exe，按优先级排序） | 手册 §1 撞名注记 | |
| A0.2 | `& $SP --version` | 输出版本号（应为当前构建版本） | — | |

### A1. init（手册 §3）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A1.1 | `& $SP init --db $TESTDB`（交互） | 依次提示 `Choose a master password:` → `Confirm master password:`（两次输入不一致会要求重输）；成功后打印 `REMINDER: back up the keyfile to a separate medium (USB/another device).` 与 `KDF: Argon2id m=… t=… p=…`（校准参数） | SP-F-101 | |
| A1.2 | 检查 `$TESTDB` 同目录 | 生成 `sp-test.keyfile`（init 自动建双因子） | SP-F-101 | |
| A1.3 | 再次执行同命令 | 报错退出码 **4**（目标库已存在） | 手册 §3 注 | |

### A2. item create（手册 §4.1）

> 以下均需先按提示输入测试主密码解锁，再按提示输入条目密码（可留空）。
> **参数校验在解锁之前**（v1.1）：A2.3/A2.4 这类坏参数**不提示主密码、不进 KDF** 直接退出 4。

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A2.1 | `& $SP item create --db $TESTDB github -u alice --url https://github.com` | 成功打印 `Created entry 'github'`（解锁提示 → 条目密码提示可留空） | SP-F-102 | |
| A2.2 | `& $SP item create --db $TESTDB work-vpn -u bob --totp "otpauth://totp/Test?secret=JBSWY3DPEHPK3PXP&issuer=Test" --tag work --tag infra` | 成功；单条目携带两个标签 | SP-F-102/108 | |
| A2.3 | `& $SP item create --db $TESTDB bad-otp --totp "not-otpauth"` | **不提示主密码**（无 `Enter password to unlock`），直接报 `sp: invalid parameter: otp must be an otpauth:// URL (KeePassXC compatible, DR-005)`，退出码 **4** | SP-F-102（校验前置） | |
| A2.4 | `& $SP item create --db $TESTDB reserved --field "Password=x"` | **不提示主密码**，直接报 `sp: invalid parameter: custom field name 'Password' is reserved`，退出码 **4** | SP-F-102（校验前置） | |
| A2.5 | `& $SP item create --db $TESTDB github -u alice2`（与 A2.1 同名） | **不报错**（同名允许，DR-016）；区分实例用 `--index`（见 A2b 组）——测试结束记得 delete 清理 | 手册 §4.1 注/§4.5 | |
| A2.6 | `& $SP item create --db $TESTDB files -u f1`（后续附件测试用） | 成功 | SP-F-104 | |

### A2b. item edit 与同名 --index 寻址（手册 §4.5；2026-10-31 新增，DR-016）

> 依赖 A2.1/A2.5 的两条同名 `github` 条目；均需先解锁。

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A2b.1 | `& $SP item list --db $TESTDB --enumerate` | `github` 两行分别带 `#1`/`#2` 序号前缀（Tab 分隔；文档序），其余标题带 `#1` | 手册 §4.3/§4.5 | |
| A2b.2 | `& $SP item get --db $TESTDB github --index 2 -a UserName` | 输出 `alice2`（A2.5 那条）；`--index 1` 输出 `alice` | DR-016 | |
| A2b.3 | `& $SP item edit --db $TESTDB github --index 2 -u alice2-edited --url https://github.com/x --field env=prod` | 成功打印 `Updated entry 'github' (#-2): username, url, custom_fields(+1)`（**不回显值**） | SP-F-102 | |
| A2b.4 | `& $SP item get --db $TESTDB github --index 1` | `#1` 完全不受影响（UserName 仍 `alice`、无 env 字段）——patch 语义 | SP-F-102 | |
| A2b.5 | `& $SP item read --db $TESTDB github/UserName --index 2` | 输出 `alice2-edited`（read/clip 同口径） | DR-016 | |
| A2b.6 | `& $SP item edit --db $TESTDB github -p` | 提示 `New entry password (empty to keep current): `；输入空行=保持现密码（`get` 复核 Password 未变） | 手册 §4.5/§12 | |
| A2b.7 | `& $SP item edit --db $TESTDB github`（无修改参数） | **不提示主密码**，报 `nothing to edit`，退出码 **4** | SP-F-102（校验前置） | |
| A2b.8 | `& $SP item edit --db $TESTDB github --index 0 -u x` | **不提示主密码**，报 `--index is 1-based`，退出码 **4** | SP-F-102（校验前置） | |
| A2b.9 | `& $SP item edit --db $TESTDB github --rename ""` | **不提示主密码**，退出码 **4** | SP-F-102（校验前置） | |
| A2b.10 | `& $SP item get --db $TESTDB github --index 5` | 退出码 **3**，stderr 含同名总数与"越界"字样 | DR-016 | |
| A2b.11 | `& $SP item edit --db $TESTDB github --index 2 --rename github-work` | 成功；此后 `github` 仅剩 1 条，`get github --index 2` 退出 3 | 手册 §4.5 示例 3 | |
| A2b.12 | `& $SP item delete --db $TESTDB github --index 1` 后 `& $SP item list` | 打印 `Deleted entry 'github' (#-1)`；列表无 `github`、有 `github-work` | 手册 §4.4 | |

### A2c. 受保护自定义字段（手册 §4.1/§4.5/§14.5；2026-10-31 新增，S1d-1/DR-017）

> 依赖 A1 建库；均需先解锁。

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A2c.1 | `& $SP item create --db $TESTDB vault-prod -u svc --field-secret api_key=k-9 --field env=prod` | 成功；一条目同时携带受保护（api_key）与普通（env）字段 | SP-F-102 | |
| A2c.2 | `& $SP item get --db $TESTDB vault-prod` | `api_key: ●●●●●●●●`（掩码）、`env: prod`（明文） | SP-F-102 | |
| A2c.3 | `& $SP item get --db $TESTDB vault-prod -a api_key` | 输出 `k-9`（显式取值明文） | SP-F-102 | |
| A2c.4 | `& $SP item edit --db $TESTDB vault-prod --field-secret api_key=k-10` | 成功；`-a api_key` 读到 `k-10` 且 `get` 仍掩码 | SP-F-102 | |
| A2c.5 | `& $SP item create --db $TESTDB bad-secret --field-secret "Title=x"` | **不提示主密码**，报保留名错误，退出码 **4** | SP-F-102（校验前置） | |
| A2c.6 | 用 KeePassXC 打开 `$TESTDB` 查看 vault-prod | `api_key` 属性显示掩码（受保护标记互认），值可经 KeePassXC 解锁查看且为 `k-10`；`env` 明文可见 | 手册 §14.5（互操作） | |
| A2c.7 | `& $SP item get --db $TESTDB vault-prod` 确认 KeePassXC 侧查看后 sp 读回无变化 | protected 位与值保真（跨工具往返） | SP-F-102 | |

### A3. item list（手册 §4.3）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A3.1 | `& $SP item list --db $TESTDB` | 逐行输出标题、**排序后**（bad-otp / files / github / reserved / work-vpn），不含其他字段 | SP-F-102 | |
| A3.2 | `& $SP item list --db $TESTDB --tag work` | 仅 `work-vpn`（精确匹配、大小写不敏感） | SP-F-108 | |
| A3.3 | `& $SP item list --db $TESTDB --tag WORK` | 同 A3.2（大小写不敏感验证） | SP-F-108 | |
| A3.4 | `& $SP item list --db $TESTDB --tag no-such-tag` | **空输出**、退出码 0 | SP-F-108 | |

### A4. item get（手册 §4.2）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A4.1 | `& $SP item get --db $TESTDB work-vpn` | 逐行 `Title: work-vpn` / `UserName: bob` / `Password: …` / `URL: …` / **`Tags: infra, work`**（有标签才打此行）——注意密码明文整行打印，勿入日志 | SP-F-102/108 | |
| A4.2 | `& $SP item get --db $TESTDB github -a Password` | 仅输出该条目密码明文（单字段） | SP-F-102 | |
| A4.3 | `& $SP item get --db $TESTDB work-vpn -t` | 输出 6 位 TOTP 动态码（每 30s 变化；记下当前值，GUI 部分 B6.2 要比对） | SP-F-601 | |
| A4.4 | `& $SP item get --db $TESTDB no-such` | 报未找到，退出码 **3** | 手册 §13 | |

### A5. clip（手册 §7）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A5.1 | `& $SP clip --db $TESTDB github/Password` | 打印 `Copied to clipboard; clearing in 30s (Win+V history is NOT controlled by sp).`；**阻塞约 30s**；立即粘贴验证值正确；30s 后粘贴为空 | SP-F-102（clip 语义） | |
| A5.2 | `& $SP clip --db $TESTDB github/Password --timeout 5` | 文案显示 `clearing in 5s`，5s 后清空（覆盖默认） | 手册 §7 | |

### A6. attachment（手册 §5）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A6.1 | 准备附件源文件并记录哈希：`"att test line" | Out-File C:\vault\att-src.txt`，然后 `Get-FileHash C:\vault\att-src.txt -Algorithm SHA256` | 记下**附件源文件**的哈希（A6.4 的比对对象是它，不是 sp.exe） | — | |
| A6.2 | `& $SP attachment add --db $TESTDB files C:\vault\att-src.txt --name att-test.txt` | 成功（同名覆盖不提示） | SP-F-104 | |
| A6.3 | `& $SP attachment list --db $TESTDB files` | 逐行输出 `att-test.txt`（排序） | SP-F-104 | |
| A6.4 | `& $SP attachment get --db $TESTDB files att-test.txt --out C:\vault\att-out.txt`，然后 `Get-FileHash C:\vault\att-out.txt -Algorithm SHA256` | stderr 提示写入路径与字节数；下载文件哈希与 A6.1 **一致**（字节往返无损） | SP-F-104 | |
| A6.5 | 限权核对：`icacls C:\vault\att-out.txt` | 仅显示当前用户 `(<机器名>\<用户>):F`，**无继承的 SYSTEM/Administrators/Everyone 等条目**（/inheritance:r 生效） | SP-F-104（限权） | |
| A6.6 | `& $SP attachment get --db $TESTDB files no-such` | 报未找到，退出码 **3** | 手册 §13 | |

### A7. read（手册 §6）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A7.1 | PowerShell：`$v = & $SP read --db $TESTDB github/Password -n; $v.Length` | 取到密码明文（`-n` 去尾换行），stdout 干净可管道 | SP-F-102 | |
| A7.2 | `& $SP read --db $TESTDB github/Password --out-file C:\vault\read-out.txt` | 写文件、无终端输出；文件内容为密码值（icacls 限权同 A6.5） | 手册 §6 | |
| A7.3 | `& $SP read --db $TESTDB malformed-no-slash` | **不提示主密码**，直接报参数错误（URI 须 `<标题>/<字段>`），退出码 **4**（校验前置，v1.1） | 手册 §6/§13 | |

### A8. 删除与错误语义

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A8.1 | `& $SP item delete --db $TESTDB reserved` | 打印 `Deleted entry 'reserved'` | SP-F-102 | |
| A8.2 | `& $SP item get --db $TESTDB reserved` | 退出码 **3**（已删） | 手册 §13 | |
| A8.3 | 任意命令输入**错误主密码** | 报凭据错误，退出码 **2**（不细分错密码/错 keyfile，防侧信道） | SP-F-105 | |
| A8.4 | 注：`sp item create`（缺位置参数）等 **clap 解析错误也是退出码 2**——与凭据错误同码，靠 stderr 文本区分（手册 §13 注） | — | 手册 §13 注 | |

### A9. 可选扩展冒烟（不影响验收结论，按需执行）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A9.1 | `& $SP import --dry-run dotenv <PATH>`（手册 §10） | 打印 `Scan: N findings -> M unique candidates ...` 与 `REVIEW [L3 conflict]` 行；**零写入** | SP-F-401 | |
| A9.2 | `& $SP export --out C:\vault\backup.zip.age`（手册 §11） | 提示 `Archive passphrase:` 两次；产出 age 加密 zip | SP-F-501 | |
| A9.3 | 环境变量注入（F8） | **完整用例独立成节，见 A10** | SP-F-301/3xx | |

### A10. 环境变量注入（F8，`sp run` 完整用例，手册 §8）——**自包含，可独立执行**

> **自包含口径（v1.2）**：本节从清理旧产物 → 初始化库 → 添加条目 → 注入验证全流程闭环，**不依赖 A 组其他用例的任何产物**；开头会删除本节自己的旧文件，可反复重跑。
> 语义要点：env 文件为 `.env` 格式（`#` 注释、支持引号/跨行/变量展开）；值写 `sp://标题/字段` 引用库内秘密（**明文不落盘，只落"引用"**——env 文件里没有机密字面量，可以放心备份/提交）；子进程 stdout/stderr 中的秘密值默认打码为 `<redacted>`；子进程**退出码原样透传**；保护变量名（PATH、XDG_*、LC_* 等，SP-F-301）拒绝注入并警告。
> **凭据卫生**：本节不含任何真实凭据字面量——主密码、条目密码、大模型 API key 全部在交互提示时输入（终端不回显）；示例中的 key 演示值均为占位说明。
> **零网络口径**：SP-N-304 约束的是 sp 自身运行期零网络；**注入的子进程**（如本节的大模型 API 调用）联网属用户行为，不受该红线约束。

#### A10.0 变量与清理（幂等重跑入口）

```powershell
$SP    = "E:\proj\SuperPasswdKey\target\release\sp.exe"   # 新构建 CLI（同 §0.2）
$LLMDB  = "C:\vault\sp-llm-test.kdbx"                     # 本节专用测试库（独立于 $TESTDB）
$LLMKEY = "C:\vault\sp-llm-test.keyfile"                  # init 自动生成的双因子 keyfile

# 清理本节旧产物（幂等；删除的只有本节自己的文件）
Remove-Item $LLMDB, $LLMKEY, "C:\vault\llm.env", `
  "C:\vault\llm-bad-uri.env", "C:\vault\llm-missing.env" -ErrorAction SilentlyContinue
```

#### A10.1 初始化库并添加条目

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A10.1a | `& $SP init --db $LLMDB`（交互设主密码，建议一次性测试口令） | 提示 `Generating keyfile (XML v1.0) at ...sp-llm-test.keyfile` + KDF 校准；库与 keyfile 生成 | SP-F-101 | |
| A10.1b | `& $SP item create --db $LLMDB --keyfile $LLMKEY github`（条目密码随意测试值） | 成功（对照条目，供 GH_TOKEN 引用） | SP-F-102 | |
| A10.1c | `& $SP item create --db $LLMDB --keyfile $LLMKEY llm-key --url https://api.deepseek.com`；提示 `Entry password (empty to skip):` 时**粘贴你的大模型 API key**（不回显） | 成功——API key 以 Password 字段加密入库（与普通密码同强度）；URL 字段存服务商地址 | SP-F-102 | |
| A10.1d | `& $SP item get --db $LLMDB --keyfile $LLMKEY llm-key -a Password`（可选自查） | 输出与你粘贴的 key 一致（注意终端历史留存） | — | |

> 注：所有后续命令都带 `--keyfile $LLMKEY`（init 自动生成双因子，缺它解锁必失败）。

#### A10.2 准备注入引用文件（秘密不落盘，只落引用）

```powershell
# llm.env：大模型 API key（引用）+ 服务商地址（明文，非机密）+ 对照条目 + 保护变量
@'
# SuperPasswdKey inject test (values are sp:// references, not secrets)
LLM_API_KEY=sp://llm-key/Password
LLM_BASE_URL=https://api.deepseek.com
GH_TOKEN=sp://github/Password
PATH=sp://llm-key/Password
'@ | Out-File C:\vault\llm.env -Encoding utf8

# 坏引用（缺字段段）
@'
BAD_URI=sp://malformed
'@ | Out-File C:\vault\llm-bad-uri.env -Encoding utf8

# 不存在的条目
@'
X=sp://no-such/Password
'@ | Out-File C:\vault\llm-missing.env -Encoding utf8
```

（示例用 DeepSeek 的 OpenAI 兼容地址；OpenAI 用户把 `LLM_BASE_URL` 改为 `https://api.openai.com/v1` 即可，其余步骤不变。）

#### A10.3 注入验证

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A10.3a | `& $SP run --db $LLMDB --keyfile $LLMKEY --env-file C:\vault\llm.env -- cmd /c echo %LLM_API_KEY%` | 解锁一次后子进程执行；stdout 输出 **`<redacted>`**（子进程 echo 出的 API key 被打码——**即使用户脚本误打印机密也安全**；`%LLM_API_KEY%` 是 **cmd 变量展开**，非 PowerShell 变量） | SP-F-301/3xx | |
| A10.3b | `... --env-file C:\vault\llm.env -- cmd /c echo %LLM_BASE_URL%` | 输出 **`<redacted>`**（v1.2 修订：打码面 = **env 文件全部值**，含明文非引用值——保守设计，防用户把机密直接写成明文时漏保护；"非引用值正确注入"改由 A10.3c 的 `--no-masking` 验证） | SP-F-3xx | |
| A10.3c | `... --no-masking --env-file C:\vault\llm.env -- cmd /c echo %LLM_API_KEY%` 与 `... --no-masking -- cmd /c echo %LLM_BASE_URL%` | 分别输出**真实 API key**（与 A10.1c 粘贴值一致）与 `https://api.deepseek.com`（`--no-masking` 仅调试核对用） | SP-F-3xx（值正确性） | |
| A10.3d | 观察 A10.3a 的 stderr | 出现 `sp: refusing to set protected variable 'PATH' (SP-F-301)`——PATH 被拒绝注入，其余变量照常 | SP-F-301 | |
| A10.3e | 退出码透传：`... -- cmd /c exit 7`，然后 `$LASTEXITCODE` | 子进程退出码原样透传：`7` | 手册 §8 | |
| A10.3f | `& $SP run --db $LLMDB --keyfile $LLMKEY --env-file C:\vault\llm-bad-uri.env -- cmd /c echo x` | **不提示主密码**，直接报参数错误（`uri 'malformed' must be <title>/<field>`），退出码 **4**（sp:// 引用格式校验前置，v1.2） | 手册 §8/§13 | |
| A10.3g | `& $SP run --db $LLMDB --keyfile $LLMKEY --env-file C:\vault\llm-missing.env -- cmd /c echo x` | 提示主密码解锁后报未找到（引用的条目不存在），退出码 **3**——条目存在性必须查库，无法前置（与 A10.3f 的格式前置相区分） | 手册 §8/§13 | |

#### A10.4 实战使用：注入 LLM API key 调用大模型 API

> 子进程（PowerShell）从**环境变量**读取 key（凭据不进命令行参数、不进文件），真实调用一次大模型服务——验证"添加 → 输入 → 使用"全链路。

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A10.4a | ```& $SP run --db $LLMDB --keyfile $LLMKEY --env-file C:\vault\llm.env -- powershell -NoProfile -Command "(Invoke-RestMethod -Uri (`$env:LLM_BASE_URL + '/models') -Headers @{Authorization = 'Bearer ' + `$env:LLM_API_KEY}).data.id"``` | 输出服务商返回的**模型 id 列表**（DeepSeek：`deepseek-chat` / `deepseek-reasoner` 等）——API key 从库经环境变量注入、真实鉴权成功 | SP-F-3xx（端到端） | |
| A10.4b | 把 A10.4a 的命令里的 `LLM_API_KEY` 故意输错验证打码兜底：改用 `-- cmd /c echo Bearer %LLM_API_KEY%` 搭配一个会回显的假调用（可选） | 若某工具把 key 打到 stdout/stderr，输出仍为 `<redacted>`（打码层兜底） | SP-F-3xx | |
| A10.4c | （可选）chat 冒烟：同 A10.4a 结构改 POST `/chat/completions`（body 传 `{"model":"deepseek-chat","messages":[{"role":"user","content":"ping"}]}`，`-ContentType 'application/json'`） | 返回 JSON 含 `choices[0].message.content`——注入的 key 可直接支撑真实应用 | SP-F-3xx | |

> key 轮换口径：key 更新用 GUI 编辑该条目（S1b 编辑流）或删除重建；env 文件无需改动（引用不变）。

#### A10.5 可选补充与清理

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A10.5a | （可选，POSIX 语义）`--stdin` 模式：手册 §8 示例 2——秘密经子进程 stdin 的 NUL 分隔记录传入（避免环境变量暴露面）。Windows 下子进程需配合读取，以实际行为为准，不做硬性断言 | — | 手册 §8 | |
| A10.5b | 清理（可选）：`Remove-Item $LLMDB, $LLMKEY, C:\vault\llm*.env -ErrorAction SilentlyContinue` | 本节产物清空 | — | |

> 注：`inject`（F8 模板渲染，`sp inject -i <模板> -o <输出>`）与 `run` 共享 `sp://` 引用与解锁路径，完整语义见手册 §9，不重复列用例。

---

## B. GUI 逐项清单

> 对象：本轮新构建的 `sp-desktop.exe`（托盘实例）；主库 `$VDB`（已按 0.5 备份）。
> 建议先按 B1 准备主库条目（若主库已有真实条目可跳过建库、直接用现有条目测 B4–B6）。

### B1. 库准备（CLI 完成，GUI 消费）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B1.1 | `& $SP init --db $VDB`（已有库则跳过；**注意此路径即 GUI 单库**，已备份） | 建库成功（或已存在则跳过） | |
| B1.2 | `& $SP item create --db $VDB gui-github -u alice --url https://github.com --tag dev` | 成功（GUI 解锁后应可见） | |
| B1.3 | `& $SP item create --db $VDB gui-vpn -u bob --totp "otpauth://totp/Test?secret=JBSWY3DPEHPK3PXP&issuer=Test" --tag work --field "region=cn"` | 成功（带 TOTP + 标签 + 自定义字段，测 GUI 全字段展示） | |

### B2. 启动与托盘（SP-F-901）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B2.1 | 双击 `sp-desktop.exe` | 主窗弹出（解锁面板：主密码 + keyfile 输入框），托盘图标出现 | |
| B2.2 | 点主窗 ✕ 关闭 → 托盘左键单击 | 窗口隐藏、进程保留（任务管理器可见 sp-desktop.exe）；托盘单击重新唤出 | |

### B3. 解锁 / 锁定 / keyfile 记忆（SP-F-902/903/904）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B3.1 | 输入**错误**主密码点"解锁" | 提示"解锁失败：主密码或 keyfile 不正确"，保持锁定态 | |
| B3.2 | 输入正确主密码 | 约 1–3s（KDF）后进入主界面；顶栏显示"已解锁 · 空闲 Xm · 绝对 Xmin"倒计时递减 | |
| B3.3 | 顶栏"锁定" | 立即切回解锁面板；再进需重新解锁（列表/详情内存态丢弃） | |
| B3.4 | **keyfile 路径记忆**（需库带 keyfile）：在 keyfile 输入框手输路径解锁成功一次 → 关闭应用重新启动 | keyfile 输入框**已预填**上次成功解锁使用的路径（settings.json 的 keyfile_path 字段；该文件不存在时启动后为空、需重新手输——记忆的配置本身保留，文件恢复即复用） | SP-F-904 |
| B3.5 | **keyfile 文件浏览**：点 keyfile 输入框旁"浏览…" | 弹系统文件选择对话框；选定文件后路径填入输入框；**取消**则不改动 | SP-F-904 |

### B4. 浏览：列表 / 搜索 / 标签（SP-F-908/108）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B4.1 | 解锁后观察左栏 | 条目列表自动加载（gui-github / gui-vpn …），gui-vpn 行有 **TOTP** 徽标与 **dev/work 标签徽标** | |
| B4.2 | 搜索框输入 `git` | 即时过滤命中 gui-github（标题/用户名/URL/**标签**均参与匹配） | |
| B4.3 | 搜索框输入 `work`（标签词） | 命中 gui-vpn（搜索覆盖标签） | |
| B4.4 | 点击 gui-github 行的 `dev` 标签徽标 | 搜索框被填为 `dev`，列表过滤到该标签（再次手动清空恢复全量） | SP-F-108 |
| B4.5 | 搜索框输入 `zzzz` | 显示"无匹配条目" | SP-F-908 |

### B5. 详情 / 密码交互 / 自定义字段（SP-F-908/909）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B5.1 | 点击 gui-vpn | 详情展示：**标签行**（work）、用户名、URL、备注、**自定义字段 region=cn 行（带复制）**、TOTP 行；密码显示为掩码 ●●●●●●●●（无明文回显） | SP-F-908 |
| B5.2 | 密码行点 ▾ → **显示** | 菜单含 显示 / 放大显示 / 复制 三项；点"显示"后掩码切换为明文；再点"隐藏"复原 | SP-F-909 |
| B5.3 | 切换到另一条目再切回 | 密码**复位为掩码**（显示状态不跨条目保持） | SP-F-707 口径 |
| B5.4 | ▾ → **放大显示** | 全屏白底遮罩 + 超大等宽字体显示密码明文；点击任意处或按 **Esc** 关闭 | SP-F-909 |
| B5.5 | 密码行"复制" → 立即粘贴 → 等 30s 再粘贴 | 提示"已复制 密码，30 秒后自动清空"；首次粘贴值正确；30s 后粘贴为空 | SP-F-908 |

### B6. TOTP（SP-F-601/908）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B6.1 | 打开 gui-vpn 详情观察 TOTP 行 | 显示 6 位数字码；**等待约 30s 自动刷新**为新码（无需手动操作） | |
| B6.2 | 复制 TOTP 粘贴，与 CLI 比对：`& $SP item get --db $VDB gui-vpn -t` | 两者**同码**（30s 窗口内） | |

### B7. 创建（SP-F-909）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B7.1 | 左栏"＋ 新的项目" → 直接点"保存" | 表单错误"标题必填"，未提交 | |
| B7.2 | 填标题，TOTP 填 `not-otpauth` → 保存 | 错误"TOTP 链接必须以 otpauth:// 开头"（对齐 CLI 退出码 4 语义） | |
| B7.3 | 自定义字段"添加字段" → 名称填 `Password` → 保存 | 错误"保留名"（与 CLI/sp-core 同源校验） | |
| B7.4 | 修正：标题 `gui-created`、用户名/URL/备注、**标签填 ` temp，测试 , 中文逗号尾空格,`**、自定义字段 `api_key=k-1` → 保存 | 成功；表单关闭；**列表与详情立即更新**并选中 gui-created；详情可见标签 `temp` `测试` `中文逗号尾空格`（**中英文逗号均解析、自动 trim、空段剔除**）与自定义字段 api_key | SP-F-909/108 |
| B7.5 | CLI 侧核对：`& $SP item get --db $VDB gui-created` | CLI 读回与 GUI 所填一致（含 `Tags:` 行与 api_key） | 互操作 |

### B8. 编辑（SP-F-909/103）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B8.1 | gui-created 详情点"编辑" | 表单预填全部字段（含标签回填为逗号串、自定义字段行预填） | |
| B8.2 | 改标题为 `gui-created-2`、改密码、**自定义字段 api_key 值改为 k-2**、再"添加字段"一行 `env=prod` → 保存 | 成功；**列表与详情立即更新**（无需手动刷新/重开）：选中态跟到新标题、详情显示新值 | |
| B8.3 | **标题不变只改字段再保存**（如改 api_key 为 k-3） | 详情**立即**显示 k-3（标题不变时此前需手动刷新，v1.1 已修复） | |
| B8.4 | 自定义字段行 ✕ 删除 `env` 行 → 保存 | 保存后详情不再显示 env | |
| B8.5 | CLI 核对：`& $SP item get --db $VDB gui-created-2` | 新标题可读（旧名 get 退出 3）；api_key=k-3；Tags 保持 | |
| B8.6 | （可选）KeePassXC 打开主库目检 | gui-created-2 的**历史记录含旧快照**（edit_entry update_history 语义，SP-F-103）；Tags 在 KeePassXC 条目中互认可见 | |

### B9. 删除（SP-F-909）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B9.1 | gui-created-2 详情点"删除" | 确认框明示条目标题与不可撤销提示 | |
| B9.2 | 确认删除 | 列表刷新（条目消失）、详情清空回"从左侧选择…"；CLI `item get gui-created-2` 退出 3 | |

### B10. 附件（SP-F-910）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B10.1 | gui-github 详情底部"添加附件"选一个本地文件 | 附件行出现（文件名）；提示"已添加附件 …（同名覆盖）"；**再次添加同名文件** → 行仍只有一个该名（覆盖语义对齐 CLI） | |
| B10.2 | 点附件行"下载" | 弹**系统保存对话框**（默认文件名=附件名）；选目录保存后提示"已保存 …→ 完整路径"；打开文件内容正确 | |
| B10.3 | 限权核对：`icacls <下载的文件>` | 仅当前用户 `:F`，无继承条目（与 CLI attachment get --out 同语义） | |
| B10.4 | 下载对话框点"取消" | 无报错、无提示（静默取消） | |

### B11. 设置 / 自启 / 双超时（SP-F-903/904/905）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B11.1 | 顶栏"设置"：空闲锁定改 **1 分钟** → 保存 | 提示"已保存：自启即时生效；超时配置重启后生效"；keyfile 路径记忆不受设置保存影响 | |
| B11.2 | 关闭应用重新启动 → 解锁 → 顶栏"设置" | 空闲锁定显示 1 分钟（持久化生效） | |
| B11.3 | 解锁后**闲置 1 分钟**（不点击不输入、**不开编辑表单**） | 自动切回解锁面板（顶栏倒计时同步归零）；重新解锁正常 | SP-F-903 |
| B11.4 | 勾选"开机自启"保存 → `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v SuperPasswdKey` | 值存在（带引号 exe 全路径）；**取消勾选**保存后值清零 | SP-F-905 |
| B11.5 | 测试完把空闲锁定改回原值（如 5 分钟） | — | |

### B12. 编辑中不被空闲锁定（SP-F-909，v1.1 新增）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B12.1 | 空闲锁定设 1 分钟 → 解锁 → 打开任一条目的**编辑表单** → 闲置 **>1 分钟**（表单内不操作） | **会话不锁定**：表单保持打开、内容不丢；顶栏空闲倒计时保持满格（编辑态暂停 idle 计时） | |
| B12.2 | 编辑中闲置超时后直接点"保存" | 保存成功（无需重新解锁）；详情刷新 | |
| B12.3 | 关闭编辑表单后再闲置 1 分钟（不操作） | 恢复正常空闲判定 → 自动切回解锁面板 | SP-F-903 |
| B12.4 | 边界说明：**绝对过期（默认 30 分钟）不因编辑而暂停**——编辑中达到绝对上限仍会锁定；此时提交提示"会话已锁定"，**表单内容保留**，重新解锁后可直接再保存 | — | |

### B13. 锁定语义与互操作

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B13.1 | 解锁后另开终端：`& $SP item list --db $VDB --tag dev` | CLI 独立解锁（再输一次主密码）后正常列出——两进程并存、会话互不干扰（每进程首次主密码，DR-013） | |
| B13.2 | GUI 解锁期间 CLI **只读**操作（list/get） | 均正常（只读互不影响） | |
| B13.3 | ⚠️ 全程避免：GUI 解锁期间在 CLI/KeePassXC **修改**主库（MVP 无外部修改检测，GUI 写可能覆盖外部改动）——见 design_docs/16 §8.2 并发注意 | — | |
| B13.4 | （浏览器填充环境已配置者）扩展走 NM 实例填充 | 不受本轮影响（NM 单进程嵌入逻辑未动） | SP-F-906/907 |

---

## C. 结果记录与问题上报

- 每项在"实际结果"列填 ✅ / ❌ / N/A；❌ 附：截图 + 复现步骤 + 当时操作序号。
- GUI 问题排查：`cd desktop/src-tauri && cargo tauri dev` 可开前端 console（devtools）观察 invoke 报错。
- CLI 问题先对照 `docs/manual/sp_cli_manual.md` 对应节（每项已标注节号）——手册口径"程序输出与手册不一致时以程序输出为准"，但仍请上报以修手册。
- 覆盖映射：B2→SP-F-901；B3/B11→902/903/904/905；B4→908/108；B5/B7/B8/B9/B12→909/103/707；B6→601；B10→910；A 组→102/104/105/108/301/3xx 及手册 §13 退出码语义。
- 测试完成：把本文件中"实际结果"列的结论留档（可直接提交回仓或归档到 docs/reviews/），主库按 0.5 备份还原（如需）。
