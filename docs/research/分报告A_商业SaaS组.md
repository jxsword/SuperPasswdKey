# 分报告 A：商业 SaaS 密码管理器调研（1Password / Proton Pass / Dashlane / LastPass / Enpass）

- 调研日期：2026-10-25
- 调研目的：以「1Password 收费全功能」为基准，评估商业 SaaS 密码管理器对本项目 F1–F10 核心功能（尤其 F8 CLI 临时环境变量注入）的参考价值
- 功能编号：F1 自动保存/填充；F2 本地加解密；F3 云端只存密文；F4 密钥文件本地+主密码不上云；F5 跨平台(Win/Linux/Android 必选, macOS/iOS 备选)；F6 私有部署；F7 CLI；F8 CLI 临时环境变量注入；F9 批量导入；F10 批量导出
- URL 可达性核实方式：WebFetch / curl（HTTP 状态码）/ 外部网页抓取服务。除特别注明外，「可达」均指 2026-10-25 实测
- 安全约束：本文所有示例命令一律使用占位符（如 `<你的服务令牌>`），不包含任何真实凭据

---

## 1. 1Password（基准产品）

### 1.1 URL 三件套
| 项 | URL | 核实结果 |
|---|---|---|
| 官网 | https://1password.com/ | 可达（2026-10-25，HTTP 200；另有中文页 https://1password.com/zh-cn 实测 200） |
| 官方文档 | https://support.1password.com/ ；开发者/CLI 文档 https://www.1password.dev/ | 可达（2026-10-25；support 域个别页面如 /security-design/ 返回 403 反爬，/security-assessments/ 正常 200） |
| 源码 | 无（产品本体闭源；CLI 亦闭源） | — |

安全设计白皮书：https://1password.com/files/1Password-White-Paper.pdf 现已 301 重定向至在线版 https://agilebits.github.io/security-design/ （可达，2026-10-25；当前版本 Release 0.5.2，2026-03-05 更新）。

### 1.2 简介
AgileBits 出品的老牌商业密码管理器，2026 年已转型为「Unified Access」统一访问平台（密码管理 + 凭证代理 + 特权访问 + SaaS 管理），营销口号 "Secure access for every human and AI agent"。个人/家庭产品线仍在正常销售。

### 1.3 收费（来源：https://1password.com/personal ，获取日期 2026-10-25）
| 档位 | 价格（美元） | 说明 |
|---|---|---|
| Individual 个人版 | $3.99/月（年付促销 $2.99/月，首年另有 25% 折扣） | 1 人，全设备 |
| Families 家庭版 | $5.99/月（年付促销 $4.49/月） | 最多 5 人，每人独立保险库，无限共享保险库 |
| Business/Enterprise | 按报价（联系销售，AWS Marketplace 有售） | — |

14 天免费试用，无需信用卡；官方支付方式为信用卡/ACH，**不支持支付宝/微信**。

### 1.4 加密架构（来源：在线白皮书 agilebits.github.io/security-design，Release 0.5.2）
- **Secret Key 机制**：注册时本地生成约 128-bit 的高熵 Secret Key，与用户 Account Password 组合后经慢哈希（白皮书专章 "Slow hashing"，PBKDF2）派生密钥。Secret Key 保存在本地/应急包（Emergency Kit PDF），**不上传服务器**——这是其对抗服务端泄露暴力破解的核心设计。
- **本地加解密、服务端只存密文**：保险库条目在客户端用 AES-256-GCM 加解密；服务端仅存储密文（白皮书 "What the server stores" 专章）。
- 账户密钥体系分层（账户密钥 → 保险库密钥 → 条目密钥），支持安全共享。

### 1.5 最新版本（截至 2026-10-25）
- 桌面/浏览器端：1Password 8 **8.12.42**（2026-10-20，来源 releases.1password.com）
- CLI：**2.43.0**（来源 app-updates.agilebits.com/product_history/CLI2，实测抓取）

### 1.6 安全审计历史（来源：support.1password.com/security-assessments，2026-10-25 可达）
- Independent Security Evaluators (ISE)：2020-04～06 渗透测试 + 代码审查
- Onica：AWS 基础设施安全架构评估
- 2025-11 起年度渗透测试报告统一经 1Password Trust Center 发布
- 2024-12 起 HackerOne 公开漏洞赏金；ISO 27001/27017/27018/27701、SOC 2 Type II

