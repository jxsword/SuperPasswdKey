# 分报告 B：Bitwarden 体系调研（官方 SaaS/自部署、Vaultwarden、Secrets Manager）

> 调研子代理 T2 产出 | 调研日期：2026-10-25
> 基准：以 1Password 收费全功能为对照，重点考察 F6 私有部署、F7 CLI、F8 CLI 临时环境变量注入。
> 所有 URL 均经实测，标注（核实: 2026-10-25 可达）。示例命令一律使用占位符，不含真实凭据。

---

## 一、Bitwarden 官方（SaaS + 官方自部署）

### 1.1 三件套（均已实测）

| 项 | URL | 状态 |
|---|---|---|
| 官网 | https://bitwarden.com/ | （核实: 2026-10-25 可达） |
| 官方文档 | https://bitwarden.com/help/ | （核实: 2026-10-25 可达） |
| 源码 | https://github.com/bitwarden （组织：server、clients、android、ios 等 69 个仓库） | （核实: 2026-10-25 可达） |

关键子页（本次实测）：定价 https://bitwarden.com/pricing/ ；CLI 文档 https://bitwarden.com/help/cli/ ；安全白皮书 https://bitwarden.com/help/bitwarden-security-white-paper/ ；自部署 https://bitwarden.com/help/hosting/ ；Lite 部署 https://bitwarden.com/help/install-and-deploy-lite/ ；审计页 https://bitwarden.com/help/is-bitwarden-audited/ ；1Password 导入 https://bitwarden.com/help/import-from-1password/ 。

### 1.2 简介

Bitwarden 是最主流的开源密码管理器之一（GPL/AGPL 系），客户端（网页、浏览器扩展、桌面、移动、CLI）与服务端全部开源，采用零知识端到端加密。商业形态为：云端 SaaS、官方自部署（标准版 + Lite）、以及 Secrets Manager（开发者密钥管理）。覆盖 Windows/macOS/Linux/Android/iOS 及全部主流浏览器。

### 1.3 收费（核实自 pricing 页）

| 套餐 | 价格（年付） | 说明 |
|---|---|---|
| Free | $0 | 无限设备、无限密码条目（早期 1Password 式条目数限制 Bitwarden 没有）；可与另 1 名用户共享；2FA 仅限验证器 App/邮件 |
| Premium | $1.65/月（≈$19.80/年） | 解锁：条目内集成 TOTP 生成器、加密附件（1GB）、保险库健康报告、YubiKey/Duo/FIDO2 两步登录、2FA via hardware key |
| Families | $3.99/月（≈$47.88/年） | 最多 6 人，人人享有 Premium 功能 |
| Teams | $4/月/用户 | |
| Enterprise | $6/月/用户 | SSO、企业策略、SCIM、管理员密码重置等 |

要点：**免费版条目数无上限**（与 LastPass 等"免费限一台设备"的路线不同）；**TOTP"生成器"是 Premium 功能，免费档不可用**——这对 F8/开发场景是隐藏收费点；旧文档中"免费版限两个条目"的说法已过时，以 pricing 页为准。

### 1.4 加密架构（核实自安全白皮书）

- **零知识**：主密码"从不经互联网传输到 Bitwarden 服务器"，Master Key/Stretched Master Key 也从不上传；服务器只存密文与二次加盐哈希。加密/哈希在本地设备完成后才同步。
- **KDF**：默认 PBKDF2-SHA256，600,000 次迭代（邮箱作盐）；可切换 **Argon2id**，官方文档当前默认参数为 memory 32 MiB / iterations 6 / parallelism 4（2023 年最初上线时为 64 MiB/3/4，后有调整；上限 1024 MiB/1024/16）。社区加固建议：最慢解锁设备上做到约 1 秒解锁。
- **算法**：AES-256-CBC + HMAC-SHA256（Encrypt-then-MAC）；每条 Cipher 有独立随机 64 字节密钥，再被用户/组织对称密钥包裹；组织共享走 RSA-OAEP 非对称分发；HKDF 拉伸至 512 位 Stretched Master Key。
- **传输/存储**：TLS + HSTS；Azure 数据库 TDE + 敏感列列级加密。**对"云端只存密文"(F3) 的要求，官方架构完全满足，且白皮书明确服务器无明文还原能力。**

