# 分报告 D：CLI 与机密管理类工具调研

> 调研子代理 T4 ｜ 调研日期：2026-10-25
> 基准：1Password（`op run` 语义：密钥临时注入环境变量 → 进程结束即失效 → 不落盘）
> 服务对象：SuperPasswdKey 选型/自研（重点关注 F8 临时注入、F6 私有部署；用户主力环境为 Windows + Git Bash）

---

## 0. 调研对象与三件套（URL 实测核实）

所有 URL 除特别注明外均为 **核实: 2026-10-25 可达**（WebFetch 实测）。

| 对象 | 定位 | 官网 | 文档 | 源码 |
|---|---|---|---|---|
| Infisical | 开源密钥管理平台（MIT 核心 + 企业版目录 ee 另授权），服务端集中管理 + CLI 注入 | https://infisical.com | https://infisical.com/docs/documentation/getting-started/cli 、https://infisical.com/docs/cli/reference | https://github.com/Infisical/infisical （30k stars，MIT，ee 目录除外） |
| HashiCorp Vault | 企业级机密管理/加密即服务（BUSL 1.1 源码可用许可） | https://www.vaultproject.io （308 重定向）→ https://developer.hashicorp.com/vault | https://developer.hashicorp.com/vault/docs 、https://developer.hashicorp.com/vault/docs/agent-and-proxy/agent | https://github.com/hashicorp/vault |
| OpenBao | Vault 的社区 fork（Linux Foundation / OpenSSF 治理，MPL 2.0） | https://openbao.org | https://openbao.org/docs 、https://openbao.org/docs/install/ 、https://openbao.org/docs/agent-and-proxy/agent/process-supervisor | https://github.com/openbao/openbao （8.7k stars，MPL-2.0） |
| SOPS（+age） | 文件级加密工具（YAML/JSON/ENV/INI/二进制），CNCF Sandbox（原 Mozilla 项目） | https://getsops.io | https://getsops.io/docs/usage/advanced | https://github.com/getsops/sops |
| age | 现代文件加密工具/格式（age-encryption.org/v1），SOPS 的推荐本地密钥后端 | https://age-encryption.org （302 重定向至 GitHub） | https://github.com/FiloSottile/age#readme | https://github.com/FiloSottile/age （24.1k stars，BSD-3-Clause） |
| direnv | shell 级目录环境变量加载/卸载器（`.envrc`） | https://direnv.net | https://direnv.net （官网即文档） | https://github.com/direnv/direnv （v2.38.2，2026-10） |
| dotenvx | "A secure dotenv"——加密 .env + 进程级注入（dotenv 原作者出品） | https://dotenvx.com | https://dotenvx.com/docs （官网内文档；run 机制已在官网首页与仓库 README 核实） | https://github.com/dotenvx/dotenvx （BSD-3-Clause） |
| （基准）1Password CLI | `op run` 注入语义基准 | https://1password.com | https://www.1password.dev/cli/secrets-environment-variables | 闭源 |

注：SOPS 官网 getsops.io 于本日通过其 `/docs/usage/advanced` 页实测可达；Infisical GitHub 仓库 README 标注许可证为 "MIT with the exception of the ee directories"。

---

## 1. 注入机制对比专节（本报告核心）

### 1.1 基准：1Password `op run`（语义参照）

核实: https://www.1password.dev/cli/secrets-environment-variables （2026-10-25 可达）

- `.env` 文件（或 shell 环境变量）中存放的是 **secret reference**（`op://<vault>/<item>/<field>` URI），不是明文。
- `op run --env-file="./.env" -- <command>` 启动**子进程**：解析所有 reference → 在内存中解密 → 以环境变量注入子进程。官方表述："runs the provided command in a subprocess with the secrets made available as environment variables **only for the duration of the process**"。
- **生命周期**：环境变量仅随子进程存活；子进程退出即消失，不污染父 shell，官方文档未提及任何明文落盘（.env 里只有 URI）。
- 补充 flag：`--environment <ID>`（注入 1Password Environment 全部变量）、`--no-masking`（关闭输出遮蔽）。
- 与 `op inject` 的区别：`op inject` 是"模板 → 配置文件"渲染（写文件），`op run` 是"纯环境变量注入"（不写任何文件）。
- 官方安全提示：同用户运行的其他进程可以读取彼此的环境变量。

这就是 F8 的黄金语义：**引用占位符 + 内存解密 + 进程级注入 + 进程结束清理 + 零明文落盘**。

### 1.2 Infisical `infisical run`

核实: https://infisical.com/docs/documentation/getting-started/cli 、https://infisical.com/docs/cli/reference （2026-10-25 可达）

