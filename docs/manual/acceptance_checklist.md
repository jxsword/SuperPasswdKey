# 验收测试清单（GUI + CLI）— v1.0（2026-10-31）

> 用途：**逐项执行**的验收清单，覆盖 S1a/S1b 全部用户可见功能（SP-F-901/902/903/904/905/908/909/910/601/108）。
> 用法：按序执行，在每行"实际结果"列填 ✅/❌；❌ 请附截图与复现步骤。命令与预期输出以 `sp_cli_manual.md` 为详注；若程序输出与手册不一致，以程序输出为准并如实记录。
> 上游：design_docs/16 §8（启动指引与协作原理图）、S1a/S1b 交付说明（docs/plans/）。

---

## 0. 前置准备（必读）

### 0.1 ⚠️ 必须重建（旧版不含本轮功能）

PATH 中的 `sp.exe`（`E:\rust\cargo\bin`）与已安装的桌面版均为 **v0.3.0**，**不含 `--tag` 与 GUI 写路径（创建/编辑/删除/附件/放大显示等）**。测试前必须重建：

```powershell
# 仓库根目录执行
cargo build --release --workspace          # CLI：产出 target\release\sp.exe
cd desktop/src-tauri
cargo tauri build                          # GUI：产出 target\release\sp-desktop.exe + NSIS 安装包
```

本轮 CLI 测试**统一用全路径**调用新构建的 `target\release\sp.exe`（下文记作 `%SP%`）——这同时规避了同名撞车问题（见 0.3）。

### 0.2 GUI 程序位置与启动方式

| 方式 | 位置 / 命令 | 说明 |
|---|---|---|
| **直接运行（本轮推荐）** | `desktop\src-tauri\target\release\sp-desktop.exe` | 双击或命令行启动；弹解锁窗 + 托盘常驻；关窗=隐藏 |
| NSIS 安装包 | `desktop\src-tauri\target\release\bundle\nsis\SuperPasswdKey_0.3.0_x64-setup.exe` | 安装后从开始菜单"SuperPasswdKey"启动 |
| 开发模式（排查问题用） | `cd desktop/src-tauri && cargo tauri dev` | Vite dev server 自动拉起；可看前端 console；Rust 改动触发重编 |
| 开机自启 | GUI 顶栏"设置"→ 勾选 | 写 HKCU Run（免管理员） |
| NM 实例 | 不手动启动 | 浏览器扩展 connectNative 经 manifest 拉起，随断开自动退出 |

### 0.3 同名撞车核对（sp.exe）

第三方程序可能与本 CLI 同名（已知案例：**win-pyvideotrans v4.17 项目的 GUI 应用亦为 sp.exe**）。开始前先核对：

```powershell
where.exe sp          # 列出 PATH 中全部同名命中（按优先级排序）
sp --version          # 确认实际命中的是谁
```

若命中了第三方程序：本轮用 `%SP%` 全路径即可；长期处置见 CLI 手册 §1（PATH 调序，推荐 `scripts/add-path.ps1`）。

### 0.4 测试库隔离与备份

- **CLI 部分**：用独立测试库，不碰主库——`--db C:\vault\sp-test.kdbx`（下文记作 `%TESTDB%`）。
- **GUI 部分**：桌面 GUI 锁定单库 `%APPDATA%\SuperPasswdKey\vault.kdbx`，无法换库——**测试前先备份**：

```powershell
copy "$env:APPDATA\SuperPasswdKey\vault.kdbx" "$env:APPDATA\SuperPasswdKey\vault.kdbx.bak-acceptance"
```

（测试完成后可用备份还原；注意备份的是 KDBX 文件本身，keyfile 如有也需一并备份。）

### 0.5 剪贴板验证环境

准备一个可粘贴的记事本/输入框，用于 D 组（复制）与 CLI clip 的验证。

---

## A. CLI 逐项清单

> 通用约定：`%SP%` = 新构建的 `target\release\sp.exe` 全路径；`%TESTDB%` = `C:\vault\sp-test.kdbx`；
> 交互提示（`Enter password to unlock ...`、`Entry password (empty to skip):`）走 stderr，stdout 是干净数据通道；
> 测试主密码建议用一次性口令（如 `sp-acceptance-only`），不入任何真实库。

### A0. 环境

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A0.1 | `where.exe sp` | 列出 PATH 中全部同名命中（含本 CLI 与/或第三方 sp.exe，按优先级排序） | 手册 §1 撞名注记 | |
| A0.2 | `%SP% --version` | 输出版本号（应为当前构建版本） | — | |