### 1.5 自部署（F6）

两种官方形态：

| 形态 | 资源需求（核实自部署文档/Lite 文档） | 特点 |
|---|---|---|
| **标准版（Standard）** | 最低 x64 1.4GHz 双核 2GHz / **2GB RAM** / 12GB 盘，推荐 4GB RAM/25GB 盘；Docker Engine 26+；开放 80/443 | 完整功能，自带 MSSQL Express（10GB 上限），需到 bitwarden.com/host 领取安装 ID/密钥 |
| **Lite（原 Unified，2025-12 从 Beta 毕业）** | 最低 **200MB RAM** / 1GB 盘；支持 ARM（树莓派/NAS） | 单镜像，数据库自备（SQL Server/PostgreSQL/SQLite/MySQL 均可），仅面向个人/homelab（官方明确"不用于商业场景"）；镜像 `ghcr.io/bitwarden/lite` |

另有 Helm/K8s、离线部署、市场镜像等路线。自部署免费（无官方收费门槛），但**部分企业功能（SSO、Key Connector、企业策略、BSM 等）仍需付费订阅**；安装时需联网向 Bitwarden 领取安装密钥（离线部署除外）。

### 1.6 CLI（F7/F8/F9/F10）

`bw`（TypeScript/Node，npm 包 `@bitwarden/cli`，另有原生二进制/choco/brew/snap）。

- **解锁与会话**：`bw unlock` 生成 session key，官方输出 `export BW_SESSION=<session-key>`；设置该环境变量后后续命令免输密码。**session key 只存在于环境变量/进程环境中，bw 自身不落盘**；`bw lock`/`bw logout` 使其失效；不跨新终端。新特性：2026.9+ 桌面端支持 `bw unlock` 走桌面生物识别（IPC），失败回退密码提示（核实自 clients 仓库 apps/cli/README）。
- **`bw get`**：取 item/username/password/totp/uri/attachment 等，`bw get password <条目名或ID> --raw` 输出裸文本到 stdout。
- **`bw run`：重要核实结论——当前版 CLI 已无 `bw run`**。官方帮助页 CLI 文档无此命令（核实：帮助页 Core/Other Commands 均未列出）；对开源仓库 `bitwarden/clients/apps/cli/src` 的命令注册（`vault.program.ts`、`register-oss-programs.ts` 等）核查，现存子命令为 list/get/create/edit/delete/restore/confirm/import/export/encode/status/serve 等，**不存在 run**。历史版 CLI 曾有基于 JSON 配置的 `bw run`（配置 env 绑定后执行命令序列），现已被移除且不再有文档。若你沿用旧脚本，需注意该命令在新版 `@bitwarden/cli`（2026.10.0）上不可用。
- **F8 结论：bw 没有与 1Password `op run --` 等价的原生临时注入命令**（社区文章亦明确指出此缺口，如 noboxdev.com 的 1Password CLI 替代对比）。社区惯用做法及风险：
  ```bash
  # 占位示例：内联命令替换模拟注入
  export BW_SESSION=$(bw unlock --raw --passwordenv MP_ENV)   # <session-key> 仅进环境变量
  docker run -e DB_PASSWORD="$(bw get password 'db-pass' --raw)" myimage
  ```
  与 `op run` 的差异与风险：1) `$(bw get ...)` 展开结果**进入外层 shell 的命令行/子进程参数**，可能被 `ps -ef`、shell history、CI 日志捕获；2) 秘密在 shell 变量中存留到脚本结束，无自动清除；3) `bw serve`（本地 REST API）是官方给出的另一条路，但把明文暴露在本地端口上。需要"子进程环境变量、父进程不留痕"的严格 F8 语义时，Bitwarden 生态内应改用 **BSM 的 `bws run`**（见第三节）。
