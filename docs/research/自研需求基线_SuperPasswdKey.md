# 自研需求基线：SuperPasswdKey

- 版本：v1.0（基线日期 2026-10-25）
- 数据来源：《总报告_密码管理器选型.md》及其 7 份分报告（A–G），本文不引入分报告之外的事实；示例命令一律使用占位符（如 `<你的服务令牌>`），不含真实凭据。
- 用途：若用户选择"路线 B：自研"，本文作为产品/工程需求基线（backlog + 验收标准 + 架构约束），供后续设计评审与排期使用；所有"参照"均指向分报告中经实测核实的官方文档或开源实现。
- 关联文档：《总报告_密码管理器选型.md》（选型结论与安全横向核实）；关键分报告专节：分报告A §6（op run 专节）、分报告B §3（bws run）、分报告C §7（KDBX 同步方案）、分报告D §1（注入机制对比）、分报告E（云盘管道）、分报告F §10（设计红线）、分报告G §5（F9/F10 设计参考）。

---

## 1. 定位与目标

- **定位**：在 1Password 收费全功能基础上，实现 F1–F10 全覆盖的个人/小团队密码与机密管理器，全栈开源。
- **双部署形态（核心差异点，市场上无单一产品同时做好两条轨）**：
  1. **私有部署轨**：部署在用户自己的服务器（参照 Bitwarden 官方 Lite 200MB RAM 级 / Vaultwarden 数百 MB 级 / Infisical 4GB 级的资源与暴露面数据，见 §7.1）；
  2. **无服务器云盘同步轨**：数据 = 本地加密数据库文件，经用户自选云盘（坚果云 WebDAV / rclone crypt / Syncthing）同步密文，云端只见密文（参照 Enpass 商业先例与 KeePass 生态 10 年验证，见 §7.2）。

### 1.1 F1–F10 目标状态

| 编号 | 功能 | 目标状态 | 备注 |
|---|---|---|---|
| F1 | 自动保存/浏览器填充 | P0（桌面扩展）/P1（Android）/P2（iOS） | 1Password 最深护城河，风险见 §8.1 |
| F2 | 本地加解密 | P0 | 拒绝 Infisical 式服务端持钥模型 |
| F3 | 云端只存密文 | P0 | 双轨均须满足（服务器仅存密文+二次加盐哈希） |
| F4 | 密钥文件本地 + 主密码不上云 | P0 | **独立可自定义密钥文件**——对标核实结论：全行业仅 1Password 以固定 Secret Key 近似实现且不可自定义 |
| F5 | 跨平台 Win/Linux/Android 必选，macOS/iOS 备选 | P0/P1 | 平台矩阵见 §8.2 |
| F6 | 私有部署/无服务器双形态 | P0 | 双轨架构 §7 |
| F7 | CLI | P0 | 条目 CRUD + 单值读取 + 导入导出 |
| F8 | CLI 临时环境变量注入 | P0 | 对标 `op run` 语义，验收规范 §3 |
| F9 | 批量导入（含散落密钥扫描） | P0 | 市场空白（分报告G 核实），§4 |
| F10 | 批量导出（默认加密归档+可选明文） | P0 | 行业空白（分报告G 核实），§5 |

### 1.2 信任模型与非目标

- **信任模型目标**：达到分报告F 定义的三级信任模型的 **L3**（纯本地开源 + 可复现构建）——唯一可"结构性证伪"厂商投毒的模型；L1（闭源 SaaS）不可选、L2（开源+官方编译分发）只能缓解的结论见总报告第 4 节。
- **非目标（MVP 边界外）**：组织级 SSO/SCIM/LDAP、目录同步（Directory Connector 类）、动态密钥（KMS/PKI/Transit）、团队审计日志流、HSM/KMIP——这些属 Vault/OpenBao/企业版领域，对个人密码管理场景"过重"是分报告D 的明确结论，自研不纳入。
- **竞品反面清单（自研必须超越的已核实缺陷）**：LastPass URL 明文与低迭代遗留账户；1Password 导出无加密选项且 CLI 文档不透明；Bitwarden 个人库无 F8 原语、TOTP 属 Premium；Vaultwarden 无审计背书；Enpass 无 CLI；Infisical 非零知识；KeePass 系无 F8——P0 目标即"同时补齐这些缺口中与本产品定位相关的全部项"。

---

## 2. 功能需求映射表（1Password 功能 → 自研 backlog）

以分报告A 核实的 1Password 能力清单为基准（加密架构白皮书 Release 0.5.2、CLI 2.43.0 命令族、特色专有功能清单）；优先级 P0（MVP 必须）/P1（第二阶段）/P2（远期）。

