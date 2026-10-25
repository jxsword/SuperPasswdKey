# 分报告 G：批量导入/导出与迁移能力横向调研

> 子代理 T7 产出 ｜ 调研日期：**2026-10-25**（文中"核实日期"均指本日）
> 服务对象：F9（批量自动化导入/迁移）、F10（批量导出，加密归档 + 可选明文导出）
> 约定：所有示例命令使用占位符（`<文件>`、`<路径>`、`<目标库>`），不含真实凭据。
> 所有 URL 均于核实日期实测可达（HTTP 200 或符合预期的重定向/404），标注方式见文中。

---

## 0. 一页结论

| 维度 | 结论 |
|---|---|
| "先迁入哪个"（F9/F10 维度最优解） | **Bitwarden**。CLI 导入格式最多（60+）、唯一提供"全量含附件 zip 导出 + 密码保护加密导出 + `--raw` stdout 流式导出"三者齐备的主流软件 |
| 附件保全最优（离线场景） | **KeePassXC/KDBX**。KDBX 单文件即全量加密导出（含附件/TOTP/自定义属性），且 `keepassxc-cli export` 原生输出 stdout |
| 最不适合作为迁移枢纽 | 1Password（无加密导出、CLI 导入命令文档不透明）、Infisical（对象模型是环境变量，不是凭据条目） |
| F9 的关键空白点 | **没有现成工具**能"扫描散落文件/脚本/环境变量 → 识别 → 导入密码库"。gitleaks/trufflehog 只出"泄漏报告"，Infisical 只吃单个 .env，dotenv 系只有目录树合并小工具 → 自研 F9 有真实价值 |
| F10 安全红线 | 对齐业界：默认加密归档；明文导出须显式 flag + 重输主密码确认 + 强警告 + 用后即删提示；提供 stdout 流式通道避免临时文件落盘 |

---

## 1. 各软件导入能力

### 1.1 1Password

**Web / 桌面端导入器**（https://support.1password.com/import/ ，核实日期 2026-10-25，实测可达）

支持直接导入的来源：Chrome、Firefox、Edge、Brave、Safari、Bitwarden、Dashlane、Delinea Secret Server、KeePass、KeePassXC、Keeper、LastPass、RoboForm；以及 **1PUX（1Password Unencrypted Export）**——Mac/Windows/Linux/iOS/Android 全平台可导入；CSV 仅限 Login/Credit Card/Secure Note 三类。1PIF 旧格式只能导入 1Password 7（官方宣布 1Password 7 于 2027 年初退役）。移动端 iOS/iPadOS 26+ / Android 14+ 支持凭据交换标准（CXP）直接互迁（含 passkey）。

**CLI `op import`**：

- `op import` 命令**确实存在**（第三方迁移指南实测示例 `op import bitwarden <bitwarden导出.json>`、`op import lastpass <csv文件>`、`op import 1password <导出.1pux>`；1Password 自家安全审计报告（Secfault Security，1Password 8 for Mac）中可见 `op-import-export` 模块与 `onepux/v2.rs` 翻译层，佐证 CLI 与桌面端共用导入器代码）。
- **重要发现（负向）**：截至核实日期，官方 CLI 参考文档**已不再收录 import/export 命令**。旧文档地址 `developer.1password.com/docs/cli/reference/management-commands/import` 现 301 重定向到 `www.1password.dev/cli/reference/management-commands/import` 并 **404**（2026-10-25 curl 实测）；新版 `www.1password.dev/cli/reference.md` 的命令清单（account/item/vault/read/run/inject/signin/document/…）**不含 import/export**；`releases.1password.com/developers/cli` 与 `app-updates.agilebits.com/product_history/CLI2` 的近期版本记录（最新稳定 **2.41.1，2026-10-21**；beta **2.43.0-beta.02，2026-10-22**）中亦无 import 命令的引入/移除条目。
- **判定**：`op import` 可用但处于"官方文档不背书"状态；格式权威清单无法从官方文档获取，实施前须用本地 `op import --help` 实测确认。这本身就是 1Password 在 CLI 迁移维度的减分项。

### 1.2 Bitwarden