- 语义与 `op run` 同族：`infisical run --env=dev -- npm run dev`（CLI ≥ 0.43.145 也可写 `infisical sm run`）。官方描述："Inject secrets from Infisical into a process as environment variables"。
- **注入级别**：进程级（子进程环境变量），语言/框架无关；`--` 后接实际命令，`--command` 可串联 shell 命令。
- **保护规则**：注入值会覆盖 shell 中已有同名变量；但 `HOME`、`PATH`、`USER`、`TERM`、`SHELL`、`XDG_*`、`LC_*` 等系统关键变量**永不注入**（防破坏）。
- 其他参数：`--path="/backend"`（按文件夹注入）、`--recursive`、`--watch`（服务端密钥变更自动重启应用）、`--print`。
- **认证与缓存**：交互场景 `infisical login`（浏览器 OAuth，SSH/WSL 可 `login -i`）；无人值守用 Machine Identity 的 `INFISICAL_TOKEN`（`export INFISICAL_TOKEN=$(infisical login --method=universal-auth --client-id=<你的-client-id> --client-secret=<你的-client-secret> --silent --plain)`，示例用占位符）。登录态/token 保存在本地 CLI 目录（社区普遍记载为 `~/.infisical`，官方 CLI 文档提及登录态本地保存；该目录含登录凭证，**注意这是磁盘上的凭证缓存，不是密钥明文缓存**——密钥每次经 TLS 从服务端拉取解密到内存注入）。
- **.env 拉取模式**：`infisical secrets export --env=dev --format=dotenv --output-file=./.env` 可把密钥导出为明文 .env（官方警告：明文落盘、易过期，建议 `.gitignore` 并尽量改用 `run`）——这是"拉取落盘"模式，与 `run` 的"注入不落盘"相对。
- **Windows 支持**：✅ 官方一等公民——`winget install infisical`、Scoop、npm 三种安装方式，文档含 PowerShell/cmd 专用示例。Git Bash 下 `infisical run -- printenv <KEY>` 可用（CLI 为原生 Windows 二进制）。
- **CI 用法**：`INFISICAL_TOKEN` 环境变量 + `infisical run`，一行接入；另支持 Docker 集成与 `agent-proxy`（AI agent 占位符模式）。

### 1.3 Vault：`vault kv get` + shell、Vault Agent 模板渲染

核实: https://developer.hashicorp.com/vault/docs/agent-and-proxy/agent （2026-10-25 可达）；CLI 命令文档见 https://developer.hashicorp.com/vault/docs/commands

- **`vault kv get` + shell 导出**：`export SECRET=$(vault kv get -field=password secret/foo)`——输出经 stdout 进入 shell 变量，**留在当前 shell 直到手动 unset 或 shell 退出**，是 shell 级注入，"临时性"最弱；写脚本时易进 shell history。
- **Vault Agent（推荐方式）**：客户端守护进程 `vault agent -config=agent.hcl`，三大功能：
  - **Auto-auth**：自动认证 + token 续期，token 可写入 sink（如 file sink——token 落盘为受限权限文件）。
  - **Template 渲染**：用 Consul Template 语法（`.ctmpl`）把密钥渲染到**文件**（`destination` 指定路径）——密钥落盘（例如 server.key/server.crt），是经典模式。
  - **`env_template` + Process Supervisor Mode**（注入到环境变量的关键）：Agent 以 exec 方式启动子进程，把 `env_template` 块渲染结果作为**环境变量**注入子进程；静态密钥按 `static_secret_render_interval` 刷新，`restart_on_secret_changes = "always"`（默认）时密钥变更会重启子进程。子进程退出后环境随之消失；该模式与文件模板互斥，设计上不写明文文件。
- **缓存**：Agent cache 可缓存 token 与带租约的密钥响应（内存），支持持久化缓存（落盘可选项）。
- **Windows**：✅ 支持，且有官方文档 "Run Vault Agent as a Windows service"（https://developer.hashicorp.com/vault/docs/agent-and-proxy/agent/winsvc ，经站内搜索确认存在）。
- **CI 用法**：一般直接用 `VAULT_TOKEN=<你的-token>` + `vault kv get -field=...` 或 AppRole；注入生态（类似 `op run` 的一键命令）不如 Infisical 顺滑——Vault 没有 `vault run -- cmd` 这种命令，进程级注入必须走 Agent supervisor。

### 1.4 OpenBao：`bao` CLI 与 `bao agent`（Vault 兼容）

核实: https://openbao.org/docs/install/ 、https://openbao.org/docs/agent-and-proxy/agent/process-supervisor 、https://github.com/openbao/openbao/releases （2026-10-25 可达）