| # | 1Password 收费功能 | 自研 backlog 条目 | 优先级 | 参照（分报告核实来源） |
|---|---|---|---|---|
| B1 | 保险库条目管理（登录/信用卡/身份/安全笔记/文档/自定义字段） | 条目模型：类型化条目 + 自定义字段 + **全字段加密（URL/标题纳入）**；条目 UUID + 逐字段修改时间 + 删除历史（history） | P0 | 分报告A §1.4；LastPass 反例（URL 明文）；KDBX 合并基础（分报告C §7.3） |
| B2 | AES-256-GCM 客户端加密 + 账户/保险库/条目密钥分层 | 三层密钥体系（账户密钥→库密钥→条目密钥）+ AEAD 加密（§6 选型）；改密/换库无需重加密全部数据（信封加密思想） | P0 | 分报告A §1.4（1Password 分层）；分报告A §2.4（Proton 信封）；分报告B §1.4（HKDF 拉伸 512 位） |
| B3 | Secret Key 双因子（128-bit，本地保存/Emergency Kit） | 独立**密钥文件**（可自定义、可轮换、可多份）+ 主密码双因子派生；**密钥文件不与库同盘分发**（绝不放同一云盘目录） | P0 | 分报告A §1.3/1.4；分报告C §7.3-4（keyfile 分渠道纪律）；分报告E §4.1 前提③ |
| B4 | 全平台客户端 + 浏览器扩展自动填充 | F1：桌面填充引擎 + 浏览器扩展（**native messaging 架构**：浏览器→proxy→本地进程，凭据不落浏览器存储）；Android 系统 Autofill；保存/填充双触发 | P0（桌面扩展）/P1（Android）/P2（iOS） | 分报告C §2（KeePassXC-Browser + keepassxc-proxy 架构，协议文档 keepassxc-protocol.md 公开）；分报告C §10（移动端匹配精度差距） |
| B5 | `op` CLI 命令族（account/item/vault/read/run/inject/signin/document/user/group 等 19 个子命令，官方参考页核实） | F7 CLI：`sp item get/create/edit/delete/list`、`sp read <uri>`（stdout、`--out-file` 0600、`-n` 去尾换行）、`sp inject`（模板渲染）、`sp run`（§3）、`sp import/export`（F9/F10——注意 1Password CLI 自身无 import/export，自研补齐即差异化） | P0 | 分报告A §6（op CLI 命令参考实测清单）；分报告A §1.7（CLI 无 import 子命令核实）；分报告G §1.1 |
| B6 | `op run` 临时环境变量注入 | F8（§3 完整验收规范） | P0 | 分报告A §6.2；分报告D §1 对比表 |
| B7 | 1Password Environments（云端 .env 管理） | 私有部署轨：按环境的变量集管理 + 引用 URI（`sp://<库>/<条目>/<字段>` 风格，支持段省略与 `?attribute=otp` 类查询参数）；云盘轨仅本地 .env 引用；`--environment <ID>` 多环境切换（支持变量替换 `sp://$APP_ENV/...`）；设计为实时拉取而非本地缓存密钥值（对齐 op Environments 行为） | P1 | 分报告A §6.1/6.2/6.6（op:// URI 语法、变量替换、2.42.0-beta 网络请求优化佐证实时拉取） |
| B8 | Shell Plugins（80+ 第三方 CLI 凭据代理，含 AWS/GitHub/Cloudflare/kubectl/Claude Code 等 AI 工具） | 进阶 F8：接管目标 CLI 认证（经本机系统认证授权后以该 CLI 期待方式临时提供凭据，不在磁盘留明文）；支持 bash/zsh/fish；提供自建插件构建指南与测试流程 | P2 | 分报告A §6.5 |
| B9 | Service Account（≤100 个/账户）+ Connect Server | 机器身份：最小授权令牌（按库粒度、可审计、可吊销、有速率限制与 usage report）+ 私有部署 REST secrets 网关（token 存本地受保护位置，文档要求不得入库）；CLI 以环境变量指向令牌 | P1 | 分报告A §6.6；分报告B §3（Access Token/state 设计） |
| B10 | Watchtower 安全巡检 | 弱密码/重复密码/易猜密码报告 + 离线 HIBP K-anonymity 检查（对齐 KeePassXC `analyze` 与 Dashlane 暗网监控的 K-anonymity 仅上传哈希前 3 字节做法） | P2 | 分报告A §1.10、§3.9；分报告C §2（analyze 离线 HIBP） |
| B11 | 通行密钥（passkey）管理 | passkey 作为条目属性存储（对齐 KDBX 做法：作为条目属性可存）；导出/迁移明示支持范围（对齐 Bitwarden "json 才含 passkeys" 显式声明） | P2 | 分报告C §3（KeePassDX Passkey 本地存储）；分报告G §4 速查 |
| B12 | SSH agent 集成 | 解锁后自动加载 SSH 密钥进 agent（支持要求确认/生命周期选项）——1Password 同名功能的开源等价物已有成熟先例 | P1 | 分报告C §2（KeePassXC 内置 SSH Agent） |
| B13 | 桌面应用集成解锁（生物识别授权 CLI） | 桌面 App 生物识别授权 CLI 会话（替代手工 token 粘贴）；失败回退密码提示；参照 Bitwarden 2026.9+ 的 IPC 生物识别方案；注意 Bitwarden CVE-2023-27706（生物识别密钥存本机凭据管理器可被本地攻击者读取）的前车之鉴 | P1 | 分报告B §1.6、§1.7 |
| B14 | TOTP 生成器（条目级） | 条目级 RFC 6238 TOTP（GUI 显示 + CLI `-t` 取值；**免费内置**——Bitwarden 免费档缺失该功能是已核实的隐藏收费点，自研应超越） | P0 | 分报告B §1.3；分报告C §2（KeePassXC TOTP 实现） |
| B15 | 安全共享（保险库共享/组织/家庭） | 非对称密钥分发共享（对齐 Bitwarden RSA-OAEP 模式 / Proton Vault Key 信封 / Dashlane RSA-2048+ObjectKey 论述）；**若实现密钥协商/分享，直接选 ML-KEM 混合方案**（量子威胁结论） | P2 | 分报告B §1.4；分报告A §2.4、§3.4；分报告F §8 |
| B16 | 应用内多来源导入器 | F9（§4） | P0 | 分报告G |
| B17 | 全量导出（加密归档默认） | F10（§5） | P0 | 分报告G |
| B18 | 应急包（Emergency Kit PDF） | 主密码+密钥文件丢失恢复方案：可打印应急包（含账户标识与派生参数，**不含密钥文件本体与主密码**）；密钥文件另路备份指引（U 盘/另一信任设备） | P1 | 分报告A §1.4；分报告C §7.3-4 |
| B19 | 多保险库 | 库/分组隔离，每库独立加密上下文与独立密码可选（对齐 Enpass Primary vault 模型：首库主密码解锁全部库，其余库各有独立密码）；跨库 merge | P1 | 分报告A §5.4；分报告C §2（merge 命令族） |
| B20 | 剪贴板自动清除 | 复制密钥后超时自动清剪贴板（对齐 keepassxc-cli `clip` 行为）；剪贴板是云盘安全模型之外的独立攻击面（分报告E §4.1 前提⑤） | P0 | 分报告C §2；分报告E §4.1 |
| B21 | 条目级附件加密存储 | 附件以加密形式随库存储（对齐 KDBX4：附件移入加密内部头，不再 Base64 内嵌）；GUI 预览 + CLI attachment-import/export | P1 | 分报告C §1/§2；分报告G §2.3 |
| B22 | 密码健康检查（离线 HIBP 之外的本地启发式） | 重复/弱/过期密码与 2FA 缺失提醒（本地计算，不联网上传） | P2 | 分报告A §1.10（Watchtower）；分报告C §2（analyze） |

**P0 边界声明**：B1–B7、B14、B16–B17、B20 + §6 红线全项构成 MVP；B8/B10/B11/B15/B22 为远期，严禁纳入 MVP（范围蔓延风险见 §8.10）。

---

## 3. F8：CLI 临时环境变量注入——参考规范

以 1Password `op run` 语义为规范基准（取自分报告A §6 实测专节、分报告D §1.1 核实语义）。黄金语义一句话：**引用占位符 + 内存解密 + 进程级注入 + 进程结束清理 + 零明文落盘**。

### 3.1 核心语义

