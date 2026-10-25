# 分报告 C：KeePass 生态与"云盘同步"路线调研

> 调研日期：2026-10-25。基准：1Password（收费全功能）。
> 核心 F1–F10：F1 自动保存/填充；F2 本地加解密；F3 云端只存密文；F4 密钥文件本地+主密码不上云；F5 跨平台；F6 私有部署（无服务器）；F7 CLI；F8 临时环境变量注入；F9 批量导入；F10 批量导出。
> 所有 URL 均经 WebFetch 实测可达，标注"（核实: 2026-10-25 可达）"。示例命令一律使用占位符。

---

## 0. 结论速览

- "KDBX 单个加密数据库文件 + 任意云盘同步"是 KeePass 生态的**原生形态**：云端只见密文（F3 天然满足），无服务器（F6 天然满足），零订阅费。
- 多设备并发写冲突由 **KDBX 条目级合并机制**兜底（KeePassXC merge / KeePass trigger 同步 / KeePassDX Merge from），但前提是各端使用**支持合并的客户端**、且不要让云盘"整文件覆盖"。
- 最佳国内云盘组合建议：**坚果云 WebDAV**（官方应用密码机制成熟）或 **Syncthing**（P2P 无云）；客户端组合：桌面 **KeePassXC** + Android **KeePassDX** + iOS **KeePassium/Strongbox**。
- 生态内唯一原生实现 F8（临时环境变量注入）的是 **gopass**（`gopass env`，等价 `op run`）；KeePassXC 的 `keepassxc-cli` 只能取值后自行包装。
- 主要风险：假冒官网/恶意安装器分发事件多发（2023、2025），必须只从官网下载；KeePass 原版有 CVE-2023-32784 主密码内存泄露史（2.54 已修复）。

---

## 1. KeePass 2.x（原版）

| 项 | 内容 |
|---|---|
| 官网 | https://keepass.info/ （核实: 2026-10-25 可达） |
| 文档 | https://keepass.info/help/base/index.html （核实: 2026-10-25 可达，安全页 keepass.info/help/base/security.html 同站可达） |
| 源码 | SourceForge 项目托管（官网明确"OSI 认证开源、可查全部源码"；官网页内链接指向 sourceforge.net 论坛/新闻，下载与源码经 SourceForge 分发。核实: 2026-10-25 官网可达；SourceForge 源码树未单独实测） |

**最新版本**：2.61.1（2026-05-01 发布；官网核实）。GPL 类开源、全免费。

**格式与加密**：KDBX 4 规范由原版定义（keepass.info/help/kb/kdbx_4.html，核实： 2026-10-25 可达）：
- 算法：AES-256、ChaCha20（2.35 起可换用 Twofish）；
- KDF：AES-KDF 或 Argon2（该页成文时仅 Argon2d；Argon2id 随 KDBX 4.1 引入，KeePass 2.48+ 支持）；
- 完整性：KDBX4 强制 HMAC-SHA-256 头部认证 + 每块 Encrypt-then-MAC；
- 附件移入加密内部头，不再 Base64 内嵌。

**同步能力**：KeePass 2.x 支持"从文件/URL 打开与同步"——官方 trigger 示例（keepass.info/help/kb/trigger_examples.html，核实： 2026-10-25 可达）给出两套现成方案：
- "Dropbox/网盘同步软件"触发器：工作库放网盘目录之外，网盘内放副本；"保存数据库文件"事件触发"Synchronize active database with a file/URL"，用后自动开关触发器防止死循环；
- Box.com 触发器：保存时用某条目里的凭据向 WebDAV 同步。
- 官方说明：冲突时保留最新修改时间，旧变更进入条目历史（history），不丢数据。

**CLI（F7/F8）**：KeePass 2.x 本体无现代 CLI，靠 **KPScript** 官方工具（单命令 + KPS 脚本两种模式，keepass.info/help/v2_dev/scr_sc_index.html，核实： 2026-10-25 可达），可 `ListEntries`/`GetEntryString` 等。没有等价 `op run` 的环境变量注入，需要 PowerShell/批处理自行包装。