**CLI `bw import`**（https://bitwarden.com/help/cli/ ，核实日期 2026-10-25）

- 语法：`bw import <format> <路径>`；组织库加 `--organizationid`。官方原话："Bitwarden supports lots of formats for import, too many to list here! Use `bw import --formats` to return the list in your CLI"。
- 加密 json 导入会提示输入密码。

**Web vault 导入器支持清单**（https://bitwarden.com/help/import-faqs/ ，核实日期 2026-10-25，逐条引录节选）：1Password (1pif)、1Password 6&7 Windows (csv)、1Password 6&7 Mac (csv)、**1Password (1pux)**、Dashlane (json/csv)、**KeePass (kdbx)**、**KeePass 2 (xml)**、KeePassX (csv)、Keeper (csv/json)、LastPass (csv)、ProtonPass (json)、RoboForm (csv)、Chrome/Edge/Firefox/Safari/macOS (csv)、Enpass (csv/json)、Avast/Avira/Bitwarden 系等 **60+ 条目**（全表见上述 URL）。未列出的格式建议手工转成 Bitwarden .csv/.json。

- 对 1Password 的 1pux 导入官方指南（https://bitwarden.com/help/import-from-1password/ ，核实日期 2026-10-25）明示：**附件不随导入**（"Upload file attachments to the new vault individually"），Sends 需重建，部分条目类型不可导入。

### 1.3 KeePassXC

**核实来源**：`keepassxc-cli(1)` man 页源码（GitHub 仓库 develop 分支 `docs/man/keepassxc-cli.1.adoc`，经 GitHub API 读取，核实日期 2026-10-25）；`src/format/CsvExporter.cpp`（同日经 GitHub API 读取）。

- **CLI 有 `import`/`export` 子命令**（与常见误解相反）：
  - `keepassxc-cli export [选项] <数据库>`：把数据库导出到 **stdout，默认 XML** 格式；
  - `keepassxc-cli import [选项] <xml> <新数据库>`：把 XML 导出的数据库导入**新建**的 KDBX 4 库；
  - `keepassxc-cli attachment-export --stdout <数据库> <条目> <附件名>` / `attachment-import`：单附件粒度的导入导出。
- GUI 侧：CSV 导入（带列映射向导）、KDB 1.x 可导入 KDBX；导出见第 2.3 节。
- KDBX 互转：KDBX 3.1 / 4 为原生格式，KDBX 2 打开后自动升级；.kdb 导入支持，导出 .kdb 不支持（https://keepassxc.org/docs/ ，核实日期 2026-10-25）。
- **限制**：CLI 不支持 CSV 导入导出；1PUX 无任何入口（需经 Bitwarden 或 KeePass 2 XML 中转）。

### 1.4 Vaultwarden

一句带过：Vaultwarden 复用 Bitwarden 全部客户端与导入导出格式（`bw import/export` 及 Web 导入器行为一致），本报告不再单列。注意其作为第三方服务端，个别新格式（如 1pux 解析）历史上出现过兼容性问题（社区反馈，未逐项复核）。

### 1.5 Infisical

**核实来源**：Infisical CLI 参考页 https://infisical.com/docs/cli/reference.md （核实日期 2026-10-25，实测可达；`secrets set` 详情节在该页被截断，另以 CLI 快速上手页 https://infisical.com/docs/cli/usage 交叉印证）；GitHub issue #2108。

- **不存在 `infisical import` 命令**。CLI 参考中无任何名为 import 的命令（2026-10-25 核实）。
- 导入 .env/.yaml 文件的官方途径是 **`infisical secrets set --file <路径>`**（`infisical secrets set`："Create secrets, or update the value of secrets that already exist"；`--file` 的设计动机之一就是避免值进入 shell 历史）。一次只吃一个文件，无目录扫描、无跨文件去重。
- 平台侧迁移：Web 端 external migrations（Doppler、Vault，https://infisical.com/docs/documentation/platform/external-migrations/overview.md ，核实日期 2026-10-25）与 secret syncs（1Password、Vercel、AWS Secrets Manager 等 40+ 目标的双向同步，https://infisical.com/docs/integrations/secret-syncs/overview.md ，核实日期 2026-10-25）。这些是"平台对平台同步"，不是 CLI 批量导入。
- 当前最新 release：v0.167.2（2026-10-23，GitHub API 核实）。