- 配置文件（.env）只存**引用 URI**（如 `sp://<库>/<条目>/<字段>`，段可省略，可用名称或 ID），不存密文值 → .env 可安全入版本库。
- `sp run --env-file ./prod.env -- <command>`：外层进程解析全部引用 → 内存解密 → 以环境变量注入**子进程** → 子进程退出，环境随之销毁；父 shell 永不获得值，不落盘。
- 补充命令（对齐 op 命令族）：
  - `sp read <uri>`：单值读取输出 stdout；`--out-file` 写文件默认权限 0600；`-n` 去尾部换行。典型用法（占位示例）：`docker login -u "$(sp read sp://prod/docker/username)" -p "$(sp read sp://prod/docker/password)"`——值经 shell 命令替换进入子进程，同样不落盘。
  - `sp inject`：模板 `{{ sp://库/条目/字段 }}` 替换为真实值，支持拼接（如 `postgres://{{ sp://lcl/db/user }}:{{ sp://lcl/db/pw }}@host/db`）；`-i <模板> -o <产物>`（`--file-mode` 默认 0600）；产物含明文，官方规范为"不再需要时删除已解析的文件"——作为 `run` 的补充而非替代。
- 认证与令牌：交互场景经桌面 App + 生物识别/系统认证（默认推荐路径，无需手工粘贴 token）；无人值守用机器令牌（环境变量指向，如 `export SP_SERVICE_TOKEN="<你的服务令牌>"`）；会话/缓存不持久化明文密钥值（对齐 op CLI 2.x：桌面 App 会话或 signin 获得会话，官方未宣传持久化明文缓存）。

### 3.2 验收标准（10 项）

| # | 项 | 标准 |
|---|---|---|
| 1 | 注入级别 | 进程级（子进程环境变量），语言/框架无关；对齐 infisical run：`HOME/PATH/USER/TERM/SHELL/XDG_*/LC_*` 等系统关键变量**永不注入**（防破坏）；注入值是否覆盖 shell 同名变量需明确定义（infisical 默认覆盖，dotenvx 默认已存在优先、`--overload` 反转——自研择一并在文档明示） |
| 2 | 生命周期 | 真实值仅存活于子进程期间；父 shell 永不获得值；对齐 `op run` 官方表述 "only for the duration of the process" |
| 3 | 清理保证 | 子进程退出即销毁；无明文落盘；不污染父 shell；提供 `--stdin` 变体（参照 `gopass env --stdin`：改走 stdin，规避 `/proc/<pid>/environ` 与 `ps eww` 泄露路径，官方推荐）与 `--file` 变体（ramdisk 临时文件 + `<KEY>_FILE` 路径，12-factor 风格，参照 gopass） |
| 4 | Windows（Git Bash）兼容 | 一等公民：原生 Windows 二进制（参照 infisical 的 winget/Scoop/npm 安装路径与 PowerShell/cmd/Git Bash 示例）；Git Bash 下 `sp run -- printenv <KEY>` 可用；**禁止**依赖 sops exec-env 类"未获官方背书"路径（分报告D §1.5 已核实其在 Windows 易踩坑、FIFO 不可用）；`--exec`（进程替换）类语义在 Windows 不可用需规避（gopass `--exec` 先例） |
| 5 | 输出遮蔽 | 默认对子进程 stdout/stderr 中出现的密钥值做遮蔽替换；提供 `--no-masking` 显式关闭（调试用）——对齐 `op run` |
| 6 | .env 解析 | 对齐 op 官方规则全集：UTF-8；`#` 注释；引号剥离/保留规则；`$VAR`/`${VAR}` 展开；`\$` 转义；文件内变量互引；多行值引号包裹；空值→空字符串 |
| 7 | Shell 展开陷阱 | 文档明示：`sp run -- echo "$VAR"` 会被外层 shell 先展开成 URI；正确用法 `sp run -- sh -c 'echo "$VAR"'` 让替换发生在子 shell 内（对齐 1Password 文档警告原文） |
| 8 | 威胁边界文档化 | 明文文档：同操作系统用户运行的进程可以互相读取环境变量（`op run` 官方原文警告 "you should assume that the processes on your computer can access the environment of other processes"）——注入式方案的固有边界，不得夸大安全承诺 |
| 9 | POSIX 变量名 | 非 POSIX 兼容的密钥名处理参照 `bws run --uuids-as-keynames`（UUID 连字符转下划线前置下划线，规避 POSIX 变量名限制；非 POSIX 兼容的 secret 名仍会设置但仅部分程序可见——该细节 bws 文档明示） |
| 10 | 机器身份最小授权 | 无人值守场景用机器令牌：按库粒度最小授权、可审计、可吊销（对齐 Service Account/Access Token 模型）；token 存本地受保护位置不入库；被撤销令牌的会话失效语义参照 bws state（加密存储、可 `state_opt_out`；被撤销令牌在会话过期前可能仍有效——自研应做到撤销即失效） |

### 3.3 参考实现（开源，经分报告核实）

| 实现 | 许可证/版本 | 参考点 | 来源 |
|---|---|---|---|
| Proton Pass pass-cli | GPL-3.0（v2.5.1，2026-10-21，Rust，426 stars） | `pass://vault/item/field` 引用模型 + 注入环境变量/模板文件 + 官方 GitHub Action（load-secret-action）与 Ansible 插件；证明该模式可以开源实现复刻 | 分报告A §2 |
| Infisical `infisical run` | MIT（CLI 0.43.145+） | 系统关键变量保护名单、`--path`/`--recursive`/`--watch`/`--print`、Windows winget/Scoop/npm 一等公民 | 分报告D §1.2 |
| 1Password `op run` | 闭源（语义规范，CLI 2.43.0） | 全部语义与 .env 解析规则（本节基准）；注意 CLI 无持久化明文缓存（Environments 实时拉取：2.42.0-beta 将网络请求从 6 次优化到 4 次佐证） | 分报告A §6 |
| dotenvx `run` | BSD-3-Clause（v1.x） | ".env 密文化（secp256k1 ECIES 封装 AES-256 临时密钥，`encrypted:...` 值可提交 git）+ 私钥进 OS 密钥库（含 **Windows Credential Manager**）+ 进程级注入"的极简无服务器形态证明；CI 一行接入（`DOTENV_PRIVATE_KEY_CI`） | 分报告D §1.6 |
| BSM `bws run` | 开源（Rust，sdk-sm） | state 加密令牌缓存、`--no-inherit-env`、`--uuids-as-keynames` POSIX 细节——分报告B 结论：本生态内唯一与 `op run` 对标的原生实现，自研最值得抄的机制设计 | 分报告B §3.3 |
| gopass `env` | MIT（v1.17.4） | `--stdin`/`--file`（ramdisk）/`--exec`（Windows 不可用）/`--keep-case` 变体语义 | 分报告C §6 |
| Vault/OpenBao Agent `env_template` | BUSL/MPL（OpenBao 标注 public beta） | 反面参照：进程监督注入可行但配置门槛高（`exec { command = [...] }` 配置文件驱动）、无内联命令语法（OpenBao issue #4110 请求 `-- "$@"` 语法未实现）——自研应提供一键 `run` 而非 Agent 配置文件；另注意 Vault 侧 cache-key collision 等 2026 漏洞集中在其策略解析 | 分报告D §1.3/1.4 |
| sops `exec-env` | MPL-2.0（v3.13.3） | 反面参照：语义正确（进程级、不落盘）但 Windows 未获官方背书（issue #628/#784/#1127 平台相关 bug；FIFO 不可用环境官方点名 Windows）——自研 Windows 路径必须自研实现而非包装 | 分报告D §1.5 |
| direnv | MIT（v2.38.2） | 反面参照：shell 级注入临时性最弱（变量活在 shell 会话，unload 依赖 cd 触发、shell 崩溃不清理；任何同 shell 程序可读），Windows 无官方支持——不符合 F8 要求；其 "environment diff + 自动 unload" 机制仅 Unix 工作流可借鉴 | 分报告D §1.6 |
| Vaultwarden/BW `bw`（当前版） | 开源 | 反面参照：社区惯用 `$(bw get ...)` 命令替换——展开结果进入外层 shell 命令行/子进程参数，可被 `ps -ef`、shell history、CI 日志捕获；秘密在 shell 变量存留无自动清除；`bw serve` 把明文暴露在本地端口——自研 F8 就是要消灭这些泄露面 | 分报告B §1.6 |