- CLI 可执行文件名是 **`bao`**（Windows 为 `bao.exe`），命令结构与 Vault 同源（fork 自 Vault 1.14 最后的 MPL 代码），`bao kv get`、`bao agent` 等对应 `vault` 同名子命令；官方支持从 **Vault CE v1.14.x 原地迁移**，且有意保持 API 兼容（多数 Vault 工具可直接对接）。任务背景中提到的 "baob" 命令**不存在**——文档与仓库中均无此命令。
- `bao agent` 同样有 **`env_template` + process supervisor mode**（目前标注 public beta）：`exec { command = [...] }` 启动子进程并注入环境变量；另有 `bao agent generate-config -type="env-template"` 辅助生成配置。社区有个 open issue（GitHub #4110）请求 `bao agent -config=... -- "$@"` 这种内联命令语法（尚未实现）。
- `bao kv get -field=...` + shell 导出的临时性与 Vault 相同（shell 级）。
- **Windows**：✅ 官方提供 `bao.exe` 与 Windows 安装指引（含页面加密硬化建议）。
- **CI 用法**：与 Vault 相同模式（token 环境变量 + CLI/Agent）。

### 1.5 SOPS + age：文件解密 → `sops exec-env` 注入

核实: https://getsops.io/docs/usage/advanced 、https://github.com/getsops/sops/releases 、https://github.com/FiloSottile/age （2026-10-25 可达）

- 工作流：`.env`/YAML/JSON 等文件被 SOPS 加密（可整文件提交 git）→ 使用时解密。
- **`sops exec-env <file> '<command>'`**："execute a command with decrypted values inserted into the environment"——把解密后的所有键值注入**子进程**环境变量，明文不落盘；也可 `sops exec-env file.env 'sh'` 起一个临时 shell。进程结束，注入的环境消失。语义上与 `op run`/`infisical run` 同级（进程级注入）。
- `sops exec-file`：解密到临时文件给子进程；默认走 **FIFO（命名管道）不落盘**，FIFO 不可用时用 `--no-fifo` 改为自动清理的临时文件——官方明确把 **Windows 列为 FIFO 不可用环境**。
- **Windows（用户主力环境，重点核实）**：
  - SOPS 本体提供 `sops.exe`（Windows 支持 OK）。
  - `exec-env` 官方文档**未明确声明** Windows 不可用，但其命令以类 Unix shell（`sh -c` 风格）语义解析执行，且 `--user` 降权仅限类 Unix 平台（官方原文）；GitHub issue 区存在 exec-env 在特殊字符/多行值上的多个平台相关 bug（#628、#784、#1127），**在 Windows（含 Git Bash）上 exec-env 属于"未获官方背书、实测易踩坑"**，可靠性显著低于 macOS/Linux。结论：Windows 上更稳妥的是 `sops decrypt --output-type dotenv` + 自行导入，或 `exec-file --no-fifo`。
  - age 在 Windows 支持良好（winget/Chocolatey/Scoop/官方二进制）。
- 密钥管理：`.sops.yaml` 创建规则按路径选加密后端；本地场景用 **age** 密钥（`age-keygen` 生成 `AGE-SECRET-KEY-1...`，v1.3.0+ 支持后量子混合密钥 `AGE-SECRET-KEY-PQ-1...`）；私钥文件默认 `~/.config/sops/age/keys.txt`（**落盘，需自行保护**）。
- **CI 用法**：`SOPS_AGE_KEY_FILE=<路径>`（或 `SOPS_AGE_KEY` 直接传私钥内容）+ `sops exec-env`/`decrypt`，GitOps 生态（Flux/Argo CD SOPS 插件）成熟。

### 1.6 direnv 与 dotenvx：`.env` 文件注入模式与"临时性"差异

核实: https://direnv.net 、https://github.com/direnv/direnv/releases 、https://dotenvx.com 、https://github.com/dotenvx/dotenvx （2026-10-25 可达）

**direnv —— shell 级注入**
- 原理：shell hook 在每次提示符前检查当前/父目录的 `.envrc`（可选 `.env`），在 **bash 子进程**中执行 `.envrc`，只把"environment diff"导回当前 shell——所以变量**加载进你的 shell 本身**，`cd` 离开目录时自动 `unload`（恢复 diff）。
- 临时性：比"进程级"弱——变量活在你的 shell 会话里，任何同 shell 启动的程序（包括你忘了关的编辑器、补全进程）都能读到；unload 依赖 `cd` 触发，shell 崩溃/直接杀进程时不会清理。
- 安全机制：`.envrc` 必须先 `direnv allow .` 授权（内容变更后需重新授权）。
- **Windows：官方文档前置条件为 Unix-like 系统，未列 Windows 为支持平台**（用户主力 Windows → 不推荐）。
- 许可证 MIT；最新 v2.38.2（2026-10）。