### 1.6 gopass

**核实来源**：GitHub 仓库 `gopasspw/gopass` `docs/commands/*.md`（master 分支，经 GitHub API 读取，核实日期 2026-10-25）。

- **没有跨密码管理器的批量导入命令**。命令集中无 import。
- `gopass convert`：仅用于**后端迁移**（storage + crypto 组合，如 gpg→age），不是外部数据导入；官方警告转换 RCS 后端会丢失部分 git 历史。
- `gopass copy / move`：单条或**整目录**密钥在库内/挂载点间复制（跨 mount 支持），实现上会解密再加密全部受影响条目。
- 实际迁移桥是第三方 **pass-import**（https://github.com/roddhjav/pass-import ，GPL-3.0，915★，最后提交 2026-06，GitHub API 核实日期 2026-10-25）：支持从 1Password/Bitwarden/KeePass 家族/LastPass 等 40+ 来源导入，**目标端明确支持 gopass**（`pass import gopass <路径>`）。
- 最新版本 v1.17.4（2026-10-10，GitHub API 核实）。

---

## 2. 各软件导出能力

### 2.1 1Password

- **`op export` 不存在**。截至核实日期，官方 CLI 参考（https://www.1password.dev/cli/reference.md ）无 export 命令，发布记录亦无。脚本化全量导出只能用 `op item list` + `op item get --fields ...` 组合拼装（明文过 stdout，需自防 shell 历史与管道残留）。
- **桌面/Web 导出**（https://support.1password.com/export/ ，核实日期 2026-10-25，关键句逐条引录）：
  - 1Password 8 桌面端：导出 **.1pux 或 CSV**；CSV **只导出 Login 和 Password 两类条目**且字段受限，官方建议"需要附加字段就用 .1pux"；仅 1Password 8 支持一次导出多个库。
  - 1PUX 为**明文**：官方原句 "**Exported data files are not encrypted. They are stored in plaintext.**"——此前调研结论"1PUX/CSV 均明文"复核**属实**。
  - 确认机制：导出前**必须输入账户密码**（SSO 解锁则走 IdP）。
  - 警告与残留：官方建议不要邮件传输、不要存网盘、"Delete them when you are done with them"。
  - **附件**：官方导出页未明确 1PUX 是否含附件（第三方工具链如 Strongbox 迁移指南确认 1PUX 可带附件与自定义字段，属第三方佐证）。
  - **Passkey**：桌面端**不可**导出 passkey（官方建议逐网站重建）；仅 iOS/Android 经 CXP 迁移可带 passkey。

### 2.2 Bitwarden

（https://bitwarden.com/help/cli/ 与 https://bitwarden.com/help/export-your-data/ ，核实日期 2026-10-25）

- 语法：`bw export [--output <文件>] [--format <格式>] [--password <密码>] [--organizationid <orgid>]`
- 格式：默认 csv；`json` / `encrypted_json` / **`zip`**；`--raw` 可把导出**直接输出到 stdout**（不落临时文件）。
- **附件批量导出痛点已解决（重大更新）**：`--format zip` 导出"包含一个 .json 文件和你的附件"（原文 ".zip (with attachments) (includes a .json file and your attachments)"）；Web/桌面/浏览器扩展同样提供 .zip（含附件）。zip 仅限个人库。版本核实：当前 CLI **cli-v2026.10.0（2026-10-15）**；zip 导出不晚于 cli-v2026.7.0（该版 release note 即在修复 zip export 格式细节，GitHub releases 核实）。
- **加密导出两档**：Account-restricted（只能导回原账户，换加密密钥后旧文件不可解）与 Password-protected（`--password` 指定，可导入任意账户）。
- 字段覆盖：**仅 json 系导出含 cards/identities/passkeys/SSH keys**；CSV 不含。trash 与 Sends 任何格式都不导出。旧版 passkey（2024.3.1 前创建）导出"不应作为长期备份"。
- 确认机制：Web/扩展/桌面导出需**输入主密码或邮箱验证码**确认；CLI 由登录态背书。官方明文警告：推荐 encrypted json；未加密文件不要走邮件等不安全通道、"delete the file immediately after use"；解密全部在本地完成，明文不上网。