### A1. init（手册 §3）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A1.1 | `%SP% init --db %TESTDB%`（交互） | 依次提示 `Choose a master password:` → `Confirm master password:`（两次输入不一致会要求重输）；成功后打印 `REMINDER: back up the keyfile to a separate medium (USB/another device).` 与 `KDF: Argon2id m=… t=… p=…`（校准参数） | SP-F-101 | |
| A1.2 | 检查 `%TESTDB%` 同目录 | 生成 `sp-test.keyfile`（init 自动建双因子） | SP-F-101 | |
| A1.3 | 再次执行同命令 | 报错退出码 **4**（目标库已存在） | 手册 §3 注 | |

### A2. item create（手册 §4.1）

> 以下均需先按提示输入测试主密码解锁，再按提示输入条目密码（可留空）。

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A2.1 | `%SP% item create --db %TESTDB% github -u alice --url https://github.com` | 成功打印 `Created entry 'github'`（解锁提示 → 条目密码提示可留空） | SP-F-102 | |
| A2.2 | `... item create --db %TESTDB% work-vpn -u bob --totp "otpauth://totp/Test?secret=JBSWY3DPEHPK3PXP&issuer=Test" --tag work --tag infra` | 成功；单条目携带两个标签 | SP-F-102/108 | |
| A2.3 | `... item create --db %TESTDB% bad-otp --totp "not-otpauth"` | 报参数错误，退出码 **4** | SP-F-102（校验） | |
| A2.4 | `... item create --db %TESTDB% reserved --field "Password=x"` | 报参数错误（保留名），退出码 **4** | SP-F-102（校验） | |
| A2.5 | `... item create --db %TESTDB% github -u alice2`（与 A2.1 同名） | **不报错**（同名允许）；此后 get/read/clip 只命中首个（遮蔽语义）——测试结束记得 delete 清理 | 手册 §4.1 注 | |
| A2.6 | `... item create --db %TESTDB% files -u f1`（后续附件测试用） | 成功 | SP-F-104 | |

### A3. item list（手册 §4.3）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A3.1 | `%SP% item list --db %TESTDB%` | 逐行输出标题、**排序后**（bad-otp / files / github / reserved / work-vpn），不含其他字段 | SP-F-102 | |
| A3.2 | `... item list --db %TESTDB% --tag work` | 仅 `work-vpn`（精确匹配、大小写不敏感） | SP-F-108 | |
| A3.3 | `... item list --db %TESTDB% --tag WORK` | 同 A3.2（大小写不敏感验证） | SP-F-108 | |
| A3.4 | `... item list --db %TESTDB% --tag no-such-tag` | **空输出**、退出码 0 | SP-F-108 | |

### A4. item get（手册 §4.2）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A4.1 | `%SP% item get --db %TESTDB% work-vpn` | 逐行 `Title: work-vpn` / `UserName: bob` / `Password: …` / `URL: …` / **`Tags: infra, work`**（有标签才打此行）——注意密码明文整行打印，勿入日志 | SP-F-102/108 | |
| A4.2 | `... item get --db %TESTDB% github -a Password` | 仅输出该条目密码明文（单字段） | SP-F-102 | |
| A4.3 | `... item get --db %TESTDB% work-vpn -t` | 输出 6 位 TOTP 动态码（每 30s 变化；记下当前值，GUI 部分 B4.2 要比对） | SP-F-601 | |
| A4.4 | `... item get --db %TESTDB% no-such` | 报未找到，退出码 **3** | 手册 §13 | |

### A5. clip（手册 §7）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A5.1 | `%SP% clip --db %TESTDB% github/Password` | 打印 `Copied to clipboard; clearing in 30s (Win+V history is NOT controlled by sp).`；**阻塞约 30s**；立即粘贴验证值正确；30s 后粘贴为空 | SP-F-102（clip 语义） | |
| A5.2 | `... clip --db %TESTDB% github/Password --timeout 5` | 文案显示 `clearing in 5s`，5s 后清空（覆盖默认） | 手册 §7 | |

### A6. attachment（手册 §5）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A6.1 | 准备本地文件 `C:\vault\att-src.txt`（内容随意）并记录其 SHA256：`Get-FileHash ... -Algorithm SHA256` | — | — | |
| A6.2 | `%SP% attachment add --db %TESTDB% files C:\vault\att-src.txt --name att-test.txt` | 成功（同名覆盖不提示） | SP-F-104 | |
| A6.3 | `... attachment list --db %TESTDB% files` | 逐行输出 `att-test.txt`（排序） | SP-F-104 | |
| A6.4 | `... attachment get --db %TESTDB% files att-test.txt --out C:\vault\att-out.txt` | stderr 提示写入路径与字节数；`Get-FileHash` 与 A6.1 **哈希一致**（字节往返无损） | SP-F-104 | |
| A6.5 | 限权核对：`icacls C:\vault\att-out.txt` | 仅显示当前用户 `(<机器名>\<用户>):F`，**无继承的SYSTEM/Administrators/Everyone 等条目**（/inheritance:r 生效） | SP-F-104（限权） | |
| A6.6 | `... attachment get --db %TESTDB% files no-such` | 报未找到，退出码 **3** | 手册 §13 | |