### 1.7 F1–F10 对照
| 功能 | 支持 | 说明 |
|---|---|---|
| F1 自动保存/填充 | ✅ | 全平台浏览器扩展 + 系统级填充，业界最成熟 |
| F2 本地加解密 | ✅ | 客户端 AES-256-GCM，白皮书完整披露 |
| F3 云端只存密文 | ✅ | 零知识架构，服务端仅存密文 |
| F4 密钥文件本地+主密码不上云 | ⚠️ | Secret Key 本地保存（Emergency Kit）+ 主密码不上云，但 Secret Key 不可自定义「密钥文件」，绑定账号且云端可辅助找回 |
| F5 跨平台 | ✅ | Win/macOS/Linux 原生应用 + iOS/Android + 浏览器扩展，Linux 为一等公民 |
| F6 私有部署 | ❌ | 纯 SaaS；1Password Connect 仅是 secrets API 网关，保险库仍托管在 1Password 云 |
| F7 CLI | ✅ | op CLI 2.x，功能最全 |
| F8 CLI 临时环境变量注入 | ✅ | `op run` / `op inject` / `op plugin run`，本组标杆，见第 6 节 |
| F9 批量导入 | ✅/⚠️ | 应用内导入器支持 LastPass/Bitwarden/CSV/1PUX 等十余种来源（2026-10-25 官方文档核实）；但 CLI 无 import 子命令，环境变量扫描导入无官方方案 |
| F10 批量导出 | ⚠️ | 1PUX 全量导出（保留更多字段）+ CSV（仅登录类字段）；官方明确 **1PUX/CSV 导出均为明文、不支持设置密码加密**（2026-10-25 官方导出文档核实）；移动端走 Credential Exchange 标准；附件/文档导出支持未见官方说明 |

### 1.8 优缺点与适用场景
- 优点：加密架构文档化程度最高；CLI/Secrets Automation 生态（Connect、Service Account、SDK、Shell Plugins、Environments）是全行业最完整；Linux 支持好；审计与合规齐全。
- 缺点：纯订阅制、无买断；不支持私有部署；导出无加密选项；国内访问与支付不便（见 1.9）。
- 适用场景：个人开发者与中小团队的全功能默认选择；作为本项目「收费全功能」基准完全成立。

### 1.9 1Password 国内版现状（核实：2026-10-25）
- **无官方国内版**：仅有面向全球的统一版本；官网提供 zh-cn 中文页面（实测 200，定价仍以美元计价），无中国本地化服务器、无国内合规主体。
- 访问：官网与 App Store 中国区可下载应用，一般可直接访问。
- 支付：官方仅支持信用卡/ACH（美国银行账户），不支持支付宝/微信；国内用户需 Visa/Mastercard 等国际卡。

### 1.10 特色专有功能
Secret Key 双因子密钥设计；1Password Environments（云端 .env 管理）；Shell Plugins（80+ 第三方 CLI 生物识别认证，含 AWS/GitHub/Cloudflare 等）；Service Account + Connect Server 密钥自动化；Watchtower 安全巡检；通行密钥与 SSH agent 集成。

---

## 2. Proton Pass

### 2.1 URL 三件套
| 项 | URL | 核实结果 |
|---|---|---|
| 官网 | https://proton.me/pass | 经外部抓取服务核实可达（2026-10-25）；**本机直连失败**：DNS 解析 proton.me 报 EAI_AGAIN，curl 退出码 6 |
| 官方文档 | https://proton.me/support/pass | 经外部抓取服务核实可达（2026-10-25）；本机同样 DNS 不通 |
| 源码 | https://github.com/protonpass （android-pass、ios-pass、pass-cli、proton-pass-common 等） | 可达（2026-10-25，curl 200；pass-cli 最新 release **2.5.1**，2026-10-21 发布，GPL-3.0，Rust 编写，426 stars） |

加密方案文档：官方博客 "The Proton Pass security model" https://proton.me/blog/proton-pass-security-model （经外部抓取服务核实可达，2026-10-25）。

### 2.2 简介
Proton（Proton Mail/VPN 同门）2023 年推出的密码管理器，发布时即同步开源客户端并通过 Cure53 审计；主打隐私全家桶集成（SimpleLogin 邮箱别名、Proton Sentinel、2FA 集成 authenticator）。

### 2.3 收费（来源：https://proton.me/pass/pricing ，获取日期 2026-10-25）
| 档位 | 价格（美元） | 说明 |
|---|---|---|
| Free | $0 | 无限密码保存、任意设备、2 个保险库、隐藏邮箱别名 10 个、支持安全共享 |
| Pass Plus | $2.99/月（年付 $35.88） | 无限隐藏邮箱、无限保险库/共享、集成 2FA、1 GB 文档存储、**支持 CLI 等进阶功能** |
| Pass Family | $4.99/月（年付 $59.88） | 6 用户，各自 Plus 功能 |
| Proton Unlimited | $9.99/月 | 全家桶：Pass Plus + Mail Plus + VPN + Drive 500 GB |

### 2.4 加密架构（来源：官方 security model 博文，2026-10-25 核实）
- **分层信封加密**：每个条目生成 32-byte 随机 Item Key 加密条目内容；Item Key 再被 32-byte 随机 Vault Key 加密；Vault Key 由用户密钥体系保护——改密/换保险库无需重加密全部数据。
- 算法：对称加密 **AES-256-GCM**；KDF 为 **bcrypt**（客户端由主密码派生）；服务端认证用 **SRP**（Secure Remote Password，服务端不掌握明文密码）。
- 全字段端到端加密（含标题、URL、备注），服务器只见密文；客户端开源（GPL 等自由软件许可）可复核。