### 3.4 典型用法示例（占位符，不入真实凭据）

```bash
# .env 文件（只存引用 URI，可安全提交版本库）
# DB_PASSWORD=sp://development/MySQL/credentials/password
# API_TOKEN=sp://$APP_ENV/GitHub/credentials/personal_token

# 进程级注入：真实密钥只在子进程存活期间存在
sp run --env-file ./prod.env -- <你的命令>

# 输出遮蔽默认开启；调试时显式关闭
sp run --no-masking --env-file ./prod.env -- <你的命令>

# 变量在子 shell 内展开（避免外层 shell 先展开成 URI）
sp run -- sh -c 'echo "$DB_PASSWORD"'

# 单值读取（stdout，不落盘）
sp read sp://app-prod/db/password
sp read --out-file <路径> sp://app-prod/db/password   # 文件权限 0600

# 模板渲染（产物 0600，用后即删）
sp inject -i config.yml.tpl -o config.yml

# 无人值守（CI）：机器令牌走环境变量
export SP_SERVICE_TOKEN="<你的服务令牌>"
sp read sp://dev/mysql/password
```

与 `op run` 行为对齐的免责边界：以上机制无法防御"同操作系统用户的其他进程读取子进程环境变量"（op run 官方原文警告，见验收标准 #8）——该边界必须写进用户文档而非仅在开发内部知晓。

---

## 4. F9：批量导入器设计参考（取自分报告G §3/§5.2）

### 4.1 输入源清单（按优先级）

1. **密码库格式**：`1pux`（1Password Unencrypted Export，注意桌面导出不含 passkey）、Bitwarden CSV/JSON（json 系含 cards/identities/passkeys/SSH keys）、`keepass2xml`、KDBX（只读解析）、通用 CSV（列映射向导，对齐 KeePassXC 10 列 `Group,Title,Username,Password,URL,Notes,TOTP,Icon,Last Modified,Created` 与 Bitwarden CSV 表头）；
2. **散落密钥文件**：`.env*`（dotenv 语法）、`id_rsa`/`*.pem`/`*.ppk`（识别私钥指纹去重）、任意 config 文件（规则引擎驱动）；
3. **脚本/编排来源**：shell rc（`export K=V` 行）、systemd `EnvironmentFile=`、docker-compose `environment:`/`env_file:`、GitHub Actions/GitLab CI 变量段；
4. **当前环境变量快照**（进程 env 枚举——**gitleaks/trufflehog 均不内置，已核实**：trufflehog `pkg/sources` 逐项核实为 git/filesystem/docker/s3/gcs/github/gitlab/jenkins/circleci/postman/syslog/stdin/huggingface/elasticsearch 等，无 env 源；gitleaks 仅 git/dir/stdin 三模式）：属自研增量能力，注意快照临时文件残留治理（§5 红线 4）。

### 4.2 发现引擎

- **gitleaks 规则块（主引擎）**：gitleaks v8.30.1（MIT，30.3k★；git/dir/stdin 三模式、报告 json/csv/junit/sarif/template 五种、`--baseline-path` 存量降噪、归档递归 `--max-archive-depth`；config/gitleaks.toml 内 235 个规则/allowlist 块，2026-10-25 计数核实）。**已核实其 README 顶部警告 feature complete、作者转向 betterleaks——已进维护期**：规则 TOML 可静态抽取内嵌，输出统一 finding 模型 `{source, file, line, ruleId, secret, fingerprint}` 再走导入管道（JSON finding 含 File/Line/RuleID/Secret/Fingerprint，可直接脚本化转换：name 用 RuleID+文件路径，value 用 Secret）。
- **trufflehog（可选插件）**：v3.99.7（**AGPL-3.0**，28.7k★），911 检测器（`pkg/detectors` 目录核实）、对 20+ 常见凭据类型活体验证（verified results，可顺手剔除失效密钥）；因 AGPL 许可与体积不宜嵌入式依赖，作为后续可选插件；**已核实不支持环境变量源**；覆盖 env 需自行导出快照到临时文件再 filesystem 扫描（注意临时文件残留）。
- **detect-secrets 基线思想**：Apache-2.0（4.7k★，最后推送 2026-04-02）；`detect-secrets scan > .secrets.baseline` 建立存量基线、后续只报新增；插件化 `--list-all-plugins/--disable-plugin`；`--all-files` 扫非 git 跟踪文件。借鉴其基线模式——首次导入做全量快照，之后增量导入；其定位是 CI 防泄漏而非导入器，仅借鉴思想。

### 4.3 去重与分类（三级策略，对齐分报告G §5.2）

1. **一级**：密钥值哈希精确匹配；
2. **二级**：URL+用户名条目合并；
3. **三级**：指纹/前缀匹配识别同一密钥的多个副本（如私钥指纹、key 前缀）。
冲突处理原则：**冲突不清除**——保留全部副本并打 `duplicate/conflict` 标记人工裁决；分类按来源路径聚类（`来源类型/项目路径/规则名`）。

### 4.4 市场空白声明与竞品定位