**dotenvx —— 进程级注入 + 文件加密**
- `dotenvx run -- node index.js`：读取 `.env`（含加密值）→ 解密 → 注入**子进程**环境变量（"injected env (N) from .env"）。默认"已存在环境变量优先于文件"，`--overload` 反转。
- 加密：`dotenvx encrypt` 把值变成 `encrypted:...` 密文，密钥为 `DOTENV_PRIVATE_KEY`（secp256k1 ECIES 封装 AES-256 临时密钥）；加密后的 .env 可直接提交 git，私钥按环境命名（`DOTENV_PRIVATE_KEY_PRODUCTION` 等），可存 OS 密钥库（macOS Keychain / **Windows Credential Manager** / Linux Secret Service）或 1Password、Bitwarden。
- 临时性：进程级（与 op run/infisical run 同级），但**解密私钥本身**常驻环境变量或密钥库，`run` 只保证"值"不落盘。
- **Windows：✅** 官方 winget 安装（`winget install dotenvx`），另有 npm/Docker/BSD-3-Clause 源码。
- CI 用法：CI 平台里设 `DOTENV_PRIVATE_KEY_CI` secret 即可，一行接入。

### 1.7 注入机制逐项对比表

| 维度 | op run（基准） | infisical run | vault kv get + shell | Vault Agent env_template | bao agent env_template | sops exec-env | direnv | dotenvx run |
|---|---|---|---|---|---|---|---|---|
| 注入级别 | 进程级 | 进程级 | shell 级 | 进程级（Agent 为父进程） | 进程级（beta） | 进程级 | **shell 级** | 进程级 |
| 结束清理保证 | ✅ 进程退出即失效（官方明示） | ✅ 进程退出即失效 | ❌ 留在 shell，需手动 unset | ✅ 子进程退出即失效 | ✅ 同 Vault | ✅ 进程退出即失效 | ⚠️ 仅 cd 离开时 unload | ✅ 进程退出即失效 |
| 明文落盘 | 无（.env 只存引用） | 无（注入内存；`secrets export` 是可选的落盘模式） | ⚠️ history/脚本风险 | 文件模板模式落盘；env_template 不落盘；token sink 落盘 | 同 Vault | exec-env 不落盘；exec-file 默认 FIFO 不落盘 | .envrc 若含明文即落盘 | 无明文（.env 是密文）；私钥落盘/密钥库 |
| 磁盘缓存位置 | 无明文缓存 | `~/.infisical`（登录凭证） | 无 | token sink 文件、可选持久缓存 | 同 Vault | age 私钥文件 | `.direnv/` 缓存 | OS 密钥库 |
| Windows 支持 | ✅ | ✅ winget/Scoop/npm | ✅ | ✅（含 Windows Service 文档） | ✅ bao.exe | ⚠️ 本体✅，exec-env 未获官方背书、FIFO 不可用 | ❌ 官方仅 Unix-like | ✅ winget |
| Git Bash 可用性 | ✅ | ✅ | ✅ | ✅（配置文件驱动） | ✅ | ⚠️ 降级方案可用 | ❌ | ✅ |
| 密钥来源 | 1Password 云/本地库 | Infisical 服务端（自托管可用） | Vault 服务端 | Vault 服务端 | OpenBao 服务端 | 加密文件（age/KMS 解密） | 明文 .env/.envrc | 加密 .env |
| CI 用法 | Service Account + op run | INFISICAL_TOKEN 一行接入 | VAULT_TOKEN + kv get | 需部署 Agent | 需部署 Agent | SOPS_AGE_KEY + exec-env | 不适用 | DOTENV_PRIVATE_KEY_CI |

**小结**：真正对齐 `op run`"进程级 + 结束即清 + 不落盘"语义的，是 **infisical run / dotenvx run / sops exec-env / (bao|vault) agent env_template** 四类；`direnv` 是 shell 级，临时性最弱；`vault kv get + shell` 导出是半吊子注入。对 Windows + Git Bash 用户，**infisical run 与 dotenvx run 是开箱即用度最高的**，sops exec-env 需降级使用，direnv 基本出局。

---

## 2. 私有部署（F6）

### Infisical 自托管
- 核实: https://infisical.com/docs/self-hosting/deployment-options/docker-compose （2026-10-25 可达）
- Docker Compose 三容器：backend（Node/TS + Go）+ PostgreSQL（存加密后密钥，卷 `pg_data` 是唯一真相源）+ Redis；可选 nginx HTTPS。
- **资源需求（官方最低/推荐）**：CPU 2/4 核，内存 4/8 GB，磁盘 20/50 GB（SSD）；需 Docker 20.10+、Compose 2.0+。自托管还支持 Kubernetes(Helm)、Linux 包、气隙部署（气隙属企业档）。
- 许可边界：核心平台 **MIT 永久免费自托管**（GitHub 仓库 MIT，`ee` 目录为企业功能需商业许可）。需付费授权的自托管功能（官网+定价页）：SSO 强制、LDAP、SCIM、KMIP、HSM、审计日志流式传输、RBAC 等。对个人/小团队场景（密钥管理 + CLI 注入），MIT 版完全够用。
- 升级运维注意：升级前备份 PostgreSQL 卷与 `.env` 中的 `ENCRYPTION_KEY`（该密钥加密全部数据，丢失即数据不可恢复）；PG 5432 / Redis 6379 不得暴露公网。