### 2.5 最新版本（截至 2026-10-25）
- CLI pass-cli：**2.5.1**（2026-10-21，GitHub API 实测）；移动/桌面客户端闭源部分已开源（android-pass / ios-pass 仓库活跃），具体 App 版本号未在本次抓取中固定，不作断言。

### 2.6 安全审计历史
- **Cure53**：2023-07 对浏览器扩展、移动应用与 API 渗透测试，报告公开 https://cure53.de/pentest-report_proton-pass.pdf （与客户端开源同批发布）；
- 后续另有 Recurity Labs 对全部 Pass 应用审计（Proton 社区页面提及，报告经 Trust 渠道发布）；
- 集团层面 ISO 27001 等认证沿用 Proton 体系。

### 2.7 F1–F10 对照
| 功能 | 支持 | 说明 |
|---|---|---|
| F1 自动保存/填充 | ✅ | 浏览器扩展 + 移动端填充，成熟度略逊 1Password |
| F2 本地加解密 | ✅ | 客户端 AES-256-GCM + bcrypt，客户端开源可复核 |
| F3 云端只存密文 | ✅ | 全字段 E2EE，SRP 认证不传密码 |
| F4 密钥文件本地+主密码不上云 | ❌ | 无独立密钥文件机制（主密码即 Proton 账户密码，不上云这点满足） |
| F5 跨平台 | ✅ | Win/macOS/Linux + iOS/Android + 扩展；Linux 桌面端相对较新 |
| F6 私有部署 | ❌ | 无自托管；开源的是客户端，服务端仍由 Proton 托管 |
| F7 CLI | ✅ | pass-cli（GPL-3.0，开源）， vault/item 管理齐全 |
| F8 CLI 临时环境变量注入 | ✅/⚠️ | 支持 `pass://vault/item/field` 引用，可注入环境变量或模板文件，并有官方 GitHub Action（load-secret-action）与 Ansible 插件；机制与 op run 同思路，但生态较新、文档成熟度不及 1Password |
| F9 批量导入 | ⚠️ | 应用内 CSV/主流管理器导入；无 CLI import 命令、无环境变量扫描 |
| F10 批量导出 | ⚠️ | CLI 支持导出（CSV/JSON）；加密归档导出、附件全量导出未见官方说明 |

### 2.8 优缺点与适用场景
- 优点：客户端全开源 + 公开审计报告；价格低于 1Password；免费档最慷慨；CLI 开源可审计，本项目可参考其 `pass://` 引用与环境变量注入实现。
- 缺点：无密钥文件机制；无私有部署；Proton 账户体系与主密码合一（换主密码=换账户密码）；生态较新。
- 适用场景：重视开源与审计、想要 Proton 隐私全家桶的个人/家庭用户。

### 2.9 特色专有功能
SimpleLogin 邮箱别名深度集成；Pass Monitor（暗网/弱密码监控）；内置 2FA 验证器（与 Proton Authenticator 打通）；pass-cli SSH agent 模式；官方 GitHub Actions/Ansible 密钥注入组件。

---

## 3. Dashlane

### 3.1 URL 三件套
| 项 | URL | 核实结果 |
|---|---|---|
| 官网 | https://www.dashlane.com/ | 经外部抓取服务核实可达（2026-10-25）；**本机直连失败**：DNS 解析 www.dashlane.com 报 EAI_AGAIN（curl/WebFetch 均不通） |
| 官方文档 | https://support.dashlane.com/ | **未能核实**：本机 DNS 不通（同上）；CLI 文档 https://cli.dashlane.com/ 亦无法从本机访问 |
| 源码 | 无（产品闭源；CLI 开源 https://github.com/Dashlane/dashlane-cli ） | CLI 仓库经 GitHub API 核实（2026-10-25）：Apache-2.0，230 stars，最近推送 2026-10-22，最新 release **v6.2642.0**（2026-10-13） |

安全白皮书：Dashlane "Security Principles & Architecture" **v2.0.3（2023-05-16）**，本次经英国 G-Cloud 供应商托管的镜像 PDF 下载并全文提取（https://assets.applytosupply.digitalmarketplace.service.gov.uk/g-cloud-14/documents/703763/825196895322183-service-definition-document-2024-04-30-1159.pdf ，2026-10-25 可达）。

### 3.2 简介
老牌商业密码管理器，特色是内建 VPN（现拆分为 Plus 档）与暗网监控；近年重心转向企业（Business/Enterprise），个人产品线仍在维护。

### 3.3 收费
官方 pricing 页为动态渲染，本机与外部抓取均只得到 Business 档结构与 "--" 占位价格（2026-10-25 实测），个人档官方数字**未能直接核实**；第三方评测（2025–2026 年多方一致）：Free（限 25 条密码/1 设备）、Premium **$4.99/月**、Friends & Family **$7.49/月**（最多 10 人）。此组数字标注为「第三方来源，官网未直接核实」。