### 2.3 KeePassXC / KDBX

（man 页与源码核实日期 2026-10-25）

- **KDBX 文件本身就是全量加密导出**：单文件携带全部条目 + 附件 + TOTP + 自定义属性 + 历史，这是四家中唯一"原生即加密全量导出"的设计。
- 明文导出：GUI 支持 CSV / XML / HTML；CLI `keepassxc-cli export <数据库>` 输出 **XML 到 stdout**（可 `> <目标>` 重定向或接管道）。无 CSV 的 CLI 导出。
- **CSV 列（源码 `CsvExporter.cpp` 逐列核实）**：`Group, Title, Username, Password, URL, Notes, TOTP, Icon, Last Modified, Created` —— **TOTP 保留**（otpauth 设置串），**附件与非默认自定义属性丢失**。
- 确认机制：无独立"重输主密码"步骤，但打开库本身需要凭据；导出为明文文件无额外警告页（比 1P/BW 弱）。

### 2.4 明文导出安全控制横向对比

| 软件 | 明文导出前的确认 | 明文风险警告 | stdout 流式（避免落盘） | 加密导出选项 |
|---|---|---|---|---|
| 1Password | 重输账户密码 | 有（官方文档原句） | 无（桌面导出为文件；脚本化组合命令可到 stdout） | **无**（1PUX/CSV 均明文） |
| Bitwarden | 重输主密码/邮箱验证码 | 有（推荐加密 json） | **有**（`--raw`） | **有**（encrypted_json + 可选口令；zip 含附件） |
| KeePassXC | 打开库凭据即确认 | 弱 | **有**（`keepassxc-cli export` 原生 stdout） | 天然（KDBX 加密，但"加密导出到新文件"需另建库） |
| Infisical | 登录态 | n/a（secrets get 有 --plain 辨识） | 有（CLI 天然 stdout） | n/a（平台端存储加密） |

---

## 3. 开发者秘密收集工具链（F9 支撑）

### 3.1 gitleaks

- 官网/源码：https://github.com/gitleaks/gitleaks （MIT，30.3k★，最新 v8.30.1，2026-03-21；GitHub API 核实日期 2026-10-25）
- 能力（README 帮助输出逐条核实）：三种扫描模式 **`git`**（git log -p 扫历史）/ **`dir`**（目录与文件，含归档递归 `--max-archive-depth`）/ **`stdin`**；报告格式 `--report-format json|csv|junit|sarif|template`；基线机制 `--baseline-path`（存量降噪）；默认规则文件 `config/gitleaks.toml` 内 235 个规则/allowlist 块（2026-10-25 计数核实）。
- **重要信号**：README 顶部警告 Gitleaks 已 **feature complete**，作者转向新项目 betterleaks——选型时注意其进入维护期。
- 适用性：**F9"导入前发现"的理想引擎之一**——JSON finding 含 `File/Line/RuleID/Secret/Fingerprint`，可脚本化转换为 Bitwarden CSV 或自定义导入清单（name 用 RuleID+文件路径，value 用 Secret）。

### 3.2 trufflehog

- 官网/源码：https://github.com/trufflesecurity/trufflehog （AGPL-3.0，28.7k★，最新 v3.99.7，2026-10-24；GitHub API 核实日期 2026-10-25）
- 能力：**900+ 检测器**（`pkg/detectors` 目录 911 项，GitHub API 核实）；对 20+ 最常见凭据类型做**活体验证**（verified results）；`--json` 结构化输出（含 SourceMetadata/DetectorType/Raw/Verified）。
- 扫描源（`pkg/sources` 目录逐项核实，2026-10-25）：git、**filesystem**、docker、s3、gcs、github、gitlab、jenkins、circleci、postman、syslog、stdin、huggingface、elasticsearch 等。
- **环境变量：不支持**。`pkg/sources` 中无 env 源（这是对"trufflehog 能否扫环境变量"疑问的负向核实结论——它扫的是 git 历史/文件系统/镜像等"载体"，不是进程环境）。要覆盖 env vars 需自行导出快照到临时文件再 `filesystem` 扫描（注意临时文件残留）。
- 适用性：比 gitleaks 多了"验证是否仍然有效"，适合 F9 导入时**顺手剔除失效密钥**；但 AGPL-3.0 与体积较大，作为嵌入式依赖不如 gitleaks 轻便。