**社区客户端生态**：KeePass 是整个生态的"格式定义者"，任何 KDBX 兼容客户端都可参与同一份数据库。

---

## 2. KeePassXC（跨平台桌面主力）

| 项 | 内容 |
|---|---|
| 官网 | https://keepassxc.org/ （核实: 2026-10-25 可达） |
| 文档 | https://keepassxc.org/docs/ （含 FAQ；核实: 2026-10-25 可达）；用户指南 https://keepassxc.org/docs/KeePassXC_UserGuide.html （核实） |
| 源码 | https://github.com/keepassxreboot/keepassxc （GPLv3；核实: 2026-10-25 可达） |

**最新版本**：2.8.0（2026-10-21 发布，Qt5→Qt6、Wayland Auto-Type、"remote sync with external tools"等；官网核实）。上一稳定版 2.7.12。完全免费。

**KDF（F2）**：数据库设置 → 安全 → 高级设置中可选 AES-KDF（KDBX 3.1/4）或 **Argon2（KDBX 4，推荐）**；Argon2 分 **Argon2d（最大兼容）与 Argon2id（抗时序攻击，推荐）**，参数为内存、迭代、并行度（线程数），另有"解密时间"滑杆（官方建议 1 秒）。（UserGuide 核实）

**KDBX4/4.1**：完整支持 KDBX 4.1（2.7.0 release notes "Implement KDBX 4.1"；GitHub release 核实），即含 Argon2id。

**浏览器自动填充（F1）**：**KeePassXC-Browser** 扩展，支持 Firefox / Chrome/Chromium / Edge；架构为浏览器 → `keepassxc-proxy`（原生消息 native messaging）→ Unix socket / 命名管道 → KeePassXC 本体；proxy 随主程序附带，协议文档 keepassxc-protocol.md 公开，扩展仓库 https://github.com/keepassxreboot/keepassxc-browser （GPLv3；核实： 2026-10-25 可达）。凭据不落浏览器存储。桌面端 F1 体验成熟。

**SSH Agent 集成**：内置（文档 FAQ 有专门章节；可在解锁数据库后自动把 SSH 密钥加载进 ssh-agent，支持要求确认/生命周期选项）。1Password 同名功能的开源等价物。

**CLI（F7）**：`keepassxc-cli`（man 页 docs/man/keepassxc-cli.1.adoc，经 GitHub API 实测读取）。关键命令（均核实）：
- `show <db> <entry>`：`-a/--attributes`（含自定义属性）、`--all`、`-t/--totp`、`--show-attachments`、`-s/--show-protected`；
- `clip <db> <entry>`：`-a` 属性、`-t` TOTP、超时自动清剪贴板；
- `export -f xml|csv`（默认 XML）、`import <xml>`（仅 XML→新建 kdbx4）；
- `merge [-s] [--dry-run]`、`add/edit/ls/mv/rm/mkdir`、`attachment-export/--stdout`、`analyze`（离线 HIBP）。
- **F8 结论：没有 `op run` 式子进程注入命令**。社区通行做法是 shell 包装：
  ```bash
  export APP_TOKEN="$(keepassxc-cli show -a Password "<数据库路径>" "<条目名>")"
  <你的命令>
  ```
  或配合 direnv 的 `.envrc` 在进入目录时经 `keepassxc-cli` 取值注入（属社区组合用法，非官方功能）。缺点：每次进目录要解锁一次数据库；密钥经进程环境传递，敏感度与 `op run` 相同。

**TOTP / 附件**：条目级 TOTP（RFC 6238，GUI 与 CLI 均可取值）；附件以加密形式存储于条目（GUI 管理预览，CLI `attachment-import/export`）。

**导出（F10）/ 导入（F9）**：GUI 支持导出 CSV/XML/HTML（有格式局限说明）；CLI `export -f xml|csv`。导入：GUI 支持 CSV、1Password、KeePass 1.x/2.x XML 等多种格式；CLI 仅 XML 导入。CSV 批量导入走 GUI 即可满足 F9。