- 已核实市场上不存在"扫描→识别→去重→分类→批量导入"的现成 CLI：Infisical 无 import 命令（`secrets set --file` 一次一个文件、无目录扫描、无跨文件去重；其设计动机之一是避免值进入 shell 历史）；gitleaks/trufflehog 把 .env 内容当"泄漏"输出报告，不做结构化键值提取入库；recursenv（Rust，向上遍历合并 .env）与 envmerge（多 .env 合并保注释）只做目录树合并小工具。
- Bitwarden 60+ 导入格式与 `bw import --formats` 机器可读列表是中介格式层的最佳参照；pass-import（GPL-3.0，915★，最后提交 2026-06，40+ 来源含 1Password/Bitwarden/KeePass 家族/LastPass，目标端含 gopass）证明"多来源导入器"在密码库领域是成熟模式。
- 平台侧参照：Infisical secret syncs（40+ 目标双向同步）与 external migrations 证明"平台对平台同步"是另一条路，但对象模型（环境变量）与凭据条目库不同构，仅作开发者 secrets 子域参考。
- 结论（分报告G §0 原话）："F9 的'扫描→识别→去重→分类→批量导入'全链路在市场上是空白，属自研正当价值点。"

### 4.5 F9 验收标准

| # | 项 | 标准 |
|---|---|---|
| 1 | 来源覆盖 | §4.1 四类输入源各有解析器；至少 1pux / Bitwarden CSV+JSON / keepass2xml / 通用 CSV / .env / 环境变量快照六项 P0 |
| 2 | 发现引擎 | 内嵌 gitleaks 规则 TOML（静态抽取，235 规则基线），输出统一 finding 模型 `{source, file, line, ruleId, secret, fingerprint}` |
| 3 | 去重 | 三级策略（值哈希 / URL+用户名 / 指纹前缀）逐级执行；冲突不清除，全部保留并打 `duplicate/conflict` 标记 |
| 4 | 分类 | 按 `来源类型/项目路径/规则名` 聚类入对应分组 |
| 5 | 基线增量 | 首次全量快照（detect-secrets 基线思想），后续导入只处理新增 |
| 6 | 环境变量快照 | 进程 env 枚举自研实现（gitleaks/trufflehog 均无此源，已核实）；快照临时文件即用即清（§5 红线 4） |
| 7 | 人工抽查提示 | 凡官方文档未逐字段声明的映射（TOTP/自定义字段），导入完成后输出"需人工抽查"清单（对齐分报告G 诚实声明原则） |

---

## 5. F10：导出安全设计（取自分报告G §5.2 五条红线）

1. **默认输出加密归档**：口令 + 主密钥双选项（对齐 Bitwarden password-protected / account-restricted 两档语义——后者换加密密钥后旧文件不可解，需明示差异）；"单一加密全量归档（含附件）"一步达成——行业空白：1Password 1PUX/CSV 官方明文（官方原句 "Exported data files are not encrypted. They are stored in plaintext."）、Bitwarden zip 与 encrypted_json 分离无一步命令、Dashlane/Enpass/LastPass 亦明文。
2. **明文导出防护**：显式 flag（如 `--plaintext`）+ 交互重输主密码 + 明确警告文案 + "用后即删"提示（对齐 1P/BW 官方措辞："delete the file immediately after use"；1P："Delete them when you are done with them"；并纳入官方建议：不邮件传输、不存网盘）。
3. **stdout 流式导出**一等支持（对齐 `bw export --raw` 与 `keepassxc-cli export` 原生 stdout），避免明文临时文件落盘。
4. **残留治理**：重点是"尽量不落盘"而不是"可靠擦除"——引用 gopass ADR A-08 结论：现代存储（SSD/日志文件系统）上 shred 不可靠。
5. **字段损失明示**：导出清单必须声明哪些类型/字段不在该格式中（对齐 Bitwarden "json 才含 passkeys"、trash 与 Sends 任何格式不导出、1P CSV 仅 Login/Password 且无 passkey 的显式声明做法）；附件/TOTP/自定义属性/history 的包含范围在每次导出输出摘要；旧版 passkey 数据"不应作为长期备份"的提示语义一并继承。

**格式建议**：

- 优先 **KDBX 兼容**：自有数据落 KDBX4（AES-256/ChaCha20 + Argon2id + HMAC-SHA-256 头部认证 + 分块 Encrypt-then-MAC，规范公开 https://keepass.info/help/kb/kdbx_4.html ，2026-10-25 核实），直接接入 KeePassXC/KeePassDX/KeePassium/Strongbox 现成客户端群与条目级合并生态——用最低成本换全平台客户端复用；分报告G 速查表显示 KDBX 是唯一"附件/TOTP/passkey/自定义属性"全保留的格式；
- 或**自有格式 + 1PUX/Bitwarden JSON 双向互导**（保迁移出口）；
- 无论哪种：全量导出必含附件/TOTP/自定义属性/history；加密归档为默认；明文通道始终保留（对齐 Bitwarden "明文 JSON 导出可用于解密审计，等于可选明文解密通道"的设计语义）。

---

## 6. 威胁模型与加密选型（取自分报告F 十条设计红线）

**威胁模型**：(1) 软件厂商后续故意针对国内用户插入恶意代码窃取主密码；(2) 某个发布版本开始被投毒。核心事实支撑：npm/PyPI 渠道最现实（CVE-2026-42994 实证 1.5 小时投毒窗口、Shai-Hulud 蠕虫两波）、搜索广告假冒分发（KeePass 2023/2025 事件）、闭源不可检测（L1）；L3（开源+可复现构建）是唯一可验证解。