### A7. read（手册 §6）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A7.1 | PowerShell：`$v = %SP% read --db %TESTDB% github/Password -n; $v.Length` | 取到密码明文（`-n` 去尾换行），stdout 干净可管道 | SP-F-102 | |
| A7.2 | `... read --db %TESTDB% github/Password --out-file C:\vault\read-out.txt` | 写文件、无终端输出；文件内容为密码值（icacls 限权同 A6.5） | 手册 §6 | |

### A8. 删除与错误语义

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A8.1 | `%SP% item delete --db %TESTDB% reserved` | 打印 `Deleted entry 'reserved'` | SP-F-102 | |
| A8.2 | `... item get --db %TESTDB% reserved` | 退出码 **3**（已删） | 手册 §13 | |
| A8.3 | 任意命令输入**错误主密码** | 报凭据错误，退出码 **2**（不细分错密码/错 keyfile，防侧信道） | SP-F-105 | |
| A8.4 | 注：`sp item create`（缺位置参数）等 **clap 解析错误也是退出码 2**——与凭据错误同码，靠 stderr 文本区分（手册 §13 注） | — | 手册 §13 注 | |

### A9. 可选扩展冒烟（不影响验收结论，按需执行）

| # | 命令/操作 | 预期结果 | 覆盖 | 实际结果 |
|---|---|---|---|---|
| A9.1 | `sp run --env-file`（见手册 §8 示例：env 文件值用 `sp://title/field` 引用） | 子进程拿到注入变量；子进程 stdout 中出现的机密值被替换为 `<redacted>`；子进程退出码原样透传 | SP-F-3xx | |
| A9.2 | `sp import --dry-run dotenv <PATH>`（手册 §10） | 打印 `Scan: N findings -> M unique candidates ...` 与 `REVIEW [L3 conflict]` 行；**零写入** | SP-F-401 | |
| A9.3 | `sp export --out C:\vault\backup.zip.age`（手册 §11） | 提示 `Archive passphrase:` 两次；产出 age 加密 zip | SP-F-501 | |

---

## B. GUI 逐项清单

> 对象：本轮新构建的 `sp-desktop.exe`（托盘实例）；主库 `%APPDATA%\SuperPasswdKey\vault.kdbx`（已按 0.4 备份）。
> 建议先按 B1 准备主库条目（若主库已有真实条目可跳过建库、直接用现有条目测 C–E 组）。

### B1. 库准备（CLI 完成，GUI 消费）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B1.1 | `%SP% init --db "%APPDATA%\SuperPasswdKey\vault.kdbx"`（已有库则跳过；**注意此路径即 GUI 单库**，已备份） | 建库成功（或已存在则跳过） | |
| B1.2 | `%SP% item create --db "%APPDATA%\SuperPasswdKey\vault.kdbx" gui-github -u alice --url https://github.com --tag dev` | 成功（GUI 解锁后应可见） | |
| B1.3 | `... item create --db "%APPDATA%\SuperPasswdKey\vault.kdbx" gui-vpn -u bob --totp "otpauth://totp/Test?secret=JBSWY3DPEHPK3PXP&issuer=Test" --tag work --field "region=cn"` | 成功（带 TOTP + 标签 + 自定义字段，测 GUI 全字段展示） | |

### B2. 启动与托盘（SP-F-901）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B2.1 | 双击 `sp-desktop.exe` | 主窗弹出（解锁面板：主密码 + keyfile 输入框），托盘图标出现 | |
| B2.2 | 点主窗 ✕ 关闭 → 托盘左键单击 | 窗口隐藏、进程保留（任务管理器可见 sp-desktop.exe）；托盘单击重新唤出 | |

### B3. 解锁 / 锁定（SP-F-902/903）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B3.1 | 输入**错误**主密码点"解锁" | 提示"解锁失败：主密码或 keyfile 不正确"，保持锁定态 | |
| B3.2 | 输入正确主密码 | 约 1–3s（KDF）后进入主界面；顶栏显示"已解锁 · 空闲 Xm · 绝对 Xmin"倒计时递减 | |
| B3.3 | 顶栏"锁定" | 立即切回解锁面板；再进需重新解锁（列表/详情内存态丢弃） | |

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
| B6.2 | 复制 TOTP 粘贴，与 CLI 比对：`%SP% item get --db "%APPDATA%\SuperPasswdKey\vault.kdbx" gui-vpn -t` | 两者**同码**（30s 窗口内） | |