**KeeShare（组间共享）**：UserGuide 专章（核实）。按组设置 Inactive/Import/Export/Synchronize 四种模式，导出为带签名的"分享容器"（独立文件+容器密码），保存库时写出、导入侧按时间顺序合并；签名只验签发者、**无法检测回滚到旧版本**——用于团队间部分条目共享，不是全库同步方案。

**云盘同步立场**：官方文档 FAQ 明确**不内置云同步**，建议把数据库文件放在云盘同步目录（Dropbox/Nextcloud 等）由同步客户端处理（keepassxc.org/docs/ 核实）。即：KeePassXC 只打开本地文件路径，**不支持直接打开 WebDAV URL**。

---

## 3. KeePassDX（Android）

| 项 | 内容 |
|---|---|
| 官网 | https://www.keepassdx.com/ （核实: 2026-10-25 可达；页面内容极简） |
| 文档 | GitHub Wiki FAQ：https://github.com/Kunzisoft/KeePassDX/wiki/FAQ （核实）；File Manager and Sync：https://github.com/Kunzisoft/KeePassDX/wiki/File-Manager-and-Sync （核实） |
| 源码 | https://github.com/Kunzisoft/KeePassDX （GPLv3；核实） |

**最新版本**：4.5.6（2026-10-17，GitHub Releases API 核实）。免费开源（另有付费构建资助开发）。

**能力**：KDBX 1–4 读写（AES/Twofish/ChaCha20 + Argon2）、TOTP/HOTP、Android 系统级 Autofill + Magikeyboard 填充、生物识别解锁、Passkey 本地存储。

**云同步（关键结论）**：**不内置任何 WebDAV/云 API**。官方 Wiki 明确设计哲学："密码编辑器不该负责云同步"，文件访问全部委托 Android Storage Access Framework（SAF）——由第三方文件管理器/云应用提供文件 URI：
- 官方实测清单：**RCX / RSAF（rclone）可给 SAF 提供 WebDAV、SFTP 及各家云**；Material Files（FTP）；Mixplorer（SFTP/WebDAV/SAMBA）；Syncthing 可用但**警告避免并发保存（有损坏风险）**；
- 明确警示：**Google Drive（链接过期、延迟保存、丢数据风险）与 OneDrive（URI 重分配、>512KiB 文件损坏风险）不要用于 KDBX**；
- 数据合并靠自身：Save / Merge / Reload / **Merge from…**（不同 URI 跨库合并）/ Save a copy。

即：KeePassDX 侧的 WebDAV 路线 = 坚果云 WebDAV + RCX/RSAF 挂载 → SAF 打开 KDBX。可用但多一层配置。

---

## 4. Strongbox（macOS / iOS，闭源）

| 项 | 内容 |
|---|---|
| 官网 | https://strongboxsafe.com/ （核实: 2026-10-25 可达） |
| 文档 | 官网内 Pricing / Compare Features 页（核实: 2026-10-25 pricing 页可达） |
| 源码 | **未公开**（官网未见开源声明；KDBX 兼容但应用本体闭源） |

**平台**：iPhone / iPad / Mac（+ Apple Watch）。**云同步**：官网宣称"sync with every major cloud storage provider or your own server over **WebDAV or SFTP**"（iCloud、Dropbox、Google Drive、OneDrive 属其支持的主流云；具体清单在应用内）。另有 YubiKey、Passkey、2FA、密码审计等。

**收费（核实： 2026-10-25）**：
- 免费版：非商业可用，缺 AutoFill、生物解锁、自定义同步选项、安全审计等"便利功能"；
- Pro：**$2.99/月、$24.99/年（含 3 个月试用）、$124.99 买断**；一证通用全设备，支持家庭共享（6 人）；学生/活动人士可申请 12 个月免费；另有商业授权。

定位：iOS 侧体验对标 1Password 最接近的 KeePass 客户端之一，代价是闭源。

---

## 5. KeePassium（iOS + macOS，开源）