### 3.4 加密架构（来源：官方白皮书 v2.0.3 全文提取，2026-10-25）
- 数据加密：所有个人数据在客户端用 **AES-256** 加密；对称密钥由 UserMP（用户主密码）+ UserSecondaryKey（32-byte 随机，本地生成）共同派生。
- KDF：**Argon2d（或 PBKDF2-SHA2）**派生 AES 密钥，抗暴力破解；历史上公开口径为 PBKDF2-SHA256 600,000 次迭代（后迁移 Argon2）。
- 主密码永不联网传输；服务端认证使用独立的 DeviceKey（40 字节：8 字节 access key + 32 字节 secret key，本地加密保存），认证密钥与数据加密密钥分离。
- 共享：RSA-2048（OpenSSL 生成）+ 每条目一次性 ObjectKey（AES-CBC + HMAC-SHA2）。
- SSO 方案（自托管/托管/机密 SSO）在白皮书第 2 章有完整密钥流说明。
- 暗网监控：Argon2d v1.3（3 次迭代、32 MiB、并行 2、32 字节输出）+ K-anonymity（仅上传哈希前 3 字节）。

### 3.5 最新版本（截至 2026-10-25）
CLI v6.2642.0（2026-10-13，GitHub API 实测）；主应用版本号未能从本机核实（官网 DNS 不通），不作断言。

### 3.6 安全审计历史
无公开的完整审计报告列表页（Trust Center 需登录）；第三方汇总称其委托过 ISE 等机构并持有 SOC 2 Type II、ISO 27001 认证；学界有 MIT 6.857 课程分析（2016）。白皮书本身公开可查是主要依据。**标注：官方公开审计报告不如 1Password/Proton Pass 充分。**

### 3.7 F1–F10 对照
| 功能 | 支持 | 说明 |
|---|---|---|
| F1 自动保存/填充 | ✅ | 浏览器扩展体验好 |
| F2 本地加解密 | ✅ | 客户端 AES-256 + Argon2d，白皮书公开 |
| F3 云端只存密文 | ✅ | 零知识 + DeviceKey 认证分离 |
| F4 密钥文件本地+主密码不上云 | ❌ | 无密钥文件；UserSecondaryKey 由服务端参与分发 |
| F5 跨平台 | ⚠️ | Win/macOS 原生 + iOS/Android；**Linux 无桌面应用**（仅网页版 + CLI） |
| F6 私有部署 | ❌ | 纯 SaaS |
| F7 CLI | ✅ | dashlane-cli 开源（Apache-2.0），活跃维护 |
| F8 CLI 临时环境变量注入 | ⚠️ | CLI 可脚本化取密钥，但截至核查日未见官方等价 `op run` 的环境变量注入包装命令（cli.dashlane.com 本机不可达，未能逐条核对命令表） |
| F9 批量导入 | ✅/⚠️ | 应用内支持 CSV 与主流管理器导入；无环境变量扫描 |
| F10 批量导出 | ⚠️ | 导出为 CSV/JSON 明文；无加密归档选项 |

### 3.8 优缺点与适用场景
- 优点：白皮书技术披露详实（Argon2、密钥分离、SSO 密钥流）；CLI 开源；内建 VPN/暗网监控。
- 缺点：无 Linux 桌面端；无私有部署；个人档价格偏高且官方页面动态渲染透明度一般；审计公开度一般。
- 适用场景：Windows/macOS 为主、看重暗网监控与 SSO 集成的个人/企业；对本项目参考价值主要是其白皮书的 KDF 与密钥分离论述。

### 3.9 特色专有功能
内建 VPN；Master Password 暗网监控（K-anonymity）；企业 Confidential SSO；本地 JSON 加密备份机制（CLI 文档描述私钥本地存储方案）。

---

## 4. LastPass（反面教材）

### 4.1 URL 三件套
| 项 | URL | 核实结果 |
|---|---|---|
| 官网 | https://www.lastpass.com/ | 可达（2026-10-25，curl 200） |
| 官方文档 | https://support.lastpass.com/ | 可达（2026-10-25，curl 200） |
| 源码 | 无（产品闭源；官方 CLI 开源 https://github.com/lastpass/lastpass-cli ） | CLI 仓库经 GitHub API 核实（2026-10-25）：GPL-2.0，2911 stars，最近推送 2026-10-19 |

### 4.2 收费（来源：https://www.lastpass.com/pricing ，获取日期 2026-10-25）
页面正文价格以动态占位符渲染（{LPPremium} 等），无法从 HTML 直接读取数字；页面 meta 描述给出 "Premium starts at $2.25/mo"（官方页面数据）。档位结构（官方页面核实）：Free（限一种设备类型）/ Premium（单用户全设备，1GB 存储）/ Plus（含 Windows 端 VPN）/ Families（6 个 Premium 账号）/ Teams（≤50 用户）/ Business / Business Max。