### B7. 创建（SP-F-909）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B7.1 | 左栏"＋ 新的项目" → 直接点"保存" | 表单错误"标题必填"，未提交 | |
| B7.2 | 填标题，TOTP 填 `not-otpauth` → 保存 | 错误"TOTP 链接必须以 otpauth:// 开头"（对齐 CLI 退出码 4 语义） | |
| B7.3 | 自定义字段"添加字段" → 名称填 `Password` → 保存 | 错误"保留名"（与 CLI/sp-core 同源校验） | |
| B7.4 | 修正：标题 `gui-created`、用户名/URL/备注、**标签填 ` temp，测试 , 中文逗号尾空格,`**、自定义字段 `api_key=k-1` → 保存 | 成功；表单关闭；列表刷新并选中 gui-created；详情可见标签 `temp` `测试` `中文逗号尾空格`（**中英文逗号均解析、自动 trim、空段剔除**）与自定义字段 api_key | SP-F-909/108 |
| B7.5 | CLI 侧核对：`%SP% item get --db "%APPDATA%\SuperPasswdKey\vault.kdbx" gui-created` | CLI 读回与 GUI 所填一致（含 `Tags:` 行与 api_key） | 互操作 | 

### B8. 编辑（SP-F-909/103）

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B8.1 | gui-created 详情点"编辑" | 表单预填全部字段（含标签回填为逗号串、自定义字段行预填） | |
| B8.2 | 改标题为 `gui-created-2`、改密码、**自定义字段 api_key 值改为 k-2**、再"添加字段"一行 `env=prod` → 保存 | 成功；列表选中态跟到新标题；详情显示新密码掩码、api_key=k-2、env=prod | |
| B8.3 | 自定义字段行 ✕ 删除 `env` 行 → 保存 | 保存后详情不再显示 env | |
| B8.4 | CLI 核对：`... item get gui-created-2` | 新标题可读（旧名 get 退出 3）；api_key=k-2；Tags 保持 | |
| B8.5 | （可选）KeePassXC 打开主库目检 | gui-created-2 的**历史记录含 1 条旧快照**（edit_entry update_history 语义，SP-F-103）；Tags 在 KeePassXC 条目中互认可见 | |

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
| B11.1 | 顶栏"设置"：空闲锁定改 **1 分钟** → 保存 | 提示"已保存：自启即时生效；超时配置重启后生效" | |
| B11.2 | 关闭应用重新启动 → 解锁 → 顶栏"设置" | 空闲锁定显示 1 分钟（持久化生效） | |
| B11.3 | 解锁后**闲置 1 分钟**（不点击不输入） | 自动切回解锁面板（顶栏倒计时同步归零）；重新解锁正常 | SP-F-903 |
| B11.4 | 勾选"开机自启"保存 → `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v SuperPasswdKey` | 值存在（带引号 exe 全路径）；**取消勾选**保存后值清零 | SP-F-905 |
| B11.5 | 测试完把空闲锁定改回原值（如 5 分钟） | — | |

### B12. 锁定语义与互操作

| # | 操作 | 预期结果 | 实际结果 |
|---|---|---|---|
| B12.1 | 解锁后另开终端：`%SP% item list --db "%APPDATA%\SuperPasswdKey\vault.kdbx" --tag dev` | CLI 独立解锁（再输一次主密码）后正常列出——两进程并存、会话互不干扰（每进程首次主密码，DR-013） | |
| B12.2 | GUI 解锁期间 CLI **只读**操作（list/get） | 均正常（只读互不影响） | |
| B12.3 | ⚠️ 全程避免：GUI 解锁期间在 CLI/KeePassXC **修改**主库（MVP 无外部修改检测，GUI 写可能覆盖外部改动）——见 design_docs/16 §8.2 并发注意 | — | |
| B12.4 | （浏览器填充环境已配置者）扩展走 NM 实例填充 | 不受本轮影响（NM 单进程嵌入逻辑未动） | SP-F-906/907 |

---

## C. 结果记录与问题上报

- 每项在"实际结果"列填 ✅ / ❌ / N/A；❌ 附：截图 + 复现步骤 + 当时操作序号。
- GUI 问题排查：`cd desktop/src-tauri && cargo tauri dev` 可开前端 console（devtools）观察 invoke 报错。
- CLI 问题先对照 `docs/manual/sp_cli_manual.md` 对应节（每项已标注节号）——手册口径"程序输出与手册不一致时以程序输出为准"，但仍请上报以修手册。
- 覆盖映射：B2→SP-F-901；B3/B11/B12→902/903/904/905；B4→908/108；B5/B7/B8/B9→909/103/707；B6→601；B10→910；A 组→102/104/105/108 及手册 §13 退出码语义。
- 测试完成：把本文件中"实际结果"列的结论留档（可直接提交回仓或归档到 docs/reviews/），主库按 0.4 备份还原（如需）。