| 项 | 内容 |
|---|---|
| 官网 | https://keepassium.com/ （核实: 2026-10-25 可达） |
| 文档 | https://support.keepassium.com/ （核实；同步专页 /kb/sync/ 核实） |
| 源码 | https://github.com/keepassium/KeePassium （GPLv3；官网核实） |

**最新版本**：未在本次抓取中获得具体版本号（官网未列，App Store 页未实测）。

**云同步支持（/kb/sync/ 核实，细节最全）**：
- **WebDAV 直连：完整支持**（Nextcloud、ownCloud、Synology、QNAP、通用 WebDAV 均完整支持；Yandex.Disk WebDAV 可用）；
- **OneDrive、Google Drive、Dropbox：直连 API + Files app 双路完整支持**；
- iCloud Drive 经 Files app 完整支持；Box、pCloud、Seafile 完整支持；SMB 共享完整支持（无离线缓存）；SFTP/FTP 经第三方 Files app；
- MEGA/Sync.com 仅手动/单向；Tresorit、Syncthing/Resilio 可用但有限制。
- 注意：定价表中"Sync with corporate storage"列于 Premium，免费版的 WebDAV/企业存储可用性需在应用内确认。

**收费（keepassium.com/pricing/ 核实，欧元计价）**：
- 免费版（GPL 开源，"对多数人完全够用"）：**仅 1 个数据库**，含 OTP、Face ID/Touch ID、系统 AutoFill；
- Premium：**€19.99/年**（rent-to-own，续费年限累加）；多数据库、YubiKey、Quick AutoFill、密码审计、附件、Linked databases 等；
- Pro：**€79.99 一次性买断**（终身）；Business/Intune：€36/用户/年。

其他：官网宣称其为"唯一经过独立安全审计的 KeePass 兼容移动应用"（审计详情 /kb/security-audits/）。

---

## 6. gopass / pass（unix password store，简要）

| 项 | 内容 |
|---|---|
| 官网 | https://www.gopass.pw/ （核实: 2026-10-25 可达）；pass 官网 https://www.passwordstore.org/ （核实） |
| 文档 | https://github.com/gopasspw/gopass/tree/master/docs （官网核实）；`gopass env` 文档 docs/commands/env.md（GitHub 核实） |
| 源码 | https://github.com/gopasspw/gopass （MIT；官网核实） |

**最新版本**：v1.17.4（2026-10-10，GitHub API 核实）。

**模型**：每条密码 = 一个 GPG 加密文件（`~/.password-store` 下目录树），**git 做版本控制与同步**（`gopass git push/pull` 或直接 git），GPG 做端到端加密；跨平台（Windows/macOS/Linux）。pass（bash 原版）是同一哲学的鼻祖，gopass 是 Go 重写+增强。

**同步方式**：git 托管（GitHub/GitLab **private repo** 即可，云端只存 GPG 密文，满足 F3）或任何能同步 git 裸仓库的盘；也可配合 gocryptfs 等。与"云盘同步"路线相比：冲突由 git 合并处理（更精细），但移动端体验弱（Android 有 Android-Password-Store，iOS passforios，见 passwordstore.org 客户端列表）。

**F8 亮点——`gopass env`（核实存在，官方文档 docs/commands/env.md）**：
```
gopass env <密钥子树前缀> <命令> [参数...]
```
把该前缀下每个密钥导出为大写环境变量后**以子进程运行命令**（如 `gopass env db/prod psql ...`，`DB_PASSWORD=<secret>`），语义等价 1Password `op run`。变体：`--stdin`（改走 stdin，规避 `/proc/<pid>/environ`、`ps eww` 泄露路径，官方推荐）、`--file`（写 ramdisk 临时文件并导出 `<KEY>_FILE` 路径，学 12-factor）、`--exec`（exec 替换进程，Windows 不可用）、`--keep-case`。**这是 KeePassXC CLI 缺失、且用户需求点名的 F8 能力。**

---

## 7. 专节：KDBX + 云盘同步的跨平台实际方案（本报告核心）

### 7.1 桌面端（Windows/macOS/Linux）：本地同步客户端 vs WebDAV 直连