### 4.3 加密架构（结合官方公告与白皮书口径）
- 保险库 blob：客户端用 AES-256 加密后上传；解密密钥由主密码经 **PBKDF2** 派生（默认迭代次数长期为 **100,100**，远低于同期 OWASP 建议的 600,000；事后为部分账户提升至 150,000，新账户再提升至 600,000）。
- 关键缺陷：**blob 内网站 URL、条目标题等字段为明文/弱混淆**，加密粒度不均（2022-12 官方公告原文承认 "unencrypted data, such as website URLs"）。
- 架构为「服务端保管加密 blob」模式，密钥派生强度与用户主密码强绑定，无类似 1Password Secret Key 的第二因子。

### 4.4 2021–2022 事件链核实（重点）
官方公告核实情况：
- **2022-12-22 "Notice of Security Incident"**（https://blog.lastpass.com/2022/12/notice-of-recent-security-incident/ ，WebFetch 实测可达并全文提取，2026-10-25）：确认第三方云存储备份被窃，含客户账户元数据（公司名、用户名、账单地址、邮箱、电话、IP）与**加密保险库备份**；保险库内 URL 等字段为明文；AES-256 + PBKDF2 100,100 次；官方称遵循默认设置者"无需行动"。
- **2021-09-13 公告**（2021 年 8 月发现异常）：未经授权方经单个被攻破的开发者账号访问部分开发环境，拿走部分源代码与专有技术信息；官方称客户数据与保险库未受影响。原文页 https://blog.lastpass.com/2021/09/unauthorized-access-privately-stored-password-information-securely-encrypted/ **实测被 Akamai 拦截（curl 403，WebFetch 返回空），未能直接核实原文**；时间线依据 2022-12 官方公告的回溯表述与多方报道交叉印证。
- **2023-02/03 扩展事实清单**：披露攻击者以定制 RAT/键盘记录器感染一名资深 DevOps 工程师家用电脑、实施第二次入侵、迭代次数口径差异等细节。原帖（blog.lastpass.com 2023-02 security-incident-update-and-expanded-fact-sheet / 2023-03 updated-and-expanded-fact-sheet）**实测 403/404，未能直接核实**；仅 2022-12 公告中官方自述"页面顶部提示已有 2023 年 3 月更新版本"得到确认。
- **事件链总结**：2021 开发环境失窃（源代码+技术信息）→ 2022-08 再次入侵开发环境（源代码）→ 2022-11-30 第三方云存储异常 → 2022-12-22 确认保险库备份被复制（URL 明文、PBKDF2 100,100）→ 2023-02/03 承认攻击者已获得解密云存储所需员工凭据与密钥（定制恶意软件）。

### 4.5 架构缺陷分析（作为反面教材）
1. **混淆而非真正的字段级加密**：URL/标题明文，泄露即暴露用户上网画像。
2. **KDF 迭代次数过低且长期不升级**：100,100 次 PBKDF2 在 2022 年显著低于行业标准（1Password 早期即 100,000 但配合 Secret Key；Bitwarden 当时默认 100,001 后升 600,000），弱主密码用户面临现实可行的离线暴力破解。
3. **无第二因子密钥**：全靠主密码，主密码强度=保险库强度。
4. **开发环境与生产凭据隔离不足**：2021 年源代码失窃的信息被用于 2022 年针对员工的定向攻击并最终解密云存储卷——纵深防御失败。
5. **危机沟通失信**：多次「调查结束、无客户影响」的结论被后续披露推翻。

### 4.6 最新版本（截至 2026-10-25）
浏览器扩展/客户端具体版本号未能在本次核实中固定（官网页面动态渲染）；CLI lastpass-cli 仓库仍活跃（2026-10-19 有推送）。

### 4.7 安全审计历史
出事前有常规渗透测试宣称，但 2021–2023 事件链暴露架构与运维缺陷；此后母公司 GoTo 进行内部整改并提升迭代次数。**无公开的完整第三方审计报告，信任修复缓慢**——本身即为「审计缺位 + 架构缺陷 + 事件响应失当」的复合案例。

### 4.8 F1–F10 对照
| 功能 | 支持 | 说明 |
|---|---|---|
| F1 自动保存/填充 | ✅ | 扩展成熟 |
| F2 本地加解密 | ⚠️ | 客户端 AES-256 加密，但字段级加密不完整（URL/标题明文） |
| F3 云端只存密文 | ⚠️ | 形式上是密文，但 blob 内明文字段与低迭代 KDF 削弱实质 |
| F4 密钥文件本地+主密码不上云 | ❌ | 无密钥文件；主密码为唯一保护因子 |
| F5 跨平台 | ✅ | Win/macOS/Linux/Android/iOS 全覆盖 |
| F6 私有部署 | ❌ | 纯 SaaS |
| F7 CLI | ✅/⚠️ | lastpass-cli 开源（GPL-2.0），但功能陈旧、体验一般 |
| F8 CLI 临时环境变量注入 | ❌ | `lpass show` 仅可脚本化取值，无官方注入包装 |
| F9 批量导入 | ✅ | CSV 导入成熟（被 1Password 等竞品重点支持迁移） |
| F10 批量导出 | ⚠️ | CSV 导出；无加密归档 |