### Vault（BUSL 许可证变更史）
- **时间线**（核实: https://www.hashicorp.com/blog/hashicorp-adopts-business-source-license 经检索确认 + 检索结论）：Vault 自始至终为 **MPL 2.0**，直至 **2023-08-10** HashiCorp 宣布全线产品改用 **BUSL 1.1**（源码可用、4 年后逐版本转回 MPL 2.0、禁止提供竞争性商业产品）；首个 BUSL 版本为 **Vault 1.15.0**（2023-10），**Vault ≤ 1.14.x 保持 MPL 2.0**。
- 对自部署的影响：BUSL 1.1 **允许内部生产使用**（个人/公司内部自部署不付费、不违法），但 (1) 源码不再是 OSI 开源，(2) 不能拿它做竞争性商业服务，(3) 部分高级功能（HSM、DR、Namespaces 等）仍锁在企业付费版，(4) 2025 年 HashiCorp 被 IBM 收购后社区对许可走向的疑虑加深——这正是 OpenBao 诞生的土壤。
- 自部署形态：单二进制 + 存储后端（文件/Consul/PG 等）；生产需要考虑 unseal（shamir 或 auto-unseal）、TLS、audit device。

### OpenBao
- 核实: https://openbao.org 、https://github.com/openbao/openbao （2026-10-25 可达）
- Linux Foundation 旗下 **OpenSSF** 治理项目（Sandbox；官网自称 "a Series of LF Projects, LLC"），**MPL 2.0**，完全免费。fork 自 Vault 1.14（最后的 MPL 版本），支持 Vault CE 1.14.x 原地迁移。
- 部署：官方 Docker 镜像（openbao/openbao，Alpine/UBI）、Helm、二进制、各发行版包管理器；2.7.0 起内置 seal 改为插件化（openbao/openbao-plugins）、新增 PebbleDB 存储后端、后量子 TLS。
- 与 Vault 的差异：去掉企业版锁与 BUSL，社区自治；API 层面刻意保持兼容（Vault CLI/SDK 大多可直接用）；功能演进已开始分叉（如 workflow、namespace sealing 等 Vault 没有的特性）。对"自己服务器 + 零许可焦虑"的场景是 Vault 的直接替代。

---

## 3. 加密架构

### Infisical —— 服务端集中加密（非零知识，如实标注）
- 核实: https://infisical.com/docs/documentation/platform/kms 、https://infisical.com/docs/documentation/platform/kms/concepts 、官网安全页（2026-10-25 可达）
- **它是服务端加密模型，不是零知识客户端加密**：密钥明文在服务端内存中用于加解密；用户/CLI 经 TLS 取回解密后的值。KMS 产品线的表述也印证模型："Keep cryptographic keys in Infisical instead of in your applications"、"The key itself never reaches the application"（密钥不出服务，但 Infisical 自己持有密钥）。
- 结构：自托管实例用 `.env` 里的 **`ENCRYPTION_KEY`**（根密钥）保护整个数据库中的密文；KMS 模块内为 root key → 各密钥（每把密钥再被 root key 加密）的层级，root key 可外置 HSM（企业档）。静态加密 **AES-256-GCM**，传输 TLS 1.2+，企业档有 FIPS 140-3 模式。
- 含义：**服务器（你自己的）被攻破 + ENCRYPTION_KEY 泄露 = 全部密钥泄露**；这与 1Password/Bitwarden 的零知识（主密码派生密钥、服务端只见密文）是不同信任模型。

### Vault —— shamir 封装 + 存储后端加密 + Transit
- 核实: https://developer.hashicorp.com/vault/docs 及 Agent 文档（2026-10-25 可达）
- 启动语义：Vault 启动后处于 sealed 状态，存储后端（文件/Consul/PG/集成存储）中的数据永远是密文；unseal 需要 **Shamir 密钥分片**（默认 N-of-M 门限）或 auto-unseal（云 KMS/HSM/PKCS#11，2.7 起在 OpenBao 为插件）。主密钥不落盘，只在内存用于加密壁垒（barrier）。
- **Transit 引擎 = 加密即服务**：应用把数据送来，Vault 加解密后返回结果，应用永远拿不到密钥（与 Infisical KMS 同思路，先来后到的问题）。
- 对比 Infisical：Vault 的信任模型相同（服务端持钥），但多了"密封"这层冷启动保护——重启后必须人工/自动 unseal 才能服务，被攻破的持久化磁盘拿不到明文。