| 路线 | 做法 | 优劣 |
|---|---|---|
| **A. 本地同步客户端**（推荐给 KeePassXC 用户） | KDBX 放同步目录：坚果云/OneDrive 官方客户端、或 **Syncthing**（syncthing.net，核实： 2026-10-25 可达；开源 P2P、TLS、无中心服务器）持续同步本地文件 | KeePassXC 始终操作本地文件（官方 FAQ 立场即此）；延迟最低；Syncthing 零云端、F6 满分。缺点：文件可见地落在同步盘，需确认云盘不做明文副本扫描的合规顾虑；冲突靠客户端合并 |
| **B. WebDAV 直连** | **KeePass 2.x** 支持"从文件/URL 打开与同步"（官方 trigger 示例即用 URL 同步，见 §1）；KeePassXC **不支持直接打开 WebDAV URL**（官方文档建议走同步目录） | 直连免装客户端、移动端一致性好；但每次开库/保存走网络，坚果云 WebDAV 有请求频率限制（免费每 30 分钟 600 次，付费 1500 次；单文件上传上限 500MB——见坚果云官方帮助 help.jianguoyun.com/?p=2064，核实： 2026-10-25 可达，含"第三方应用密码"生成步骤）。KDBX 体积小、操作频率低，通常不触限 |

**坚果云 WebDAV 要点**（官方帮助核实）：网页登录 → 账户信息 → 安全选项 → 第三方应用管理 → "添加应用密码"，第三方客户端用"服务器地址 + 账号 + 应用密码"连接；主账号密码不外泄，可按应用单独吊销。WebDAV 服务器地址在坚果云帮助/客户端内提供（帮助页本身未列出具体域名，需在其帮助中心"WebDAV"词条获取）。

**OneDrive 注意**：桌面端 OneDrive 客户端同步 KDBX 可行，但 KeePassDX Wiki 实测 OneDrive 的 Android 文件提供器曾致大文件损坏；iOS/桌面端各自客户端成熟。若坚持 OneDrive，建议各端都走"本地文件+官方同步客户端"，禁止移动端直连其文档提供器（或改用 KeePassium/Strongbox 的 OneDrive 原生 API 直连）。

### 7.2 移动端云集成对照（逐个核实官方文档）

| 客户端 | WebDAV 直连 | SFTP | 主流云（Dropbox/OneDrive/GDrive） | 其他 |
|---|---|---|---|---|
| **KeePassDX** (Android) | ❌ 原生无；经 RCX/RSAF (rclone)→SAF 可用 | 同左（RCX/RSAF、Mixplorer） | ❌ 不建议（Wiki 明示 Google Drive/OneDrive 数据损坏风险） | Syncthing 可用但避免并发保存 |
| **Strongbox** (iOS/macOS) | ✅（官网：自家服务器 over WebDAV） | ✅ | ✅ "every major cloud storage provider" | iCloud；Pro 功能 |
| **KeePassium** (iOS/macOS) | ✅ 完整支持（Nextcloud/ownCloud/NAS/通用） | ⚠️ 经第三方 Files app | ✅ Dropbox/OneDrive/GDrive 双路完整支持 | SMB ✅、iCloud ✅（Files app）；企业存储列 Premium |

### 7.3 多设备并发写冲突：KDBX 合并机制