### 4.9 适用场景与结论
仅剩存量企业与「图省事」用户；对个人与安全敏感用户不推荐。对本项目的价值：**反面规范**——必须做字段级全加密、必须用强 KDF（≥600k PBKDF2 或 Argon2）并允许用户升级参数、必须提供不依赖服务端信任的第二因子（密钥文件）、开发/生产环境严格隔离。

---

## 5. Enpass

### 5.1 URL 三件套
| 项 | URL | 核实结果 |
|---|---|---|
| 官网 | https://www.enpass.io/ | 部分可达（2026-10-25）：curl 浏览器 UA 首页 403（反爬），/pricing/ 与 /release-notes/ 等子页 200；WebFetch 均被 403 拦截，定价数据经 curl 抓取 HTML 提取 |
| 官方文档 | https://support.enpass.io/ （301 重定向至 https://help.enpass.io/ ） | 可达（2026-10-25，HTTP 200；同步机制文档 https://help.enpass.io/personal/latest/all/vault-management 实测可达） |
| 源码 | 无（客户端闭源） | — |

### 5.2 简介
「本地优先」密码管理器：**没有任何 Enpass 自有服务器**，数据保存在本地加密数据库，同步完全走用户自己的云盘或局域网。与本项目「云盘同步」路线直接同构，是本组内唯一不依赖厂商云的产品。

### 5.3 收费（来源：https://www.enpass.io/pricing/ HTML 直提，获取日期 2026-10-25）
| 档位 | 价格（美元） | 说明 |
|---|---|---|
| Free | $0 | 最多 25 条、1 个保险库 |
| Premium | $1.99/月（按年付）；3 年方案 **$49.99**（付 2 年送 1 年）；**Lifetime 买断 $99.99**（一次性，"Pay once and get massive savings"） | 无限条目/保险库 |
| Family | $3.99/月 | 最多 6 人 |

**一次性买断是五者中唯一仍在售的永久授权**。

### 5.4 同步机制（来源：官方帮助文档 vault-management 页，2026-10-25 可达）
- 保险库 = 一个**加密数据库文件**；官方明确："Enpass 在你的所有设备间同步你的 vault，而不会向 Enpass 服务器发送任何数据"。
- 三种同步方式：① 用户自选云账户在线同步，支持 **Dropbox、Google Drive、OneDrive、iCloud、Box、WebDAV、Nextcloud**；② **Wi-Fi 同步**（完全离线，仅经本地 Wi-Fi）；③ **Folder Sync**（同步到本地/挂载文件夹）。
- 云盘上存放的是整库加密文件（密文），即使云盘被攻破，攻击者仍需面对主密码派生密钥（官方称 AES-256 级加密）。
- 首库为 Primary vault（其主密码解锁全部库），其余库各有独立密码与存储位置。
- 对本项目意义：**「本地加密数据库 + 用户自选云盘同步密文文件」是已被商业产品验证 10 年的可行路线**，免服务器、免账号体系；代价是冲突合并、实时共享、网页端等能力缺失。

### 5.5 最新版本（截至 2026-10-25）
桌面版处于 6.11–6.12 区间（Windows 官网版持续更新；macOS Beta 6.12.7 于 2026-09-22 发布——来源 enpass.io/release-notes 与搜索结果）；未固定精确稳定版号，不作断言。

### 5.6 安全审计历史
历史上委托过第三方审计（早年曾公布过外部安全评估结论），但当前官网无完整公开审计报告列表；闭源 + 审计公开度有限是其主要短板。标注：**未能核实近年独立审计**。

### 5.7 F1–F10 对照
| 功能 | 支持 | 说明 |
|---|---|---|
| F1 自动保存/填充 | ✅ | 浏览器扩展 + 移动端填充 |
| F2 本地加解密 | ✅ | 本地加密数据库（官方称 AES-256 级加密），主密码永不离开设备 |
| F3 云端只存密文 | ✅ | 云盘上只有整库加密文件；且云盘由用户自选 |
| F4 密钥文件本地+主密码不上云 | ⚠️ | 主密码不上云满足；无独立密钥文件机制 |
| F5 跨平台 | ✅ | Win/macOS/Linux/Android/iOS 全原生支持，Linux 支持好 |
| F6 私有部署 | ✅ | 以「无服务器」形态天然满足：数据全本地/自选云盘/局域网 Wi-Fi 同步 |
| F7 CLI | ❌ | 无官方 CLI |
| F8 CLI 临时环境变量注入 | ❌ | 无此能力 |
| F9 批量导入 | ⚠️ | 文件导入（CSV/JSON，十余种来源）；无脚本/环境变量扫描导入 |
| F10 批量导出 | ⚠️ | CSV/JSON 明文导出；本地加密备份（.enpassbak 类）；无加密归档导出 |

### 5.8 优缺点与适用场景
- 优点：唯一无服务器依赖；一次性买断；全平台含 Linux；数据主权完全在用户。
- 缺点：闭源；无 CLI/注入能力；无网页端与实时协作；同步冲突处理依赖云盘文件语义。
- 适用场景：隐私敏感、多设备个人用户，尤其是希望「数据只在本地+自己云盘」的人；**对本项目 F2/F3/F6 + 云盘同步路线是最直接的商业先例**。