| # | 红线 | 要求 | 事实依据（分报告F） |
|---|---|---|---|
| 1 | 版本签名与校验 | 发布物（安装包/二进制）带可离线验证签名（Minisign/ed25519 或 sigstore），公钥预置多个独立位置（文档、源码仓库、公证渠道）；校验失败拒绝运行 | KeeLoader 假安装器持有效 CA 签名证书——仅"有签名"不够，须公钥多位置预置 |
| 2 | 可复现构建（第一优先级目标） | 锁定工具链版本、剔除时间戳/路径等非确定源、发布时公开复现指引（参照 KeePassXC #4411 做法）；每次发布至少邀请第三方复现并公示比对；做不到逐字节前至少提供"源码→构建→哈希"公开验证流程 | KeePassXC 全行业唯一走通；Bitwarden 无计划（官方文档反向核实无 reproducible 条目）；跨平台逐字节复现仍有未决难题（openSUSE 状态页） |
| 3 | 分发渠道纪律 | 官方渠道之外（尤其 npm/PyPI 等包管理器、搜索广告投放）一律不作为获取途径；不做可能被劫持为投放点的竞价广告 | CVE-2026-42994（npm 1.5 小时投毒，官方下载渠道不受影响）；KeePass Punycode 假冒站（Google 广告"已验证"绿标）；CVE-2018-25081 编号年份误解说明预置年份与披露年份差异需在公告中写清 |
| 4 | 无遥测 | 主程序零网络上报（崩溃报告须显式开关且内容可审）；同步流量目标为用户提供的中立存储；**元数据（URL/标题）一律纳入加密** | LastPass 教训（URL 明文泄露上网画像，官方公告原文承认 "unencrypted data, such as website URLs"） |
| 5 | 主密码永不出本地进程（架构红线） | KDF 与加解密全部本地完成；不落盘明文；不进入任何 CI/云进程（凡进过构建环境的密钥默认视为泄露）；对内存中的主密码与派生密钥做主动清理并缓解内存残留（不同 API 路径 + 随机串污染） | Shai-Hulud（CI 扫描翻找凭据并回写公开仓库）；CVE-2023-32784（.NET 明文串残留可从 pagefile.sys/hiberfil.sys/整机 RAM 恢复，首位字符除外） |
| 6 | KDF/加密选型 | **Argon2id**（内存 ≥64MiB 起步、随硬件上调；对标 KeePassXC 官方建议 1 秒解密时间滑杆）或 PBKDF2 ≥600,000（仅兼容场景）；**AES-256-GCM 或 XChaCha20-Poly1305**（AEAD 强制，杜绝无完整性保护模式）；**全员强制高参数、无遗留降级路径** | LastPass 低至 1 次迭代的遗留账户教训；AES-256-CBC 无 MAC 的结构缺陷（研究者分析）；Bitwarden Argon2id 32MiB/6/4 起步、上限 1024MiB/1024/16 参数参照；KDBX4 的 HMAC 头部认证 + 分块 Encrypt-then-MAC 是格式层范本 |
| 7 | 双因子 | **密钥文件（可自定义）+ 主密码**；密钥文件不与库同盘分发、走独立渠道（U 盘/另一信任设备）；主密码人类可记（低熵）时 KDF 穷举是真实短板（量子模型下 Argon2id 内存困难性仍有效） | 全行业仅 1Password Secret Key 近似实现且不可自定义；分报告C §7.3-4/F4 纪律；分报告F §8（主密码熵是量子威胁的真实短板） |
| 8 | 零知识 | 云端/服务器只见密文（自建服务器轨也须满足：服务器仅存密文与二次加盐哈希、无明文还原能力）；拒绝 Infisical 式服务端持钥模型 | Bitwarden 白皮书 + ETH Zurich"假设服务器完全恶意"审计背书该模型；Infisical ENCRYPTION_KEY 泄露=全部密钥泄露（分报告D §3） |
| 9 | 量子余量 | 对称层 AES-256/XChaCha20 + Argon2id 已达量子余量上限（AES-256→~128 位量子安全，Grover 实际代价远超理论上限——需海量纠错量子比特与串行查询）；未来分享/密钥协商直接选 ML-KEM 混合方案；age v1.3.2 已支持后量子混合密钥（mlkem768x25519）可参考 | NIST PQC FAQ（"AES-192 与 AES-256 将在很长时间内保持安全"）+ PQC 会议论文；Proton Pass 截至 2026-10-25 未上线 PQC（生态滞后即自研机会） |
| 10 | 供应链卫生 + 事件披露纪律 | 依赖锁定+定期审计（对照 Shai-Hulud 传递投毒）、CI 最小权限、签名密钥离线保管、构建环境与密钥物理隔离；发现投毒/漏洞时**一次性说清泄露范围与加密实现缺陷**，不用"无证据"话术拖延 | Shai-Hulud 攻击链（窃维护者 npm 令牌→发毒版→受害者 CI 用 TruffleHog 类扫描翻云凭据→回写公开仓库→继续传播）；LastPass 公告翻车反例（2022-12 "无证据"被 2023-03 推翻） |

---

## 7. 双轨架构建议

### 7.1 自建服务器轨

参照对象与资源/暴露面基线（均取自分报告B/D 核实数据）：

| 参照 | 资源基线 | 可借鉴要点 | 不采纳点 |
|---|---|---|---|
| Bitwarden 官方 Lite | 最低 **200MB RAM**/1GB 盘，支持 ARM（树莓派/NAS），单镜像 `ghcr.io/bitwarden/lite`（2025-12 从 Beta 毕业） | 个人用途定位；数据库自备（SQL Server/PostgreSQL/SQLite/MySQL）；单镜像部署极简；Helm/K8s/离线部署路线并存 | 安装需联网领安装密钥（自研应免此环节）；MSSQL 标准版偏重 |
| Bitwarden 标准版 | 最低 x64 双核 2GHz / 2GB RAM / 12GB 盘（推荐 4GB/25GB），Docker Engine 26+，开放 80/443 | 完整功能、企业就绪；备份三件套（database+attachments+keys） | 资源重；安装 ID/密钥环节；MSSQL Express 10GB 上限 |
| Vaultwarden | 空闲 10–30MB，典型 400–500MB，单 SQLite（源码编译需约 7GB RAM——低配用预构建镜像） | 极简运维、免费全功能、单文件备份 | **服务端授权逻辑是重实现高发缺陷区**（2024–2026 授权类 CVE 密集：2FA 绕过/跨组织越权/admin RCE/SSO email_verified 缺陷）——自研服务端必须以此为反面镜，授权代码须经审计；不对数据丢失负责的免责框架不可学 |
| Infisical | CPU 2/4 核、内存 4/8GB、盘 20/50GB（SSD），PG（pg_data 唯一真相源）+Redis+backend 三容器 | run/importer UX、环境/项目对象模型 | **服务端持钥模型不采纳**（与零知识红线 §6-8 冲突）；ENCRYPTION_KEY 单点风险 |

服务端设计要求（自部署安全注意事项，取自分报告B §1.7）：

- **零知识**：服务器仅存密文与二次加盐哈希，无明文还原能力（对齐 Bitwarden 白皮书 "What the server stores" 语义）；
- **HTTPS 强制**（Web 端依赖 secure context 的 Web Crypto）；反向代理收口、仅暴露 80/443；
- 数据库/缓存端口（PG 5432、Redis 6379 等）不暴露公网（Infisical 自托管教训）；SMTP 配置正确否则重置邮件外泄；
- KDF 参数应调高——服务器主机即"所有密文保险库"的持有者，**主机沦陷=离线爆破主密码**；
- 备份策略内建：数据库+附件+密钥（标准版语义）/单数据卷（Lite 语义）；备份为产品内建能力而非用户自觉（Vaultwarden 免责声明反例：维护者不对数据丢失负责）；
- **客户端优先本地代码**（扩展/桌面）而非服务端推送网页——自部署 Web 端页面由服务端推送、服务端投毒可直接投 JS 是结构性残余风险（分报告B §2.5）；
- 认证与会话：session/令牌只存环境变量或加密 state，不落明文（对齐 `BW_SESSION` 与 bws state 设计）；管理端口（admin 类页面）默认锁定；
- 功能取舍明示：参照 Vaultwarden Wiki 的"缺失功能清单"做法——自建轨哪些企业功能（SSO/SCIM/策略）不支持要显式声明，避免用户误判覆盖面。