### SOPS + age —— 文件级端到端加密（离线、无服务端）
- 核实: https://getsops.io/docs/usage/advanced 、https://github.com/FiloSottile/age （2026-10-25 可达）
- SOPS 只加密**文件的值**（键名、注释明文保留），支持 YAML/JSON/ENV/INI/二进制；每文件的数据密钥被各"接收者"（age 公钥、AWS/GCP/Azure KMS、PGP）分别加密，支持**密钥组 + Shamir 分片**（N-of-M 后端组合解密）。
- age：极简文件加密格式（age-encryption.org/v1），X25519 密钥对（v1.3.0+ 可选 mlkem768x25519 后量子混合），密钥短小、零配置、可管道组合；另有 Rust 实现 rage、TS 实现 Typage。
- 信任模型与上两者相反：**无服务端、纯本地端到端**，密文随 git 分发，私钥在你手里——最接近"密钥文件本地 + 主密钥不上云"（F4）的哲学，但没有团队共享 UI/审计。

---

## 4. 收费

| 工具 | 云/SaaS | 自托管 | 核实 |
|---|---|---|---|
| Infisical | Secrets Management 按身份计费：Free $0（5 身份、3 环境、无 RBAC/SSO/轮换）；Pro $20/身份/月（年付，月付 $23）；Advanced $40；Enterprise 定制（SSO/SCIM/审计流/KMIP/HSM、99.99% SLA）。PAM $20/用户/月起 | MIT 核心永久免费；受支持企业版需联系销售报价 | 核实: https://infisical.com/pricing （2026-10-25 可达） |
| Vault | HCP Vault Dedicated 按小时计费：Dev 档约 $0.03/小时（约 $22/月），生产档约 $1.58–$9.41/小时（约 $1,150–$6,900/月，第三方向 2026 估价，官方按规格/区域浮动）；另有企业授权（未公开价） | 社区版（BUSL）二进制免费自部署；企业功能另购 | 核实: https://www.hashicorp.com/en/pricing 官方定价页（具体小时价来自检索，标注为第三方估计）；GitHub Releases 实测 |
| OpenBao | 无云版、无付费档 | 完全免费（MPL 2.0） | 核实: https://openbao.org （2026-10-25 可达） |
| SOPS / age | 无 | 完全免费（MPL 2.0 / BSD-3-Clause） | 核实: https://github.com/getsops/sops 、https://github.com/FiloSottile/age （2026-10-25 可达） |
| direnv / dotenvx | 无（dotenvx 自称 free & open source，有增值云 Armor ⛨） | 免费（MIT / BSD-3-Clause） | 核实: https://direnv.net 、https://dotenvx.com 、https://github.com/dotenvx/dotenvx （2026-10-25 可达） |

---

## 5. 安全性：审计史、CVE、最新版本（截至 2026-10-25）

| 工具 | 审计史 | 近期 CVE/安全事件 | 最新版本 |
|---|---|---|---|
| Infisical | SOC 2 Type II、HIPAA、GDPR 合规（官网宣称）；**未见公开的第三方代码审计报告** | 无重大公开 CVE 记录（本调研范围内） | CLI 0.43.145+（CLI 参考文档提及该版本线），平台持续高频发版（30.5k commits） |
| Vault | 独立第三方审计：Vault 1.9 由 **Kroll** 审计，更早版本由 **ATSEC International** 审计（报告发布于 HashiCorp 官网安全页，经检索确认） | 2026 年多个：HCSEC-2026-32 / CVE-2026-5006（模板策略路径斜杠注入→认证绕过/提权，2026-08-24 修复）；CVE-2026-89322（1.21.12 修复）；cache-key collision 策略别名解析漏洞（v2.2.1 修复）；另有 ControlPlane 披露的 4 漏洞链可完全接管 Vault/OpenBao 服务器（2026-09，相关补丁已发） | v2.2.1（2026-10-22，GitHub Releases 实测 Latest；1.21.x LTS 线并行维护至 1.21.12+） |
| OpenBao | ControlPlane 2026 年研究（漏洞链披露）；OpenSSF 治理、安全邮箱 openbao-security@lists.openssf.org | 代码执行漏洞链（v2.6.3/2.7.0 修复）；信息泄露 CVE 影响 2.2.0–2.6.3 与 2.7.0（v2.6.4 / v2.7.1 修复，2026-10-01，含 6 项 GHSA） | v2.7.1（2026-10-01，GitHub Releases 实测 Latest） |
| SOPS | CNCF Sandbox；仓库 Security 页记载已修复漏洞（exec-file/exec-env 切换用户时 UID 设为 GID，仅 Unix）——无重大公开 CVE | 低 | v3.13.3（2026-07-23，GitHub Releases 实测） |
| age | **无公开商业审计报告**（检索未发现 Cure53/ROS 报告）；格式有学术形式化分析（ProVerif/Tamarin 方向的 "Formally Verifying the age encryption format" 研究）；作者为前 Go 安全负责人 Filippo Valsorda | 低 | v1.3.2（含后量子密钥支持，GitHub 实测） |
| direnv | 未见公开审计 | 无重大公开 CVE 记录 | v2.38.2（2026-10，GitHub Releases 实测） |
| dotenvx | 未见公开审计（注意：工具较年轻，加密代码历史短） | 无重大公开 CVE 记录 | v1.x 活跃维护（README 标签示例 1.28.0） |