- **机制基础**：KDBX 条目有 UUID、逐字段修改时间与删除历史（history）。KeePass 系合并按"更新时间新者胜 + 旧版本入 history"处理，因此**两端同时编辑不同条目，合并后都不丢**；同一条目并发改则保留最新、旧值可从 history 找回。
- **KeePassXC**：GUI"Database → Merge From Database"（UUID/时间戳合并）；CLI `keepassxc-cli merge <db1> <db2> [-s]`，`--dry-run` 可先看差异。2.8.0 还新增"remote sync with external tools"。
- **KeePass 2.x**：触发器自动化（保存事件 → Synchronize URL → 自禁/自启触发器防死循环，官方示例可直接导入 XML）。
- **KeePassDX**：Save/Merge/Merge from…（可跨 URI）；官方 Wiki 建议每次修改后尽快上传。
- **KeePassium**：/kb/database-merge/ 提供合并专页 + 应用内自动备份。
- **数据丢失的真实风险场景**：
  1. 云盘客户端"最后写入者覆盖整文件"（把同事/另一设备刚保存的版本静默替换）——**规避**：保存前让客户端先拉取最新；发现同步冲突副本（"conflicted copy"）时立即用 merge 而不是覆盖；
  2. 移动端文件提供器写坏文件（KeePassDX Wiki 记录的 GDrive/OneDrive 案例；Syncthing 并发保存）——**规避**：Android 端只走 RCX/RSAF WebDAV 或原生 API 客户端；
  3. 忘记保存/断网期间本地编辑后直接被覆盖——**规避**：开启各客户端的自动备份（KeePassium 应用内备份、KeePassXC 备份选项、KeePassDX 备份到外部存储）；
  4. **密钥文件（F4）不要与 KDBX 放同一同步目录**：否则"知道云盘密码即可解开库"，F4 失效。密钥文件应走独立渠道（U 盘/另一信任设备）分发。

---

## 8. 安全性专节

- **CVE-2023-32784**（核实：多源交叉，NVD 原文页抓取为空、以下按 sysdig/wiz/vdohney PoC 一致口径）：KeePass 2.x（< 2.54）中，经键盘逐字符输入的主密码以 .NET 明文字符串残留在进程内存（含锁定后与进程退出后的 dump），**PoC 可恢复全部主密码**；触发前提是攻击者已能对本机做内存转储。**修复：2.54（2023-05）**。影响面限于本机内存可读的对手，不涉及云端文件泄露。
- **假冒站点/恶意分发**（核实：2026-10-25 检索）：
  - 2023-10：Google 广告投放 Punycode 同形异码域名假冒 KeePass 官网分发木马安装器（Malwarebytes、BleepingComputer 报道）；
  - 2025-05：Kaspersky 报告木马化 KeePass 事件复盘；WithSecure/Broadcom 报告 "KeeLoader" 假 KeePass 安装器（窃密+勒索）。
  - **对策**：只从 keepass.info / keepassxc.org / 官方应用商店下载，核对安装包签名。
- **审计**：KeePassXC 官方审计页（keepassxc.org/audits/，核实）列出两份独立审查：**ANSSI Security Visa ANSSI-CSPN-2025/16**（法国国家网络安全局认证流程，Synacktiv 执行，覆盖 2.7.9/Windows 10，2025-11-17 至 2028-11-17 有效）与 Molotnikov 源码审查（2023-01，2.7.4）。KeePassium 官网宣称已独立审计（唯一经审计的 KeePass 兼容移动应用）。KeePass 原版无同级别现代第三方审计记录，但经 20+ 年公开检验。
- **官方安全议题页**（keepass.info/help/kb/sec_issues.html，核实）当前列出 CVE-2026-86776（头部字段内存分配问题）与 CVE-2022-0725（Fedora 日志泄露敏感数据）等条目，未见收录 CVE-2023-32784（官方处理方式见其 2.54 更新）。
- **KeePassium 生态注意**：Strongbox 闭源（信任基于商店审计与厂商声誉），其余对象均开源可复核。

---

## 9. 收费对比

| 产品 | 价格 | 备注 |
|---|---|---|
| KeePass 2.x | 免费 | 开源（SourceForge） |
| KeePassXC | 免费 | GPLv3 开源 |
| KeePassDX | 免费 | GPLv3；官网 APK 免费，Play 商店版付费支持开发 |
| Strongbox | $2.99/月；$24.99/年；$124.99 买断 | 免费版缺 AutoFill/生物解锁等 |
| KeePassium | 免费 1 库；Premium €19.99/年（rent-to-own）；Pro €79.99 买断；企业 €36/人/年 | 多库/YubiKey/企业存储属付费 |
| gopass | 免费 | MIT 开源 |

对照 1Password（约 $36–60/人/年 订阅）：KeePass 路线**软件费为 0**，最多在 iOS 客户端花 €19.99/年 或 $24.99/年。

---