### 5.9 特色专有功能
Wi-Fi 局域网离线同步；Folder Sync；多保险库独立密码；一次性买断授权。

---

## 6. 专节：1Password CLI（op）机制——F8 的参考规范

以下内容基于 2026-10-25 实测抓取的官方文档：命令参考 https://www.1password.dev/cli/reference 、环境变量指南 https://www.1password.dev/cli/secrets-environment-variables 、Shell Plugins https://www.1password.dev/cli/shell-plugins.md 等。CLI 当前版本 2.43.0（app-updates.agilebits.com 实测）。命令清单（官方参考页核实）：account、completion、connect、document、environment、events-api、group、inject、item、plugin、read、run、service-account、signin、signout、update、user、vault、whoami。**注意：无 `op import` / `op export` 子命令**。

### 6.1 Secret Reference（op:// URI）
- 语法：`op://<vault>/<item>/<section>/<field>`（段可省略），可用名称或 ID，例：`op://development/GitHub/credentials/personal_token`。
- 支持查询参数：`?attribute=otp`（取 TOTP）、`?ssh-format=openssh`。
- 支持变量替换实现多环境切换：`op://$APP_ENV/mysql/password`，运行时以 `APP_ENV=prod` 切换保险库。

### 6.2 `op run`——临时环境变量注入（F8 核心）
机制：
- 解析 `.env` 文件（内含 `KEY=op://...` 引用）和/或 `--environment <ID>` 指定的云端 1Password Environments，将 secret reference 替换为真实值，**以环境变量形式注入子进程**：`op run --env-file ./prod.env -- <command>`。
- **生命周期**：真实密钥只存在于该子进程存活期间（"for the duration of the process"）；`op run` 是父进程，命令在子进程中运行，**进程退出后子进程环境随之销毁，不污染父 shell，不落盘**。`.env` 文件中保存的只是 `op://` 引用（URI），不是密文值，因此可以安全入版本库。
- **输出遮蔽（masking）**：默认对子进程 stdout/stderr 中出现的密钥值做遮蔽替换；`--no-masking` 关闭（调试用）。
- **官方风险提示**：同一操作系统用户运行的进程可以互相读取环境变量——注入式方案无法防御本机同权限进程，这是此类机制的固有边界（文档原文警告 "you should assume that the processes on your computer can access the environment of other processes"）。
- Shell 展开陷阱：`op run -- echo "$MY_VAR"` 中变量会被外层 shell 先展开成 URI；正确写法是 `op run -- sh -c 'echo "$MY_VAR"'` 让替换发生在子 shell 内。
- `.env` 解析规则（官方文档详列）：UTF-8；`#` 注释；引号剥离/保留规则；`$VAR`/`${VAR}` 展开；`\$` 转义；文件内变量互引；多行值引号包裹；空值→空字符串。

### 6.3 `op inject`——模板文件注入
- 将模板中 `{{ op://vault/item/field }}` 替换为真实值；支持拼接（如 `postgres://{{ op://lcl/db/user }}:{{ op://lcl/db/pw }}@host/db`）。
- `op inject -i config.yml.tpl -o config.yml`（`--file-mode` 默认 0600）；输出含明文密钥，官方明确提示"不再需要时删除已解析的文件"。适合生成一次性配置文件，不如 `op run` 无残留。

### 6.4 `op read`——单值读取
- `op read op://app-prod/db/password` 输出到 stdout；`--out-file` 写文件（默认权限 0600）；`-n` 去尾部换行。
- 典型用法：`docker login -u "$(op read op://prod/docker/username)" -p "$(op read op://prod/docker/password)"`（值经 shell 命令替换进入子进程，同样不落盘）。

### 6.5 `op plugin run`——Shell Plugins（第三方 CLI 凭据代理）
- 原理：第三方 CLI（AWS、GitHub、Cloudflare、kubectl、Stripe 等 80+，含 Claude Code/Cursor 等 AI 工具）的凭据保存在 1Password 保险库中；`op plugin run -- aws s3 ls` 时插件拦截认证流程，经**本机生物识别/系统认证**授权后，把凭据以该 CLI 期待的方式（环境变量或凭据文件句柄）临时提供给子进程，**不在磁盘留明文凭据**。
- 支持 bash/zsh/fish；可自建插件（官方提供构建指南与测试流程）。
- 这是 F8 的进阶形态：不仅注入变量，而是接管目标 CLI 的认证本身。

### 6.6 认证与令牌（Secrets Automation）
- **桌面应用集成**：CLI 可经 1Password 桌面 App + 生物识别认证，无需手工粘贴 token（默认推荐路径）。
- **Service Account**：不绑定个人的自动化身份（上限 100 个/账户）；token 需保存到本地配置（官方文档指导存入安全位置）；按 vault 粒度最小授权；有速率限制与 usage report。CLI 侧以环境变量指向服务令牌使用，例如：
  ```bash
  export OP_SERVICE_ACCOUNT_TOKEN="<你的服务令牌>"
  op read op://dev/mysql/password
  ```
  （令牌字面量仅存于本地受保护位置，文档要求不得入库。）