> Vault/OpenBao 的 2026 漏洞集中在其核心 ACL/策略解析，均已有补丁版本——它们属于"高价值攻击面、需要跟上补丁节奏"的系统；SOPS/age 攻击面小得多（本地文件工具）。

---

## 6. F1–F10 逐项对照表

适用性说明：这一组工具是**机密管理（secrets management）**而非**个人密码管理器**——它们管理 API key、数据库凭证、证书，不做登录密码/身份信息管理，多数没有浏览器自动填充。对个人密码管理场景的适用性如实标注：

| 编号 | 功能 | Infisical | Vault | OpenBao | SOPS+age | direnv | dotenvx |
|---|---|---|---|---|---|---|---|
| F1 | 自动保存/填充（浏览器/应用） | ❌（无浏览器插件自动填充；是密钥平台不是密码管理器） | ❌ | ❌ | ❌ | ❌ | ❌ |
| F2 | 本地加解密 | ❌ 加解密在服务端进行（客户端只取值） | ❌ 服务端加密（barrier 内） | ❌ 同 Vault | ✅ 纯本地文件加解密 | ❌（不做加密） | ✅ 解密在本地进程内（secp256k1+AES-256） |
| F3 | 云端只存密文 | ⚠️ 云版存密文但**服务端持钥**（非零知识）；自托管时"云"就是你自己的服务器 | 同左（服务端持钥） | 同左 | ✅ 密文文件可公开分发（git/云盘皆可） | ❌ 明文 .envrc | ✅ 加密 .env 随 git 分发 |
| F4 | 密钥文件本地 + 主密码不上云 | ❌（密钥在服务器 DB，靠 ENCRYPTION_KEY 保护） | ⚠️ 数据落你自己的存储，unseal 密钥可离线保管 | 同 Vault | ✅ age 私钥文件本地 + 可加口令，无任何服务端 | ❌ | ✅ 私钥本地/OS 密钥库 |
| F5 | 跨平台 | ✅ 全平台 CLI + Web UI | ✅ | ✅（含 Windows bao.exe） | ✅（SOPS 各平台；exec-env Windows 受限） | ❌ 官方仅 Unix-like | ✅ 含 winget |
| F6 | 私有部署 | ✅ Docker Compose/K8s，MIT 版免费（4GB 内存起） | ✅ BUSL 允许内部自部署（许可需读懂） | ✅ 完全开源免费 | ✅ 无服务器，天然"私有" | — | — |
| F7 | CLI | ✅ 功能完整（run/secrets/import/export/scan） | ✅ 功能完整 | ✅（bao，兼容 Vault CLI 语法） | ✅ | ✅ | ✅ |
| F8 | 临时注入环境变量 | ✅✅ `infisical run`——与 op run 同语义，Windows 一等公民 | ⚠️ 无一键注入命令；Agent env_template 或 kv get+shell | ⚠️ 同 Vault（bao agent，beta） | ⚠️ exec-env 语义正确但 Windows 未获官方背书 | ⚠️ shell 级、临时性弱 | ✅ dotenvx run 进程级注入 |
| F9 | 批量导入 | ✅ `secrets import`（.env 等）+ 控制台 Secret Syncs（含 1Password/Vault/AWS 同步） | ✅ API/CLI 写入方便 | ✅ 同 Vault | ⚠️ 需自己写脚本转换 | ⚠️ 就是复制 .env | ✅ 天然 .env 生态 |
| F10 | 批量导出 | ✅ `secrets export`（.env/JSON/stdout） | ✅ kv get/`vault kv export` | ✅ 同 Vault | ✅ sops decrypt 即导出 | ✅（本就是明文文件） | ✅ dotenvx get --all 等 |