### 3.3 detect-secrets（Yelp）

- 官网/源码：https://github.com/Yelp/detect-secrets （Apache-2.0，4.7k★，最后推送 2026-04-02；GitHub API 核实日期 2026-10-25）
- 能力：**基线管理模式**——`detect-secrets scan > .secrets.baseline` 建立存量基线，后续只报新增；插件化（`--list-all-plugins` / `--disable-plugin`）；`--all-files` 扫描非 git 跟踪文件。
- 适用性：基线思想值得 F9 借鉴（首次导入做全量快照，之后增量）；但其定位是 CI 防泄漏而非导入器。

### 3.4 .env 管理现状（诚实结论：没有现成的"扫描→汇总→导入"工具）

- 调研（2026-10-25，Web 检索 + GitHub）：dotenv 系工具生态里**不存在**"扫描目录树下所有 .env + shell rc + systemd EnvironmentFile + docker-compose environment + CI 变量 → 识别 → 汇总导入密码库"的现成 CLI。最接近的小工具：
  - recursenv（https://github.com/douglance/recursenv ）：Rust，从当前目录向上遍历**合并** .env——只做目录树合并，不做密钥识别/导入；
  - envmerge（https://github.com/yinheli/envmerge ）：多 .env 智能合并、保注释——同上。
- Infisical `secrets set --file` 只吃单个 .env/.yaml；gitleaks/trufflehog 把 .env 内容当"泄漏"输出报告，不做结构化键值提取入库。
- **判定：F9 的"扫描→识别→去重→分类→批量导入"全链路在市场上是空白，属自研正当价值点。**

---

## 4. 迁移闭环矩阵

路径标注：✅ 直接支持 ｜ 🔄 需中转格式 ｜ ❌ 不支持；括号内为关键字段说明。
字段保留原则先列三条硬事实（均有官方出处，见第 1、2 节引用）：
① Bitwarden 任何导入格式**不带附件**；② 1Password 桌面导出 1PUX **明文、无 passkey**、CSV 仅 Login/Password；③ KeePassXC CSV **保留 TOTP** 但丢附件与自定义属性。

| 源 ↓ 目标 → | 1Password | Bitwarden | KeePassXC/KDBX | pass/gopass | Infisical |
|---|---|---|---|---|---|
| **1Password** | — | ✅ 1pux（TOTP/自定义字段随迁；**附件不随迁**；官方指南要求逐个重传） | 🔄 经 Bitwarden CSV 或 KeePass 2 XML（附件/TOTP 大概率丢失，须抽查） | ✅ pass-import（`pass import` 源列表含 1password） | 🔄 secret-syncs 单向同步（对象模型不同，仅环境变量类） |
| **Bitwarden** | ✅ 桌面导入器 / `op import bitwarden <json>`（CLI 命令文档不透明，需实测） | — | 🔄 经 CSV（丢附件；TOTP 列是否被 KeePassXC CSV 向导映射**未核实**，导入后抽查） | ✅ pass-import | 🔄 secret-syncs（双向，含附加属性映射） |
| **KeePassXC/KDBX** | ✅ 桌面导入器支持 KeePassXC（kdbx） | ✅ `bw import keepass <kdbx>` / `keepass2xml <xml>`（附件**不随迁**；TOTP 依赖格式映射，抽查） | — | ✅ pass-import（源含 keepass/keepassx/keepassx2） | ❌（无 KDBX 通道；需手工转 .env/CSV） |
| **pass/gopass** | ❌ 无官方通道 | 🔄 无直接格式；pass 是纯文本库，可脚本转 BW CSV/JSON（gpg 解密后） | 🔄 脚本转换 CSV→KeePassXC 导入 | — | 🔄 gopass 无 Infisical 同步；可脚本导出 .env |
| **Infisical** | 🔄 经其平台"secret syncs"导入 1Password 数据（仅 secrets） | 🔄 同左（Secrets Manager 类同步） | ❌ | ❌ | — |