## 10. F1–F10 逐项对照

图例：✅ 完整支持；⚠️ 有条件/需插件/需组合；❌ 不支持。

| 功能 | KeePass 2.x | KeePassXC | KeePassDX | Strongbox | KeePassium | gopass |
|---|---|---|---|---|---|---|
| F1 自动保存/填充 | ⚠️ Auto-Type 原生，浏览器填充靠旧插件（KeeFox 已停更） | ✅ Auto-Type + KeePassXC-Browser（native messaging，Firefox/Chrome/Edge） | ✅ Android Autofill + Magikeyboard | ✅ iOS/macOS AutoFill（Pro） | ✅ 系统 AutoFill（Quick AutoFill 属 Premium） | ❌ 无 GUI（passff/browserpass 可浏览器填充） |
| F2 本地加解密 | ✅ KDBX4 AES/ChaCha20 + Argon2 | ✅ 同左，Argon2id 可选 | ✅ KDBX1–4 | ✅（实现闭源） | ✅ | ✅ GPG 端到端 |
| F3 云端只存密文 | ✅ 云盘只见 KDBX 密文 | ✅ 同左 | ✅ | ✅ | ✅ | ✅ git 上只有 GPG 密文 |
| F4 密钥文件本地+主密码不上云 | ✅ 支持 keyfile+主密码组合；**密钥文件勿与库同盘目录** | ✅ 同左 | ✅ | ✅ | ✅ | ✅（GPG 私钥本地；无主密码概念） |
| F5 跨平台 | ⚠️ Windows 为主（.NET/Mono 兜底） | ✅ Win/macOS/Linux（无移动端） | ✅ Android | ✅ iOS/macOS | ✅ iOS/macOS | ✅ Win/macOS/Linux（移动端生态弱） |
| F6 私有部署（无服务器） | ✅ 文件型天然无服务器 | ✅ | ✅ | ✅ | ✅ | ✅（git 服务器可自建） |
| F7 CLI | ⚠️ KPScript（官方脚本工具，偏自动化） | ✅ keepassxc-cli（show -a/-t、export xml/csv、merge 等，man 页核实） | ❌ | ❌ | ❌ | ✅✅ 原生即为 CLI |
| F8 临时环境变量注入 | ⚠️ KPScript 取值 + 手工包装 | ❌ 原生无 `op run` 等价物；shell/`direnv` 社区组合可覆盖 | ❌ | ❌ | ❌ | ✅ `gopass env`（--stdin/--file 变体） |
| F9 批量导入 | ✅ GUI 导入 CSV/XML/KeePass1.x 等 | ✅ GUI 多格式（含 1Password 导出）；CLI 仅 XML | ⚠️ 仅 kdbx/kdb 打开 | ⚠️ 常见格式导入 | ⚠️ KDB/KDBX/Apple Passwords 导入 | ✅ 文件树直接 cp/gopass insert |
| F10 批量导出 | ✅ KDBX 开放格式=天然全量导出；另可导 XML/CSV 明文 | ✅ GUI CSV/XML/HTML；CLI `export -f xml\|csv` | ✅ KDBX 即导出 | ✅ KDBX 即导出 | ✅ KDBX 即导出 | ✅ 目录即明文/密文文件树 |

**F1 的体验差距要点**：浏览器自动填充桌面端 KeePassXC 最佳（且不落浏览器存储）；移动端 KeePassDX/Strongbox/KeePassium 都依赖系统 AutoFill 框架，字段匹配精度弱于 1Password 的专有填充引擎，Strongbox/KeePassium 的高级匹配属付费项；KeePass 原版浏览器生态基本停摆，不建议以原版承担 F1。

---

## 11. 小结：可行性结论与最佳客户端组合建议

**结论：**"KDBX 文件 + 国内云盘"路线对用户需求**高度可行**，是 1Password 订阅之外功能覆盖度最高的免费方案：F2/F3/F4/F6 满分，F5/F9/F10 靠开放格式与客户端组合满足，F7 由 keepassxc-cli 覆盖；**唯一硬缺口是 F8（环境变量注入），KeePass 系全生态无原生实现**，需 shell 包装凑合，或 CLI 重度场景直接补一个 gopass。