**结论性观察**：这组工具在 F1（自动填充）上全军覆没——它们从设计上就不是密码管理器；F2/F4（本地端到端）只有 SOPS+age 与 dotenvx 做到；F8 是 Infisical / dotenvx 最接近 1Password 基准；F6 上 Infisical（MIT）与 OpenBao（MPL）最省心，Vault 许可与运维成本最高。

---

## 7. 结论：面向"自研 SuperPasswdKey + 自己服务器"的可复用性

### 7.1 可直接整体复用（部署即用）
- **Infisical = 本组最接近"直接满足 F6+F7+F8"的成品**：自己的服务器上 `docker compose up`（4GB 内存起，MIT 免费），装 Windows CLI，`infisical run --env=dev -- <命令>` 即获得与 `op run` 同级的"临时注入、进程结束即清、不落盘"体验；`secrets import/export` 覆盖 F9/F10；Web UI 弥补其无 F1 的短板。代价：信任模型是**服务端持钥**（非零知识），且平台较重（PG+Redis+backend）、版本迭代快需跟升级。
- **OpenBao = 免费开源的 Vault 替身**：MIT/MPL 焦虑为零、`bao agent` 进程监督注入（beta）可用，适合想要 Vault 能力（动态密钥、Transit、PKI）且自建在个人服务器上的场景；但无一键 `run` 命令、Agent 配置门槛高，F8 体验逊于 Infisical。

### 7.2 只适合做组件/借鉴
- **SOPS + age 的加密方案**：SOPS"只加密值、密文随 git 走、数据密钥被多个接收者各加密一份、支持 Shamir 密钥组"的文件格式设计，是自研 F2/F3/F4（本地加密、密文上云、私钥不落云）时最值得抄的架构；age 的 X25519 密钥模型（短密钥、零配置、可后量子混合）值得作为自研的底层原语。`exec-env` 的进程级注入语义在 Windows 上不可依赖，可自行实现（解密→内存→`os/exec` 注入→进程退出清理）。
- **direnv 的注入模式**：仅当目标是 Unix shell 开发工作流时参考；其"environment diff + 自动 unload"机制可借鉴，但 shell 级注入不符合 F8 的临时性要求，且 Windows 无官方支持——对 SuperPasswdKey 价值有限。
- **dotenvx 的工程折中**：".env 密文化 + 私钥进 OS 凭据管理器 + 进程级注入"是轻量级路线的代表，若 SuperPasswdKey 走"无服务器、纯本地"路线，dotenvx 证明了这个极简形态可行（但无审计背书、无团队 UI，加密实现年轻）。

### 7.3 一句话建议
- 想"**今天就可用、零自研**"：自部署 **Infisical + `infisical run`**，即同时点亮 F6/F7/F8（F9/F10 顺带解决），接受服务端持钥模型。
- 想"**自研加密内核**"：以 **age（密钥/加密原语）+ SOPS 文件格式思想**为底座，实现自己的"解密→进程环境注入→退出即清"运行时（对标 `op run` 语义），Windows 上用 Go `os/exec` 自行实现注入而不依赖 sops exec-env。
- **Vault/OpenBao** 对个人密码管理场景过重（unseal、策略、租约体系），仅在需要动态密钥/证书体系时考虑。

---

## 附：主要核实来源清单（均为 2026-10-25 实测可达）

- Infisical：https://infisical.com ｜ https://infisical.com/docs/documentation/getting-started/cli ｜ https://infisical.com/docs/cli/reference ｜ https://infisical.com/docs/self-hosting/deployment-options/docker-compose ｜ https://infisical.com/docs/documentation/platform/kms/concepts ｜ https://infisical.com/pricing ｜ https://github.com/Infisical/infisical
- Vault：https://developer.hashicorp.com/vault ｜ https://developer.hashicorp.com/vault/docs/agent-and-proxy/agent ｜ https://github.com/hashicorp/vault/releases ｜ 许可变更 https://www.hashicorp.com/blog/hashicorp-adopts-business-source-license （经检索确认）
- OpenBao：https://openbao.org ｜ https://openbao.org/docs ｜ https://openbao.org/docs/install/ ｜ https://openbao.org/docs/agent-and-proxy/agent/process-supervisor ｜ https://github.com/openbao/openbao ｜ https://github.com/openbao/openbao/releases
- SOPS/age：https://getsops.io/docs/usage/advanced ｜ https://github.com/getsops/sops ｜ https://github.com/getsops/sops/releases ｜ https://github.com/FiloSottile/age
- direnv/dotenvx：https://direnv.net ｜ https://github.com/direnv/direnv/releases ｜ https://dotenvx.com ｜ https://github.com/dotenvx/dotenvx
- 1Password 基准：https://www.1password.dev/cli/secrets-environment-variables