### 7.2 云盘同步轨（无服务器）

数据形态 = 单个加密数据库文件（优先 KDBX 兼容或自有单文件格式），参照 Enpass/KeePass 生态（分报告C/E）：

- **管道选项**（均经分报告E 核实）：
  1. **坚果云 WebDAV（首选）**：正式功能、应用密码机制（网页端：账户信息→安全选项→第三方应用管理；主密码与同步凭据分离、可按应用单独吊销）、免费每月 1GB 上/3GB 下（单库月消耗 MB 级绰绰有余）、限速 600（免费）/1500（付费）次每 30 分钟不触限、单文件上限 500MB；专业版约 199.8 元/年（以购买页实时为准）；
  2. **rclone crypt（进阶）**：文件名+内容双加密（XSalsa20+Poly1305 64KiB 分块 / EME AES-256 文件名），坚果云按 `vendor = other` 接入；rclone.conf 口令仅轻度混淆须妥善保管（可配配置文件密码）；Windows `rclone mount`（需 WinFsp）挂盘符或定时 sync + `--backup-dir` 归档被覆盖文件；
  3. **Syncthing（零第三方云账号）**：P2P、无中央服务器（"数据从不存储在计算机之外"）、TLS；Android 用社区分支 Syncthing-Fork（官方应用 2024-12 已归档，分支 2026-10-22 仍活跃）；需一台常开节点；**避免并发保存（损坏风险，KeePassDX Wiki 警告）**。
- **排除项**：阿里云盘/百度网盘（官方无 WebDAV；第三方桥接有永久封号案例与违规凭据注入风险）；Google Drive/Dropbox（国内长期阻断）；OneDrive（国内不稳定；Android 文件提供器有 URI 重分配与 >512KiB 文件损坏记录——KeePassDX Wiki 官方警示）。
- **客户端实现要求**：
  - 实现**条目级合并**（UUID+逐字段时间戳新者胜、旧值入 history，对齐 KDBX 合并机制），杜绝"最后写入者覆盖整文件"语义——这是云盘轨与"单文件覆盖"型工具的本质区别；
  - 保存前先拉取最新（降低覆盖窗口）；冲突副本（conflicted copy）检测与一键 merge；`--dry-run` 预览差异（对齐 keepassxc-cli merge）；
  - Android 端禁止直连 Google Drive/OneDrive 类文件提供器；WebDAV 直连或 SAF 桥接（RCX/RSAF 先例）；
  - 各端自动备份内建（对齐 KeePassium 应用内备份/KeePassXC 备份选项/KeePassDX 备份到外部存储）。
- **纪律内建为产品能力**：密钥文件与库文件分渠道分发的强提示；中性文件名建议（不叫 passwords.kdbx）；云盘账号 2FA 指引；3-2-1 备份与季度恢复演练提醒；"conflicted copy"出现时引导 merge 而非覆盖。
- **CLI 与注入在云盘轨的落点**：CLI 直接打开本地库文件（无需服务器），`sp run`/`sp read` 照常工作——这是云盘轨相对 Bitwarden 系（必须连服务器）与 Enpass（无 CLI）的关键体验优势，也是双形态统一（同一加密内核）的意义所在。

### 7.3 双轨选择建议（产品内呈现）

| 用户约束 | 建议轨 | 依据 |
|---|---|---|
| 无常开服务器 / 不想运维 | 云盘同步轨 | 成本≈0、零公网暴露（分报告E 量化结论）；自建服务器对"只同步一个几百 KB 文件"是过度工程 |
| 已有 VPS/NAS 且需多端实时共享、网页端访问 | 自建服务器轨 | Bitwarden Lite 200MB RAM 可跑树莓派/NAS（分报告B）；服务器轨换来 Watchtower 类服务端能力与实时推送 |
| 安全要求极高、文件名都不能给云盘看 | 云盘轨 + rclone crypt | 文件名+内容双加密（分报告E §2.7 机制核实） |
| 零第三方云账号 | 云盘轨 + Syncthing（需常开节点） | 无云账号/月费/暴露面；iOS 覆盖弱是代价 |
| 完全离线 | 云盘轨（仅 Folder Sync/Wi-Fi 同步形态） | Enpass 三种同步方式中的离线形态先例（分报告A §5.4） |

---

## 8. 风险与开放问题清单