- **导入/导出（F9/F10）**：`bw import <format> <path>`（`--formats` 列出全部格式，含 bitwardencsv/lastpasscsv/**1password1pux/1passwordpif** 等；从 1Password .1pux 导入详见 T7 分报告）；`bw export --format json|encrypted_json|csv|zip`。**zip 格式会连同附件一起导出**（核实自 CLI 文档），encrypted_json 用账户密钥或 `--password <口令>` 二次加密——**加密归档 F10 基本满足；但"附件+加密 JSON"无法一步合成单一归档**（附件单独在 zip 里，或需脚本逐个 `bw get attachment`）。明文 JSON 导出可用于解密审计，等于可选明文解密通道。

### 1.7 安全审计与 CVE 时间线

- **审计**（核实自官方审计页 is-bitwarden-audited）：2018 起持续第三方审计；2023 Cure53（Web 应用/桌面/核心库/扩展/网络）；2024 Cure53、Mandiant、IOActive、Paragon、Fracture Labs；**最新一次：2025 年密集审计——Cure53（浏览器扩展与自动填充浮层、核心应用、桌面、RustCrypto、Web Vault）+ ETH Zurich 应用密码学小组（假设服务器完全恶意下的核心密码学操作）+ Unit 42（移动端）+ Fracture Labs（Web 与网络组件）**，报告 PDF 公开。另有 SOC 2 Type 2、ISO 27001、HackerOne 漏洞赏金。
- **2023 年"iframe/点击劫持"类 CVE 核实结论**：网传"CVE-2023-24044 / CVE-2023-43140"**均不是 Bitwarden**（经 NVD 关键词全量检索，CVE-2023-24044 实为 Plesk Obsidian 开放重定向）。Bitwarden 真实相关记录是：
  - **CVE-2018-25081**（2023-03-09 发布公告）：Bitwarden 允许在跨域 iframe 内自动填充（供应商立场：存在合法跨域场景，属"设计争议"而非典型漏洞）；
  - **CVE-2023-27974**（同日）：二级域名匹配即自动填充的问题；
  - 另有 CVE-2023-27706（Windows 桌面生物识别密钥存于本机凭据管理器，本地攻击者可读）、CVE-2023-38840（桌面 2023.7.0 本地内存信息泄露）。
- **值得写进威胁模型的新事件**：**CVE-2026-42994——npm 渠道的 Bitwarden CLI 2026.4.0 在 2026-04-22 约 1.5 小时窗口内被植入恶意代码（Checkmarx 披露的供应链事件）**。这说明"官方渠道二进制"并非绝对可信，CI 安装 bw 应锁定版本 + 校验，这在"厂商投毒"威胁模型下是官方生态的真实前科。
- 2025–2026 官方 Server 亦有多个访问控制类 CVE（如 CVE-2026-43638/43639/43640、CVE-2026-57520/57521/57522、CVE-2026-60104 等），说明官方服务端需及时跟版。
- **自部署安全注意事项**（官方文档 + Vaultwarden 实践通用）：必须 HTTPS（Web Vault 依赖 secure context 的 Web Crypto）；反向代理收口、仅暴露 80/443；SMTP 配置正确否则重置邮件外泄；定期备份 `database + attachments + keys`（标准版）/ `/data` 卷（Lite）；锁定管理端口（标准版 admin 页、BSM 无关）；服务器主机本身即"所有密文保险库"的持有者，主机沦陷=离线爆破主密码，故 KDF 参数应调高。

### 1.8 F1–F10 对照（官方 Bitwarden）

| 编号 | 结论 | 说明 |
|---|---|---|
| F1 自动保存/填充 | ✅ | 全平台浏览器扩展 + 移动端自动填充，成熟度业界第一梯队 |
| F2 本地加解密 | ✅ | 零知识，加解密全程在客户端 |
| F3 云端只存密文 | ✅ | 白皮书明确服务器仅存密文 |
| F4 密钥文件本地+主密码不上云 | ⚠️ | 主密码不上云 ✅；但**没有 KeePass 式独立密钥文件**（可加分项：两步登录/紧急访问，无 keyfile 概念） |
| F5 跨平台 | ✅ | Win/Linux/Android 官方原生支持 |
| F6 私有部署 | ✅ | 标准版（2GB RAM 级）与 Lite（200MB RAM 级）双路线 |
| F7 CLI | ✅ | `bw` 全功能（条目 CRUD、导入导出、serve API） |
| F8 临时环境变量注入 | ❌ | 无原生 `op run` 等价物；命令替换方案有 ps/history 泄露面 |
| F9 批量导入 | ✅ | CLI/GUI 多格式，含 1Password .1pux |
| F10 批量导出 | ⚠️ | json/encrypted_json/csv/zip（zip 含附件）；加密+附件可达成但无"单一加密全量归档"的一步命令 |

### 1.9 优缺点 / 适用场景 / 特色

- 优点：全栈开源、零知识架构被 ETH Zurich"恶意服务器假设"审计背书；免费档条目不限量；自部署双形态资源弹性大；生态（导入格式、第三方集成）最全。
- 缺点：TOTP 生成器、附件属 Premium 收费；CLI 无安全注入原语；标准版自部署偏重（MSSQL）；官方 Server 近年访问控制 CVE 频出，跟版压力大。
- 特色：Bitwarden Send（阅后即焚分享）、紧急访问、FIDO2/passkey、SSH 代理（新）、Directory Connector（企业目录同步——属于"官方迁移器"范畴：另有专门的导入工具 Importers，Directory Connector 仅做 AD/LDAP/GSuite/Entra ID 用户与组同步，不迁移密码条目；从 1Password .1pux 的导入能力由内置 Importer/`bw import 1password1pux` 提供，详见 T7）。
- 适用：想要"1Password 全功能 + 可自托管"的个人/小团队；对厂商信任要求高、又要保留 SaaS 便利的混合场景。

---

## 二、Vaultwarden（第三方 Rust 实现）

### 2.1 三件套（均已实测）

| 项 | URL | 状态 |
|---|---|---|
| 主页/仓库 | https://github.com/dani-garcia/vaultwarden | （核实: 2026-10-25 可达，经 GitHub API 与页面双重核实） |
| 文档 Wiki | https://github.com/dani-garcia/vaultwarden/wiki | （核实: 2026-10-25 可达，66 页，2026-10-06 仍在更新） |
| 源码 | 同上仓库（AGPL-3.0，69,616 stars，最近 push 2026-10-22） | （核实: 2026-10-25 可达） |

### 2.2 简介、活跃度与官方性质

- Bitwarden Client API 的**非官方**替代服务端实现（Rust，原名 bitwarden_rs，2021 年 v1.21.0 起更名以切割商标）。与**官方 Bitwarden 客户端**（扩展/桌面/移动）直接兼容，为官方服务端"资源太重"的自部署场景而生。
- **活跃度**：最新 release **1.37.4（2026-10-05）**；仓库 2026-10-22 仍有提交；约 7 万 stars。README 特别注明：**一位现任维护者受雇于 Bitwarden**（以个人时间贡献、经其他维护者评审），项目与 Bitwarden 官方无隶属关系。
- **免责声明原文要点**（核实自 README）：项目与 Bitwarden/Bitwarden Inc. 无关联；出 bug 请报给 Vaultwarden 而非官方支持渠道；**维护者不对使用 Vaultwarden 造成的任何数据丢失（密码、附件等）负责**，强烈建议定期备份文件与数据库。此外官方 Wiki 明确其**缺失功能**：公共 API 仅部分支持、Passkey 登录、新设备登录保护、自定义角色及部分企业策略不支持。
- 功能覆盖（README）：个人/组织保险库、Collections、共享、事件日志、Admin Password Reset、Directory Connector、策略、Send、附件、网站图标、个人 API Key、紧急访问、2FA（TOTP/邮件/FIDO2/YubiKey/Duo）、自带管理后台与改版 Web Vault（`bw_web_builds` 打包进容器）。

### 2.3 资源需求（与官方自部署对比）

- Vaultwarden **不发布正式最低配置**（社区 FAQ 定位"能跑在树莓派上"）。实测社区数据：空闲约 10–30MB RAM，生产典型 400–500MB；Pi Zero 2 W（512MB）可用。**注意：从源码编译需约 7GB RAM（Rust 构建），低配机应使用预构建 Docker 镜像**（ghcr.io/docker.io/quay.io 三渠道）。
- 对比：官方标准版最低 2GB RAM/自带 MSSQL；官方 Lite 已降到 200MB。**当前官方 Lite 出现后，Vaultwarden 的资源优势缩小，其剩余优势是：免安装密钥、单 SQLite 文件、默认解锁全部 Premium 类功能（社区公知行为，非 README 官方承诺）、配置极简。**

### 2.4 安全性：CVE 时间线与风险分析

Vaultwarden 独立承担服务端全部安全责任，CVE 流水明显高于官方 Server（NVD 全量检索核实，节选）：

| CVE | 发布 | 内容 |
|---|---|---|
| CVE-2024-39924/39925/39926 | 2024-09 | 紧急访问元数据端点鉴权缺陷；成员离组不轮换组织密钥；admin 面板 XSS/HTML 注入 |
| CVE-2024-56335 | 2024-12 | 特定条件下攻击者可更新/删除组织分组 |
| CVE-2025-24364/24365 | 2025-01 | admin 面板 RCE（需已认证）；越权获取组织 owner |
| CVE-2026-26012 | 2026-02 | 普通成员绕过集合限制读取全部组织条目 |
| CVE-2026-27801/27802/27803/27898 | 2026-03 | 2FA 绕过；批量权限提权；Manager 越权；cipher_id 越权写入 |
| CVE-2026-31835/33420 | 2026-05 | WebAuthn 元数据缺陷；集合详情端点漏 has_full_access 检查 |
| CVE-2026-43911/43912/43913/43914 | 2026-05 | 改密/KDF 变更后 refresh token 不失效；分组越组织绑定；未确认 owner 可清空组织库；邮件 2FA 下爆破保护可绕过 |
| CVE-2026-47158/47159/47160/47164 | 2026-07 | SSO OAuth state 未绑定会话；SSO 元数据泄露；icons 端点 SSRF 检查缺失；email_verified 校验缺口 |

解读：多为**授权逻辑/组织边界类**缺陷（客户端加密不受影响，密文仍安全），但说明该项目的服务端实现质量需以"高频打补丁、依赖你自己及时升级"的假设运营。

### 2.5 威胁模型："厂商投毒"下的缓解与残余

用户的威胁模型是"密码管理器厂商投毒"（即 1Password 式闭源云服务被植入后门/窃取）。Bitwarden 系的缓解与残余：

- **缓解（有效）**：
  1. 服务端开源且可自部署（官方 Server 或 Vaultwarden），云端只见密文；即使服务端整体投毒，零知识设计下拿不到主密码/主密钥（ETH Zurich 2025 审计正是"假设服务器完全恶意"验证客户端加密操作）。
  2. 客户端全开源，扩展/桌面可自审或社区审。
- **残余（无法完全消除）**：
  1. **客户端二进制/扩展仍来自官方渠道**（商店扩展、桌面安装包）——这是最尖锐的攻击面。且**已有先例：CVE-2026-42994，npm 渠道 bw CLI 2026.4.0 被投毒 1.5 小时**。商店渠道目前无 2026 类似事件，但结构上无法排除。缓解手段：锁定版本、关注 GHSA/CVE 订阅、企业内可走离线分发自建构建（代价高）。
  2. Vaultwarden 场景下信任链更长：服务端是**社区项目**（其自带改版 Web Vault `bw_web_builds` 亦是社区重打包），无官方审计背书，投毒面从"厂商"变成"维护者 + 分发镜像"——对"不信任厂商"的用户这未必更优，需自行评估 dani-garcia 及镜像仓库（ghcr.io 构建流程开源可复核）。
  3. 网页端（Web Vault）自部署时页面仍由服务端推送——**服务端投毒可直接投 JS**，这是自部署网页端的结构性残余风险；用扩展/桌面客户端（本地代码）可收窄。

### 2.6 F1–F10 对照（Vaultwarden）

客户端即官方客户端，F1/F2/F3/F5 与官方一致；服务端差异集中在 F6–F10：

| 编号 | 结论 | 说明 |
|---|---|---|
| F1 | ✅ | 同官方（官方客户端） |
| F2 | ✅ | 同官方（加密在客户端） |
| F3 | ✅ | 同官方，数据在你自己的服务器上 |
| F4 | ⚠️ | 同官方：无 keyfile 概念 |
| F5 | ✅ | 同官方 |
| F6 | ✅ | 极轻量（数百 MB RAM 级），单 SQLite 文件即可 |
| F7 | ✅ | `bw config server <你的自部署地址>` 即可指向 Vaultwarden |
| F8 | ❌ | 同官方 bw：无原生注入；另注意 Vaultwarden 不支持完整公共 API，`bw serve` 场景受限 |
| F9 | ✅ | 同官方（导入由客户端完成） |
| F10 | ⚠️ | 同官方（export 由客户端完成）；差异：无官方技术支持兜底，备份须自建且自担数据丢失风险（README 免责声明原文） |

### 2.7 优缺点 / 适用场景

- 优点：资源占用极小、部署/备份极简（SQLite 单文件）、免费获得官方 Premium 档体验（TOTP 生成器、附件等）、社区极活跃（7 万 stars、月度 release）。
- 缺点：非官方、无安全审计背书、不对数据丢失负责、CVE 频出需自觉跟版；缺 Passkey 登录、新设备登录保护、完整公共 API、部分企业策略；改版 Web Vault 引入额外信任环节。
- 适用：个人/homelab/极小团队，把"自主可控 + 低资源"放在"官方审计背书"之前；不适合有合规要求的企业。

---

## 三、Bitwarden Secrets Manager（BSM）

### 3.1 三件套（均已实测）

| 项 | URL | 状态 |
|---|---|---|
| 官网 | https://bitwarden.com/products/secrets-manager/ | （核实: 2026-10-25 可达，经官网导航确认入口） |
| 官方文档 | https://bitwarden.com/help/secrets-manager-overview/ ；CLI 文档 https://bitwarden.com/help/secrets-manager-cli/ ；套餐 https://bitwarden.com/help/secrets-manager-plans/ | （核实: 2026-10-25 可达） |
| 源码 | SDK 与 GitHub Action 开源：https://github.com/bitwarden/sdk-sm 、https://github.com/bitwarden/sm-action （服务端复用官方 server） | （核实: 2026-10-25 可达，经 bitwarden 组织页确认） |

### 3.2 定位与收费

- 定位：面向开发者/DevOps 的**基础设施密钥（API key、连接串、证书）**管理，与 Password Manager 是并列产品线、可组合；核心对象为 Secrets / Projects / Machine Accounts / Access Tokens（机器账户令牌）。
- 免费额度（核实自套餐页）：Free 计划 **2 用户 / 3 项目 / 3 机器账户，secret 存储不限量**；Teams/Enterprise 无限 secrets/项目，分别含 20/50 个机器账户。
- **自部署**：概述页写明 "Bitwarden Secrets Manager can also be self-hosted"；但套餐页将自托管标为"Enterprise、即将推出（coming soon）"。两页口径不一致，结论：**自部署能力存在但受 Enterprise 订阅与版本节奏约束，采用前需实测当前 Unified/Lite 镜像是否含 SM 组件**。
- 与 Infisical 的定位差异（详表归 T4）：BSM 是密码管理器厂商的"开发者附加品"，复用 Bitwarden 账号体系与端到端加密，偏"密码库的开发者扩展"；Infisical 是原生 secrets 平台，自带更完整的密钥轮转、动态密钥、KMS 集成与平台化工作流。

### 3.3 CLI：`bws` 与 F8

`bws`（Rust，独立于 `bw`）：

- 命令族（核实自 SM CLI 文档）：`bws secret get/list/create/edit/delete`、`bws project ...`、`bws config`、以及 **`bws run -- <命令>`**。
- **`bws run` 注入机制**：`bws run -- 'npm run start'` 以 Access Token 认证拉取指定项目 secrets，**把 secret 名作为环境变量注入子进程**执行；选项含 `--project-id`、`--shell`、`--no-inherit-env`、`--uuids-as-keynames`（UUID 连字符转下划线前置下划线，规避 POSIX 变量名限制；非 POSIX 兼容的 secret 名仍会设置但仅部分程序可见）。
- **认证与残留**：状态文件默认 `~/.config/bws/state`，**加密存储认证令牌并跨运行复用**；被撤销的 Access Token 在存储会话过期前可能仍有效，需删除 state 文件或 `state_opt_out`。子进程退出后环境变量随之销毁，无明文落盘（state 为加密令牌，非 secret 本体）。
- **F8 对照结论：`bws run` 是 Bitwarden 体系内最接近 1Password `op run` 的原生实现**（子进程级注入、支持 `--no-inherit-env`），但其作用域是 **Secrets Manager 的机器密钥**，不是个人密码库条目——个人密码条目仍无 `bw run`。若自研产品要做 F8，`bws run` 是最佳参照实现（含 `--uuids-as-keynames` 这类 POSIX 兼容细节与 state 加密设计）。

### 3.4 F1–F10 对照（BSM，按其自身定位）

BSM 不做浏览器自动填充/密码管理：F1 ❌（非其职责）、F2 ✅（端到端加密，SDK 开源）、F3 ✅、F4 ⚠️（以 Access Token 而非主密码/密钥文件为信任根，token 可配 state 不落盘）、F5 ✅（bws 跨平台 + SDK）、F6 ⚠️（存在但 Enterprise/coming soon 口径）、F7 ✅（bws + SDK + CI Action）、**F8 ✅（`bws run`）**、F9 ⚠️（无密码库式批量导入，API/脚本批量建 secret）、F10 ⚠️（API 导出，无附件/归档概念）。

### 3.5 适用场景

CI/CD 与本地开发注入环境变量、服务间机器密钥分发。个人/小团队免费档（3 项目/3 机器账户）即可支撑自研工具的 F8 需求；注意自部署限制。

---

## 四、版本快照（截至 2026-10-25，均实测）

| 组件 | 最新版本 | 日期/来源 |
|---|---|---|
| 官方 Server | v2026.10.0 | 2026-10-14，GitHub releases（bitwarden/server） |
| 桌面/扩展（clients 仓库） | Desktop v2026.10.1 | 2026-10-22，GitHub releases（bitwarden/clients） |
| CLI（npm @bitwarden/cli） | 2026.10.0 | npm registry latest |
| 移动端 | bitwarden/mobile 仓库已归档，最终 release v2024.10.0；现移至 bitwarden/android、bitwarden/ios 独立仓库，商店版本未在本次逐一核实 | GitHub API + 组织页 |
| Vaultwarden | 1.37.4 | 2026-10-05，GitHub releases |

---

## 五、小结：官方自部署 vs Vaultwarden 的推荐边界

1. **要官方审计背书 + 可能商用/合规 → 官方自部署**：小内存设备用 Lite（200MB RAM，个人用途），正经营业环境用标准版（2GB RAM+，MSSQL）。付出的是资源与运维复杂度，换来"2025 年含 ETH Zurich 恶意服务器假设审计"的加密背书与官方安全响应（尽管 2025–2026 Server 侧访问控制 CVE 仍需跟版）。
2. **个人/homelab/极低资源 + 想免费拿 Premium 档体验 → Vaultwarden**：但必须接受其非官方、无审计、不对数据丢失负责的免责框架，并把"及时升级 + 定期备份 + fail2ban/HTTPS 加固"当作默认运维成本；其 CVE 密度（2024–2026 连续多轮授权类缺陷）表明这层风险是真实的。
3. **威胁模型为"厂商投毒"时的关键残余**：自部署解决的是**服务端**投毒；**客户端二进制仍来自官方渠道**，且已有 CVE-2026-42994（npm CLI 被投毒 1.5 小时）实证。任何 Bitwarden 系方案都应叠加：锁定 CLI 版本 + 订阅 GHSA、优先用本地代码客户端（扩展/桌面）而非自部署 Web Vault 页面、KDF 调至 Argon2id 高参数。
4. **F8 落位**：个人密码库的 `bw` 无注入原语（当前版连旧 `bw run` 都已移除）；环境变量注入请直接采用 **BSM `bws run`** 的模式，它是本生态内唯一与 `op run` 对标的原生实现，也是自研时最值得抄的机制设计。