**推荐组合（国内云盘优先）**：

1. **桌面（Win/macOS/Linux）：KeePassXC 2.8.0**
   - 方案 A（最简）：坚果云/OneDrive 官方客户端同步目录内的 KDBX（官方 FAQ 推荐形态）；
   - 方案 B（无云）：Syncthing P2P 同步 KDBX 目录，F6 最彻底；
   - 浏览器装 KeePassXC-Browser（F1），SSH Agent 集成即开即用。
2. **Android：KeePassDX 4.5.6 + RCX/RSAF（rclone）挂坚果云 WebDAV**——避免 Google Drive/OneDrive 文件提供器（官方 Wiki 损坏警告）。
3. **iOS/macOS：KeePassium（WebDAV 直连完整支持；多库需 €19.99/年）**，预算充足且追求 iOS 填充体验可换 **Strongbox（$24.99/年 或 $124.99 买断，WebDAV/SFTP/主流云全支持）**。
4. **CLI/开发场景加配 gopass**：`gopass env` 直接补齐 F8（`--file`/`--stdin` 变体更安全），可与 KDBX 库并存分工（服务凭据走 gopass+git private repo，个人密码走 KDBX+云盘）。
5. **冲突纪律**：全端使用支持合并的客户端（勿用"最后写入覆盖"型工具直接替换 KDBX）；开启各端自动备份；密钥文件与 KDBX 分渠道分发（F4）。
6. **供应链纪律**：仅官网下载（2023/2025 假冒站事件教训），KeePass 原版保持 ≥2.54（CVE-2023-32784）。

**待补核实项**：Strongbox 与 KeePassium 的当前具体版本号未取得；坚果云 WebDAV 服务器域名未在帮助页 2064 文中列出（需查其帮助中心 WebDAV 词条）；KeePassium 免费版 WebDAV 是否可用需装机确认（定价表将"企业存储同步"列入 Premium）。

---

## 附：已核实 URL 清单（核实: 2026-10-25 可达）

- KeePass：https://keepass.info/ · https://keepass.info/help/kb/kdbx_4.html · https://keepass.info/help/kb/trigger_examples.html · https://keepass.info/help/kb/sec_issues.html · https://keepass.info/help/v2_dev/scr_sc_index.html
- KeePassXC：https://keepassxc.org/ · https://keepassxc.org/docs/ · https://keepassxc.org/docs/KeePassXC_UserGuide.html · https://keepassxc.org/audits/ · https://github.com/keepassxreboot/keepassxc · https://github.com/keepassxreboot/keepassxc-browser ·（man 页 docs/man/keepassxc-cli.1.adoc 经 GitHub API 读取）
- KeePassDX：https://www.keepassdx.com/ · https://github.com/Kunzisoft/KeePassDX · https://github.com/Kunzisoft/KeePassDX/wiki/FAQ · https://github.com/Kunzisoft/KeePassDX/wiki/File-Manager-and-Sync ·（版本经 GitHub Releases API）
- Strongbox：https://strongboxsafe.com/ · https://strongboxsafe.com/pricing/
- KeePassium：https://keepassium.com/ · https://keepassium.com/pricing/ · https://support.keepassium.com/ · https://support.keepassium.com/kb/sync/ · https://support.keepassium.com/kb/licensing-free/ · https://github.com/keepassium/KeePassium
- gopass/pass：https://www.gopass.pw/ · https://www.passwordstore.org/ · https://github.com/gopasspw/gopass ·（`gopass env` 文档 docs/commands/env.md 经 GitHub 读取；版本经 GitHub API）
- 其他：https://syncthing.net/ · https://help.jianguoyun.com/?p=2064
- 事件来源：Malwarebytes《Clever malvertising attack uses Punycode…》(2023-10) 、BleepingComputer《Fake KeePass site uses Google Ads and Punycode to push malware》(2023-10)、Sysdig CVE-2023-32784 检测指南、vdohney/keepass-password-dumper（PoC）