**TOTP/Passkey 字段保留速查**（截至 2026-10-25）：

| 格式 | TOTP | Passkey | 附件 |
|---|---|---|---|
| 1PUX（1P 明文导出） | ✅ 含 | ❌ 桌面导出不含（仅移动 CXP 含） | ✅ 含（第三方佐证；官方文档未明示） |
| Bitwarden json/encrypted_json | ✅ 含 | ✅ 含（cards/identities/SSH keys 仅 json 系） | ❌ |
| Bitwarden csv | ✅ 含 totp 列 | ❌ | ❌ |
| Bitwarden zip | =json+附件打包 | ✅（随 json） | ✅ |
| KeePassXC CSV | ✅ 含 TOTP 列（源码核实） | ❌ | ❌ |
| KDBX | ✅ | ✅（作为条目属性可存） | ✅ |

> 诚实声明：表中"TOTP 映射"凡标"抽查"处，官方文档均未逐字段声明映射结果，属导入器实现细节，迁移后必须人工抽查；本报告不凭记忆断言。

---

## 5. 结论

### 5.1 F9/F10 维度"先迁入哪个"的最优解

**Bitwarden 为第一迁移枢纽，KeePassXC 为附件保全与离线备份的落点。**理由：
1. 导入格式覆盖面第一（60+，含 1pux/kdbx/keepass2xml），`bw import --formats` 机器可读，适合作为 F9 的中介格式层；
2. 导出能力最完整：`bw export --format zip`（全量+附件）、`--format encrypted_json --password`（口令保护加密导出）、`--raw`（stdout 流式，避免临时文件）——**恰好覆盖 F10 的两种模式需求**；
3. KeePassXC 补位：KDBX 天然加密全量（附件不丢），`keepassxc-cli export` 原生 stdout，适合做"加密归档"目标；
4. 1Password 迁入能力尚可但 CLI 文档不透明（import/export 命令脱离官方参考），且**无加密导出**，不宜作枢纽；Infisical 对象模型（环境变量）与凭据库不同构，仅在"开发者 secrets"子域有价值。

### 5.2 自研 SuperPasswdKey F9/F10 的设计参考

**F9 输入格式清单（按优先级）**：
- 密码库格式：`1pux`、`bitwardencsv` / Bitwarden json、`keepass2xml`、`kdbx`（只读解析）、通用 CSV（列映射向导，对齐 KeePassXC 的 10 列与 Bitwarden CSV 表头）；
- 散落密钥文件：`id_rsa`/`*.pem`/`*.ppk`（识别私钥指纹去重）、`.env*`（dotenv 语法）、任意 config（gitleaks 规则引擎驱动）；
- 脚本/编排来源：shell rc（`export K=V` 行）、systemd `EnvironmentFile=`、docker-compose `environment:`/`env_file:`、GitHub Actions/GitLab CI 变量段；
- 环境变量快照（自研：进程 env 枚举——gitleaks/trufflehog 均不内置，见 3.2）。

**F9 发现引擎**：内嵌 gitleaks 规则集（gitleaks 已进维护期，规则 TOML 可静态抽取；trufflehog 的 AGPL 与验证逻辑可后续作为可选插件），输出统一 finding 模型 `{source, file, line, ruleId, secret, fingerprint}` 再走导入管道。

**F9 去重与分类策略**：三级主键（① 密钥值哈希精确匹配；② URL+用户名条目合并；③ 指纹/前缀匹配识别同一密钥的多个副本）；冲突不清除——保留全部副本并打 `duplicate/conflict` 标记人工裁决；分类按来源路径聚类（`来源类型/项目路径/规则名`）。