- **Connect Server**：自托管 REST 网关（仅 secrets API，非全量保险库托管），配合 `op connect` 与 Connect token；适合企业内网 CI。
- 缓存/会话：CLI 2.x 默认依赖桌面 App 会话；独立使用时经 signin 获得会话（环境变量方式），官方文档未宣传持久化明文缓存。`op environment read`/`op run --environment` 的网络请求次数在 2.42.0-beta 中从 6 次优化到 4 次（releases.1password.com，2026-10-25 核实），说明 Environments 是实时拉取而非本地缓存密钥值。

### 6.7 item 管理与导入导出
- `op item`（get/create/update/delete/list/edit/template/share 等）+ `op document`、`op vault`、`op user`、`op group` 覆盖日常管理；`op item create --template` 支持模板化批量创建（脚本批量导入的官方路径）。
- **无 `op import`/`op export`**：批量导入靠桌面应用导入器（LastPass/Bitwarden/CSV/1PUX 等，2026-10-25 官方文档核实，iOS 26+/Android 14+ 另支持 Credential Exchange 标准）；导出靠桌面应用 1PUX/CSV（明文，见 1.7）。

### 6.8 对 F8 的设计结论（供自研参考）
1. 引用与值分离：配置文件只存 URI（`scheme://vault/item/field`），值仅在运行时解析。
2. 子进程包裹器模式：外层进程解析引用→注入子进程环境→子进程退出即销毁；父 shell 永不获得值。
3. 默认输出遮蔽 + 显式关闭开关。
4. 明确文档化威胁边界（同用户本机进程可读环境变量）。
5. 机器身份最小授权（Service Account 按库授权、可审计、可吊销）。
6. 提供 `inject` 模板模式作为补充，但对产物文件给出 0600 权限与「用后即删」规范。

---

## 7. 本组小结

### 7.1 横向对照（截至 2026-10-25）
| 维度 | 1Password | Proton Pass | Dashlane | LastPass | Enpass |
|---|---|---|---|---|---|
| 个人价（美元/月） | $3.99（促销 $2.99） | $2.99 | ~$4.99（第三方口径） | ~$2.25 起 | $1.99 或 $99.99 买断 |
| 开源 | 无 | 客户端全开源 | 仅 CLI | 仅 CLI | 无 |
| 公开审计 | ISE/Onica + Trust Center | Cure53(2023) 公开报告 | 白皮书+认证，报告少 | 出事前缺位 | 有限 |
| 密钥文件/第二因子 | Secret Key ⚠️ | ❌ | ❌ | ❌ | ❌ |
| 私有部署 | ❌ | ❌ | ❌ | ❌ | ✅（无服务器形态） |
| CLI | ✅ 最强 | ✅ 开源 | ✅ 开源 | ⚠️ | ❌ |
| F8 环境变量注入 | ✅ 标杆 | ✅ 同思路较新 | ⚠️ | ❌ | ❌ |
| Linux | ✅ | ✅ | ❌(桌面) | ✅ | ✅ |
| 加密导出 | ❌（1PUX 明文） | ⚠️ | ⚠️ | ⚠️ | ⚠️ |

### 7.2 关键结论
1. **F8 的唯一成熟范本是 1Password `op run`**（§6）；Proton Pass pass-cli 证明了该模式可以以开源实现复刻，是自研时更可读的代码参考。
2. **F4（密钥文件）全行业仅 1Password 以 Secret Key 形态近似实现**，且不可自定义文件——自研提供独立密钥文件将是差异化点。
3. **F6/F3+云盘同步路线的先例是 Enpass**：本地加密库 + 用户云盘同步密文，10 年验证可行；其缺失面（无 CLI、无冲突处理、闭源）正是自研可以补足的方向。
4. **导出加密（F10）是全行业空白**：没有任何一家提供官方加密归档导出（1PUX 明文、CSV 明文）——自研实现「加密导出+可选明文」即可超越全部基准产品。
5. **LastPass 是架构反例规范**：字段级全加密、强 KDF、第二因子、开发/生产隔离四条底线缺一不可。
6. 国内可用性：五者均无国内版/国内主体；1Password 有中文页但仅信用卡支付；Enpass 买断制对国内用户支付门槛最低（第三方渠道）。

### 7.3 调研局限性声明
- Dashlane 个人档官方价格、LastPass 具体档位价格因官网动态渲染未能取得官方数字（已注明第三方口径）；blog.lastpass.com 2021-09 与 2023-02/03 三篇公告被 Akamai 拦截（403），未能直接核实原文，事件链经 2022-12-22 官方公告（已核实）与多方报道交叉印证。
- proton.me 与 dashlane.com 在本次执行环境存在 DNS 阻断（EAI_AGAIN），相关页面经外部抓取服务核实；1Password CLI 文档个别 URL 404 为文档改版所致，均以 2026-10-25 实测的现行 URL 为准。