1. **F1 自动填充引擎工作量（最大风险）**：1Password 填充体验是其最深的护城河；KeePass 系移动端依赖系统 AutoFill 框架，字段匹配精度弱于 1Password 专有引擎（分报告C §10 已核实），Strongbox/KeePassium 的高级匹配甚至属付费项。浏览器扩展 + native messaging 可参照 KeePassXC-Browser（协议文档公开），但全平台填充是本项目最大工程量。**开放问题**：MVP 是否先做 CLI+桌面+加密内核、浏览器扩展（P0）先行、Android/iOS 填充后置。
2. **跨平台客户端矩阵**：Win/Linux/Android 必选 + macOS/iOS 备选意味着至少 3–5 个客户端实现。若选 KDBX 兼容格式，桌面/移动客户端群可直接复用（大幅降低成本）——但 KDBX 格式演进受制于 KeePass 生态规范（KeePass 原版为格式定义者，SourceForge 分发）。与 §8.6 联动决策。
3. **加密实现正确性责任**：自研加密内核须以成熟原语（age/Argon2/AEAD 标准库）组装而非自造算法，并按红线 #2 邀请第三方审计；**内存清理在带 GC 语言（.NET/Go/JVM）上难做彻底**（CVE-2023-32784 根因即 .NET 字符串残留，2.54 才修复）——语言选型（Rust/Go/C++）在架构评审时定，直接影响红线 #5 可达性。age 无公开商业审计报告（仅有学术形式化分析）也是选型时须知的事实。
4. **可复现构建的前期投入**：工具链锁定与跨平台逐字节复现有未决难题（KeePassXC 的 openSUSE 状态页记录部分非确定性来源，分报告F §7）；做不到逐字节前至少提供"源码→构建→哈希"公开验证流程（红线 #2 降级路径）。
5. **待补核实项（继承自分报告，未取得）**：Dashlane 个人档官方价格（官网动态渲染）；LastPass 2021-09 与 2023-02/03 公告原文（被 Akamai 403）；Strongbox/KeePassium 当前精确版本号；坚果云 WebDAV 具体服务器域名（帮助页 2064 未列出，需查其帮助中心词条）；KeePassium 免费版 WebDAV 可用性（定价表将"企业存储同步"列 Premium，需装机确认）；`op import` 可用格式权威清单（官方文档已不收录，需 `op import --help` 实测）；1PUX 是否含附件仅有第三方佐证（官方文档未明示）。若路线 B 涉及 1PUX 互导，相关项需补充实测。
6. **KDBX 兼容 vs 自有格式的取舍（重大架构决策，须单独走决策记录流程）**：KDBX 兼容换客户端群复用 + 条目级合并生态 + 迁移出口，但绑定其规范与 Argon2 参数生态；自有格式自由度最高但需自建全平台客户端与迁移出口（1PUX/Bitwarden JSON 互导）。分报告G 速查表显示 KDBX 是唯一"附件/TOTP/passkey/自定义属性"全保留的格式。
7. **同步冲突的用户纪律依赖**：云盘轨的并发写安全依赖"全端使用支持合并的客户端"，技术上无法完全阻止用户用整文件覆盖型工具直接替换库文件（分报告C §7.3 风险场景 1）——需在产品内做强提示与冲突副本检测；KeePass 原版 trigger 方案（保存事件→Synchronize URL→自禁/自启触发器防死循环，官方示例可直接导入 XML）可作自动化参照。
8. **PQC 路线**：对称层/KDF 无量子焦虑（NIST FAQ）；仅当实现分享/密钥协商（B15）时引入 ML-KEM 混合——避免过早引入年轻实现；age v1.3.2 的后量子混合密钥（mlkem768x25519）是现成参照。
9. **维护者与运维风险**：参照 Vaultwarden 教训（单一维护者+社区模式的服务端授权质量短板，分报告F §6.3），自研服务端必须避免单点维护、服务端授权逻辑（组织/集合/权限边界）重点审计，并将"及时升级+备份"作为产品内建能力而非用户自觉；自托管者补丁滞后即风险窗口。
10. **范围蔓延风险**：Shell Plugins（B8）、Environments 云端化（B7）、组织共享（B15）、Watchtower（B10）、密码健康检查（B22）均为 1Password 生态深度功能；MVP 严禁纳入。P0 边界 = B1–B7/B14/B16–B17/B20 + §6 红线全项 + §7.2 云盘轨最小实现。
11. **双轨同步语义的工程复杂度**：同一数据模型同时服务"服务器 API 轨"与"单文件云盘轨"，需在数据层设计时就确定统一的变更日志/合并原语（KDBX history 语义是现成参照）；这是双形态承诺的最大内部工程风险。
12. **国内合规与分发**：调研核实全部国外产品均无国内主体（1Password 仅国际卡支付、无国内版；Enpass 买断制支付门槛最低但亦无国内主体）；自研若面向国内分发，更新服务器、签名公钥公证渠道（红线 #1）的国内可达性需在发布方案中设计——此点分报告未覆盖，属开放问题。
13. **迁移枢纽兼容性承诺**：分报告G 结论是"Bitwarden 为第一迁移枢纽，KeePassXC 为附件保全落点"——自研若想承接迁移流量，至少须支持 Bitwarden CSV/JSON（含 encrypted_json 口令保护导入）与 1PUX 输入，否则用户从 1Password/Bitwarden 迁入无低成本路径。

---

## 9. 分阶段里程碑建议（由 P0/P1/P2 推导，供排期讨论）

| 阶段 | 范围 | 出口标准（对应本文验收项） |
|---|---|---|
| M1 加密内核 + CLI（云盘轨最小版） | B1/B2/B3/B20 + §6 红线全项 + KDBX 兼容或自有格式定稿（§8.6 决策） | 库文件本地加解密通过；密钥文件+主密码双因子可用；`sp read` 可取值；"源码→构建→哈希"公开验证流程可用 |
| M2 F8 注入 | §3 全部 10 项验收标准 | Git Bash 下 `sp run -- printenv <KEY>` 可用；输出遮蔽默认开启；.env 引用化可提交版本库 |
| M3 F9/F10 | §4.5 + §5 五条红线 | 六类输入源导入 + 三级去重；默认加密归档含附件；明文导出三重防护 + stdout 流式 |
| M4 桌面客户端 + 浏览器扩展（F1） | B4 桌面部分 | native messaging 填充可用（参照 KeePassXC-Browser 协议）；自动保存 |
| M5 云盘轨完整化 | §7.2 全部（合并机制、冲突检测、自动备份、纪律提示） | 条目级合并通过并发写测试；坚果云 WebDAV 管道端到端可用 |
| M6 自建服务器轨 | §7.1 | 零知识服务端（仅存密文+加盐哈希）；HTTPS + 端口收口；备份内建 |
| M7 移动端与进阶 | B4 Android / B12/B13/B14 扩展 / P1 批次 | Android 系统 Autofill；TOTP；SSH agent |
| M8 可复现构建 + 第三方审计 | 红线 #1/#2 完整达成 | 逐字节复现比对公示；至少一次第三方审计 |

说明：里程碑仅为 P0/P1/P2 的时序化呈现，不含工期估计（分报告无工期数据，不臆造）；每阶段出口必须复验 §6 十条红线不回退。

---

## 附：本基线引用的分报告核实要点索引（均 2026-10-25 核实）

| 主题 | 分报告位置 |
|---|---|
| `op run`/`op inject`/`op read`/`op plugin run` 语义与 .env 解析规则 | 分报告A §6.1–6.8 |
| 1Password 收费功能清单（Secret Key/Environments/Shell Plugins/Service Account/Connect/Watchtower/SSH agent） | 分报告A §1.3–1.10 |
| Bitwarden 白皮书加密架构、Argon2id 参数、Lite 200MB 自部署、`bw run` 已移除、`bws run` 机制 | 分报告B §1.3–1.6、§3.3 |
| Vaultwarden 资源数据、免责声明、缺失功能清单、CVE 群 | 分报告B §2.2–2.5；分报告F §6 |
| KDBX4 格式、Argon2id、条目合并机制、keyfile 纪律、客户端群云同步矩阵 | 分报告C §1–§7、§10 |
| `gopass env` 变体语义、pass-import、注入机制对比表（infisical/dotenvx/sops/direnv/Vault/OpenBao） | 分报告C §6；分报告D §1 |
| 坚果云 WebDAV 配额与限速、rclone crypt 机制、Syncthing Android 归档真相、云盘排除项、成本对比 | 分报告E §2–§6 |
| LastPass 事件链、CVE-2023-32784、CVE-2026-42994、假冒分发、可复现构建、量子结论、三级信任模型、十条红线 | 分报告F §1–§10 |
| 导入格式清单、导出明文声明、迁移矩阵、字段保留速查、F9 工具链、F10 红线 | 分报告G §1–§5 |