**F10 安全红线（对齐第 2.4 节业界基准）**：
1. 默认输出**加密归档**（口令+主密钥双选项；对齐 Bitwarden 的 password-protected 与 account-restricted 两档语义）；
2. 明文导出必须：显式 flag（如 `--plaintext`）+ 交互重输主密码 + 明确警告文案 + "用后即删"提示（对齐 1P/BW 原文措辞）；
3. 一等支持 **stdout 流式导出**（对齐 `bw export --raw` 与 `keepassxc-cli export`），避免明文临时文件落盘；
4. 残留治理提示：引用 gopass ADR A-08 的结论——现代存储（SSD/日志文件系统）上 shred 不可靠，因此**重点是"尽量不落盘"而不是"可靠擦除"**；
5. 导出清单必须明示**字段损失**（哪些类型/字段不在该格式中——对齐 Bitwarden "json 才含 passkeys" 的显式声明做法）。

---

## 附录 A：核实记录（全部核实于 2026-10-25）

| 主题 | URL | 结果 |
|---|---|---|
| 1P Web 导入器清单 | https://support.1password.com/import/ | 200，清单如 1.1 |
| 1P 导出与明文声明 | https://support.1password.com/export/ | 200，"not encrypted...plaintext" 原句 |
| 1P CLI 参考（新站） | https://www.1password.dev/cli/reference.md | 200，无 import/export 命令 |
| 1P 旧 op import 文档 | https://developer.1password.com/docs/cli/reference/management-commands/import/ | 301→404（curl 实测） |
| 1P CLI 版本 | https://releases.1password.com/developers/cli ｜ https://app-updates.agilebits.com/product_history/CLI2 | 稳定 2.41.1（2026-10-21）/ beta 2.43.0-beta.02 |
| bw CLI import/export | https://bitwarden.com/help/cli/ | 200，含 `--format zip`、`--raw`、`--password` |
| bw 导出详解 | https://bitwarden.com/help/export-your-data/ | 200，zip 含附件、仅 json 含 passkeys |
| bw 导入格式全表 | https://bitwarden.com/help/import-faqs/ | 200，60+ 格式 |
| bw←1P 迁移指南 | https://bitwarden.com/help/import-from-1password/ | 200，附件不随迁 |
| bw CLI 版本 | https://github.com/bitwarden/clients/releases | cli-v2026.10.0（2026-10-15） |
| keepassxc-cli man 页 | GitHub API `repos/keepassxreboot/keepassxc/contents/docs/man/keepassxc-cli.1.adoc`（develop） | 200，含 export/import/attachment-* 子命令 |
| KeePassXC CSV 列 | GitHub API `.../contents/src/format/CsvExporter.cpp` | 200，10 列含 TOTP |
| KeePassXC 版本/格式 | https://keepassxc.org/docs/ ＋ GitHub API releases | 2.8.0（2026-10-21）；KDBX 3.1/4 |
| Infisical CLI 参考 | https://infisical.com/docs/cli/reference.md ｜ https://infisical.com/docs/cli/usage | 200，无 import 命令；`secrets set --file` |
| Infisical 平台迁移 | https://infisical.com/docs/documentation/platform/external-migrations/overview.md | 200 |
| gopass 命令文档 | GitHub API `repos/gopasspw/gopass/contents/docs/commands/{convert,copy}.md` | 200，convert 仅后端迁移 |
| pass-import | https://github.com/roddhjav/pass-import | GPL-3.0，目标端含 gopass |
| gitleaks | https://github.com/gitleaks/gitleaks | v8.30.1，MIT；git/dir/stdin；报告格式 5 种；feature-complete 警告 |
| trufflehog | https://github.com/trufflesecurity/trufflehog | v3.99.7，AGPL-3.0；`pkg/sources` 无 env 源；detectors 目录 911 项 |
| detect-secrets | https://github.com/Yelp/detect-secrets | Apache-2.0；基线模式原句引录 |
| .env 聚合工具 | https://github.com/douglance/recursenv ｜ https://github.com/yinheli/envmerge | 均为小众合并工具，非导入器 |

（raw.githubusercontent.com 与 archive.org 在本机网络不可达，GitHub 内容一律改经 GitHub Contents API 读取核实。）
