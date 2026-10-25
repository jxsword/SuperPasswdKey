# 分报告 F：主流密码管理器安全记录与"投毒/供应链"风险横向核实

> 调研子代理 T6 产出 | 核实截止日期：2026-10-25
> 威胁模型背景：用户关心两个具体风险——(1) 软件厂商后续故意针对国内用户插入恶意代码窃取主密码；(2) 某个发布版本开始被投毒。
> 核实方法：官方公告、NVD/CVE/GitHub Advisory API 实测查询、可靠安全媒体（Malwarebytes、Ars Technica、WithSecure、Wiz、Unit42、Sophos、Checkmarx 生态等）交叉验证。所有 URL 均标注核实状态；"已证实"= 官方/权威一手来源可直接确认；"社区传闻/二手"= 仅有媒体或社区转述。
> 说明：本文不包含任何真实凭据字面量。

---

## 1. LastPass 2021–2022 事件链（已证实，官方公告 + 学术分析）

### 1.1 事实时间线

| 时间 | 事件 | 来源 |
|---|---|---|
| 2021-08 | 攻击者进入 LastPass 开发者环境，窃取部分源代码与专有技术信息 | LastPass 2021-12-22《Notice of Security Incident》（blog.lastpass.com，详见 1.3 核实说明） |
| 2022-08-25 | LastPass 披露第二波：开发者环境再被入侵，源代码与技术信息被窃 | 同上系列公告；Wikipedia 条目归纳：https://en.wikipedia.org/wiki/2022_LastPass_data_breach |
| 2022-11-30 | 攻击者利用 2022-08 窃取的信息（含云备份加密密钥副本）进入 LastPass 的第三方云存储，**下载客户保险库备份** | LastPass 2022-12-22《Notice of Security Incident》：https://blog.lastpass.com/posts/notice-of-recent-security-incident （公告当时仍称"无证据表明保险库数据被未授权访问"） |
| 2023-03-01 | LastPass 正式承认：**攻击者已窃取加密保险库备份副本**，并披露加密范围缺陷 | LastPass《Security Incident Update and Recommended Actions》(2023-03-01)：https://blog.lastpass.com/posts/security-inccident-update-recommended-actions |
| 2025-02 | 学术界复盘攻击路径 | arXiv《Breaking the Vault — A Case Study of the 2022 LastPass Data Breach》：https://arxiv.org/abs/2502.04287 |

### 1.2 暴露的具体缺陷（官方承认 + 研究者分析）

1. **URL 明文存储（官方承认）**：2023-03-01 公告明确——保险库备份中"除 URL 外的敏感字段均被加密"，即每条密码条目对应的**网站 URL 未经加密**，攻击者可完整还原每个用户访问过哪些站点（含敏感内网/政府/金融站点）。
2. **PBKDF2 迭代次数不均（官方承认）**：2022-12 起新默认值提升到 600,000 次 PBKDF2-SHA256，但历史遗留账户迭代次数极低——大量用户仅 5,000 次，**部分用户低至 1 次**（第三方归纳：https://russharvey.bc.ca/resources/lastpass.html ；LastPass 官方 update 公告承认存在低迭代账户）。迭代次数极低的账户主密码实际处于可低成本暴力破解状态。
3. **保险库 blob 加密结构缺陷（研究者分析）**：加密采用 AES-256-CBC、密钥派生自主密码，缺乏对密文的完整性保护（无 MAC），且迭代次数与完整性校验绑定的设计被研究者批评为"结构上不足以对抗离线爆破"；详见 Wikipedia 条目（对多份研究者分析如 Wladimir Palant 评述的归纳）与 arXiv 2502.04287。**注意：此条属研究者共识而非 LastPass 官方定性，标记为"研究者分析"。**

### 1.3 后果评估（已证实）

- 大批企业与个人迁移至其他密码管理器；LastPass 母公司 GoTo 公告披露事件连带影响其其余产品线。
- 该事件成为"零知识架构口号不等于加密实现质量"的标志性案例：官方最初 2022-12-22 公告声称"无证据表明加密保险库数据被访问"，三个月后被 2023-03-01 公告推翻——**厂商公告的"无证据"表述不可作为安全结论**。
- 对本项目的直接启示：主密码 KDF 必须对所有用户强制同一高参数（无遗留降级路径）；密文必须带完整性保护；元数据（URL、标题）也要纳入加密范围。

**1.3a URL 核实说明**：blog.lastpass.com 两个公告 URL 经搜索引擎缓存与多方（arXiv、BSAFE 归档、Wikipedia）交叉引用确证存在且内容一致；本机直接访问被其 CDN 反爬拦截（HTTP 403），核实日期 2026-10-25，判定为可达性受反爬影响而非链接失效。

---

## 2. KeePass

### 2.1 CVE-2023-32784：主密码内存泄露（已证实，NVD API 实测）

- **NVD API 实测返回原文（核实日期 2026-10-25）**：
  - 影响范围：**KeePass 2.x < 2.54**；
  - 后果：即使工作区已锁定或 KeePass 已退出，仍可从内存转储恢复**明文主密码**；转储来源包括进程转储、pagefile.sys、hiberfil.sys、整机 RAM dump；
  - 限制：**首位字符无法恢复**；
  - 修复：2.54 改用不同 API 调用并插入随机字符串缓解；
  - CVSS 3.1：7.5（HIGH），发布于 2023-05-15。
  - NVD URL：https://nvd.nist.gov/vuln/detail/CVE-2023-32784
- PoC：https://github.com/vdohney/keepass-password-dumper （2023-05 公开，搜索结果确认）。
- 分析文章：Sysdig 检测指南 https://www.sysdig.com/blog/keepass-cve-2023-32784-detection （2023-06-13）；Sophos《Don't panic》https://www.sophos.com/en-us/blog/serious-security-that-keepass-master-password-crack-and-what-we-can-learn-from-it 。
- 定性：**前提是攻击者已能读到你的内存/页面文件**（本地执行权限或拿到休眠文件），不扩大网络暴露面；但对"拿到硬盘镜像即可尝试破解"的威胁模型是实打实的削弱。
- 当前最新版本（核实 2026-10-25，官方新闻页）：**KeePass 2.61.1**（2.58：2025-03-04，2.59：2025-07-09，2.60：2025-11-05，2.61：2026-03-06；来源 https://keepass.info/news/n250304_2.58.html 、https://keepass.info/download.html ）。

### 2.2 假冒站点与恶意 KDBX 分发（已证实，多家独立安全厂商）

**事件 A：2023-10 Punycode 假冒官网（Google 广告投毒）**
- 事实：Google 搜索 "keepass" 置顶广告指向 Punycode 同形异义域（渲染后外观与 keepass.info 几乎一致，形如 `хxеepass` 类西里尔混排域），广告还带 Google "已验证"绿标；站点分发木马化安装器（DLL 劫持加载器投放 FakeBat/SectopRAT），并有版本诱导用户**上传自己的 KDBX 数据库文件**。
- 来源（核实 2026-10-25）：
  - Malwarebytes（2023-10-18）：https://www.malwarebytes.com/blog/threat-intel/2023/10/clever-malvertising-attack-uses-punycode-to-look-like-legitimate-website
  - Ars Technica（2023-10-18）：https://arstechnica.com/security/2023/10/google-hosted-malvertising-leads-to-fake-keepass-site-that-looks-genuine
  - BleepingComputer（2023-10-19）：https://www.bleepingcomputer.com/news/security/fake-keepass-site-uses-google-ads-and-punycode-to-push-malware
  - 海德堡大学 URZ 警示（德国内网教育网受害集中）：https://www.urz.uni-heidelberg.de/en/newsroom/manipulated-keepass-version-in-circulation

**事件 B：2025 KeeLoader 假安装器（已证实，WithSecure 首报）**
- 事实：WithSecure W-Labs 2025-05-08 披露，假冒 KeePass 安装器（KeeLoader）通过 malvertising 与 typo 域分发，**携带受信任 CA 签名的有效数字证书**，活动持续约 8 个月；功能包括从 KDBX 与浏览器提取凭据、内存加载 Cobalt Strike、最终投放 Qilin 勒索软件（含 ESXi 环境）。
- 来源：https://www.withsecure.com/en/resources-hub/w-labs/keepass-trojanised-in-advanced-malware-campaign （2025-05-08）；Symantec/Broadcom 与 ThaiCERT 2025-05-21~22 均发告警（搜索结果确认）。
- 注意：首报方为 WithSecure 而非 Proofpoint（后者未发布 KeeLoader 专题，网传归属有误——已核实修正）。

### 2.3 keepass.info 域名信任问题

- **已证实**：keepass.info 本体从未被入侵；风险全部来自**搜索广告与同形域**。KeePass 官方安装器由作者 Dominik Reichl 个人签名（单维护者模式），签名可信但代码审计依赖开源社区与个人维护。
- 结论：KeePass 生态的最大攻击面是**分发渠道**（搜索广告、第三方镜像）而非官方渠道本身；这与本报告第 9 节"版本投毒"威胁模型完全同构。

---

## 3. Bitwarden

### 3.1 浏览器扩展 CVE（NVD 核实）

| CVE | 内容 | 状态 |
|---|---|---|
| CVE-2018-25081 | 跨域 IFRAME 自动填充：≤2023.2.1 的扩展在"页面加载时自动填充"开启时会向不可信 iframe 填充凭据；Flashpoint 披露，2023-03 公开。CVE 编号中的"2018"是**预留年份**，非实际披露年份——常见误解。来源：CVE Record https://www.cve.org/CVERecord?id=CVE-2018-25081 ；Flashpoint https://flashpoint.io/blog/bitwarden-password-pilfering | 已证实 |
| CVE-2023-27974 | 二级域（SLD）匹配自动填充：example.com 存的密码可能被填充到同 SLD 的其他子域/站点。NVD：https://nvd.nist.gov/vuln/detail/CVE-2023-27974 （2023-03-09 发布，影响 ≤2023.2.1） | 已证实（NVD 摘要确认） |
| CVE-2023-24044 | **不是 Bitwarden**。NVD API 实测（2026-10-25）：为 **Plesk Obsidian ≤18.0.49** 的 Host 头注入问题，且被标记 disputed | 已证伪——网传"Bitwarden CVE-2023-24044"系张冠李戴 |

两均为自动填充策略缺陷而非加密缺陷，且默认设置（用户点击触发的自动填充、iframe 黑名单）已大幅缓解（Bitwarden 官方说明 https://bitwarden.com/help/auto-fill-browser ，搜索确认）。

### 3.2 官方审计历史（已证实）

- **2018 Cure53 审计**：2018-11 完成，11 项发现（多为轻微），报告原件：https://cure53.de/pentest-report_bitwarden.pdf （2026-10-25 实测 HTTP 200 可下载）。Bitwarden 官方公告：https://bitwarden.com/help/is-bitwarden-audited （列明 2018 及后续年度审计，含 2019/2020/2022/2023 年度第三方审计与 Insight Risk 参与，来源页搜索确认）。
- Bitwarden 另持 SOC 2 等合规认证（官方合规页确认）。

### 3.3 CVE-2026-42994：npm 渠道 bw CLI 供应链投毒（已证实，GitHub Advisory API 实测）

- **GitHub Advisory API 实测（GHSA-999x-2wm3-jfx8，核实 2026-10-25）原文要点**：
  - **Bitwarden CLI 2026.4.0，在 2026-04-22T21:57Z 至 23:30Z 之间从 npm 获取时内嵌恶意代码**——投毒窗口约 **1.5 小时**；
  - 与 Checkmarx 供应链事件相关（攻击者疑似滥用 Bitwarden CI/CD 中的 GitHub Action）；
  - CVSS 3.1 **9.8**（CWE-78/94：OS 命令注入、代码注入）；NVD 发布 2026-05-01；
  - Advisory URL：https://github.com/advisories/GHSA-999x-2wm3-jfx8 ；NVD：https://nvd.nist.gov/vuln/detail/CVE-2026-42994 ；
  - Bitwarden 官方声明帖：https://community.bitwarden.com/t/bitwarden-statement-on-checkmarx-supply-chain-incident/96127 （URL 被官方 advisory 引用；本机直访被反爬 403，判定可达性受反爬影响）；
  - Sophos 双事件分析（Checkmarx 与 Bitwarden 开发者工具同日中招）：https://www.sophos.com/en-us/blog/supply-chain-attacks-hit-checkmarx-and-bitwarden-developer-tools
- 关联背景：该事件与 **Shai-Hulud npm 蠕虫**生态相关联（见第 5 节）；社区修复指引为升级到 @bitwarden/cli ≥ 2026.4.1 并轮换可能泄露的令牌（r/sysadmin 等，社区级别）。
- **缓解结论（对本项目最重要）**：npm 渠道被投毒约 1.5 小时期间拉取的 CLI 是恶意的；而**从 bitwarden.com 官方下载渠道获取的桌面客户端不受影响**——同一公司、同一代码库、两条分发渠道风险不同。包管理器渠道（npm/PyPI）是"版本投毒"最现实的入口。

---

## 4. 1Password

### 4.1 Okta 供应链事件波及（已证实）

- 时间线：2023-09-29 1Password 检测到其 Okta 实例（管理员工应用）中的可疑活动，处置并上报；2023-10-23 公开披露（在 BeyondTrust、Cloudflare 相继披露 Okta 客服系统失陷后）。
- 细节：一名员工的 Mac 笔记本疑似被入侵（曾在不安全的酒店 Wi-Fi 上执行身份提供方超管操作；IT 管理员凭据疑似经 DevOps 账户中的明文暴露）。**仅波及员工侧应用，无用户保险库数据受影响**；Okta 根因（被盗服务账号访问客服工单）见 Okta 2023-11-03 RCA。
- 来源：1Password 官方 https://1password.com/blog/okta-incident ；Okta RCA https://sec.okta.com/articles/2023/11/unauthorized-access-oktas-support-case-management-system-root-cause ；Cybersecurity Dive https://www.cybersecuritydive.com/news/1password-okta-breach/697636 （核实 2026-10-25）。
- 定性：**SaaS 型密码管理器厂商自身的"员工身份/CI 体系"同样是供应链一环**；1Password 处置得当但暴露面客观存在。

### 4.2 闭源对"投毒风险评估"意味着什么

- 1Password 客户端核心闭源，**不存在可复现构建**，第三方无法验证官方二进制与任何公开代码一致；社区无法在厂商针对特定区域（如国内用户）插入恶意代码时自行发现。
- 其安全承诺只能依赖：官方分发渠道完整性、独立审计、漏洞赏金与厂商声誉激励。这意味着**闭源 SaaS 的"投毒"风险无法被用户侧技术手段排除，只能被"厂商作恶成本"抑制**——对本威胁模型（假设厂商可能主动作恶）而言是结构性劣势。

### 4.3 漏洞赏金（Bugcrowd 已过时，须核实修正）

- **已证实的准确事实**：2016 年 AgileBits 在 Bugcrowd 开设公开赏金（https://bugcrowd.com/agilebits ）；**2022 年起项目迁至 HackerOne**（https://hackerone.com/1password ，2022 年另设 100 万美元 CTF 赏金挑战 https://hackerone.com/1password_ctf ）。核实 2026-10-25。网传"现役 Bugcrowd 项目"已过时。

### 4.4 "2023 年 Secret Key 128→256-bit 升级"是否属实——不属实

- **核实结果：1Password 的 AES-256 迁移发生在 2013 年 3 月**（官方博客《Why we moved to 256-bit AES keys》：https://1password.com/blog/why-we-moved-to-256-bit-aes-keys ，2026-10-25 实测 HTTP 200），系保险库**数据加密算法**从 128 位升级到 AES-256。
- 官方支持文档（https://support.1password.com/secret-key-security ）明确 **Secret Key 本身即按 128 位熵设计**，从未升级到 256 位。网传"2023 年 128→256 升级"**未找到任何官方来源**，判定为以讹传讹（年份与对象双重错误）。

---

## 5. 供应链投毒横向：开发者生态中的密码窃取

### 5.1 npm/PyPI 密码窃取包（已证实案例）

- **PyPI W4SP Stealer 变种（2023-07）**：Palo Alto Unit42 披露 6 个恶意 PyPI 包窃取凭据与加密钱包数据。来源：https://unit42.paloaltonetworks.com/malicious-packages-in-pypi （2023-07-11）。
- **Shai-Hulud npm 蠕虫（2025-09 / 2025-11 两波）**：
  - 第一波（2025-09-16 Wiz 首报）：自我传播蠕虫窃取 npm 令牌，感染数百个包；https://www.wiz.io/blog/shai-hulud-npm-supply-chain-attack ；Unit42（2025-09-18）确认被窃密据被推到公开 GitHub 仓库（描述 "Sha1-Hulud: The Second Coming"）：https://unit42.paloaltonetworks.com/npm-supply-chain-attack
  - 第二波（2025-11，StepSecurity 2025-11-23）：Zapier、ENS Domains 等知名包被感染；https://www.stepsecurity.io/blog/sha1-hulud-the-second-coming-zapier-ens-domains-and-other-prominent-npm-packages-compromised ；Elastic 应对指南（2025-12-01）：https://www.elastic.co/blog/shai-hulud-worm-2-0-updated-response
- 与第 3.3 节 CVE-2026-42994 合并看：**针对开发者的凭据窃取已从"假包 typosquatting"进化为"自传播蠕虫 + 真实包投毒"**，且密码管理器 CLI 本身已成被投毒对象。

### 5.2 GitHub Actions / CI 投毒对"本地密钥管理"的启示（已证实路径）

- 攻击链模式（Shai-Hulud + Checkmarx/Bitwarden 事件归纳）：窃取维护者 npm 令牌 → 发布带毒版本 → 在受害者 CI（GitHub Actions）中用 TruffleHog 类扫描器翻找云凭据 → 回写公开仓库 → 继续传播。Bitwarden 官方 CI 的 GitHub Action 即疑似按此路径被滥用（Sophos 分析）。
- 启示：**任何"主密码/密钥曾进入联网构建环境"的设计都默认已被扫描**。本地密钥管理工具必须保证主密码不经过任何 CI/云进程（详见第 10 节红线）。

---

## 6. Vaultwarden：非官方自托管服务的安全责任分析

### 6.1 定位与官方立场（已证实）

- Vaultwarden 自我声明即为 **"Unofficial Bitwarden compatible server written in Rust"**（仓库 https://github.com/dani-garcia/vaultwarden ），各分发渠道明确"not associated with the Bitwarden project nor 8bit Solutions LLC"。
- Bitwarden 官方的审计（Cure53 等）、SOC 2、安全白皮书**均不覆盖** Vaultwarden；不存在官方背书。Bitwarden 安全白皮书：https://bitwarden.com/help/bitwarden-security-white-paper 。

### 6.2 2024–2026 授权类 CVE 核实（NVD/GitHub Advisory/第三方数据库）

| CVE/GHSA | 问题 | 修复 |
|---|---|---|
| CVE-2025-24364 | 远程代码执行（RCE） | 2025 年内修复（数据库：https://www.sentinelone.com/vulnerability-database/cve-2025-24364 ） |
| GHSA-c5rv-q295-7w4g | 绕过登录暴力破解保护（启用邮箱 2FA 场景尤为相关） | 1.35.4（官方公告：https://github.com/dani-garcia/vaultwarden/security/advisories/GHSA-c5rv-q295-7w4g ） |
| CVE-2026-27801 | 2FA 绕过 | 见数据库页 https://www.sentinelone.com/vulnerability-database/cve-2026-27801 |
| CVE-2026-43912 | **跨组织授权绕过**：一个组织的管理员可越权访问另一组织 | 汇总：https://zeropath.com/blog/cve-2026-43912-vaultwarden-cross-org-authorization-bypass |
| SSO email_verified 缺陷 | SSO 登录仅对新用户校验 `email_verified`，存在账号接管路径 | 1.36.0（OpenCVE 汇总：https://app.opencve.io/cve/?product=vaultwarden&vendor=dani-garcia ） |

- 当前最新版本（GitHub API 实测，核实 2026-10-25）：**Vaultwarden 1.37.4（2026-10-05 发布）**。
- CVE-2025-24364 与 CVE-2026-27801 的具体细节本文以数据库页为准，均属"授权/会话"类缺陷集群——**独立重实现版本的服务端授权逻辑质量与官方实现不可同日而语**。

### 6.3 自部署与"厂商投毒"风险的权衡（分析）

- **降低的风险**：Vaultwarden 是 Rust 重实现、社区维护，用户可自编译，Bitwarden 官方服务端代码变更（含假设性的针对区域作恶）不影响自托管实例——**厂商作恶面被移除**。
- **引入的新风险**：(1) 信任从"Bitwarden 公司"转移为"单一志愿者维护者 + 社区"（代码不经过商业审计）；(2) 服务端授权类 CVE 历史显示重实现质量短板；(3) 自托管者承担运维/升级责任，补丁滞后即风险窗口；(4) 不可复现构建时，Docker Hub 上的第三方镜像又是一条投毒链。
- 结论：Vaultwarden 换掉了"厂商投毒"问题，但换成了"维护者信任 + 运维质量"问题，并非免费收益。

---

## 7. 开源可复现构建现状（对"版本投毒检测"的意义）

| 产品 | 可复现构建状态 | 核实依据 |
|---|---|---|
| KeePassXC | **有官方承诺**：2.6.0（2020-07）changelog 明确 "Make builds reproducible [#4411]"；但跨平台发布二进制的端到端逐字节复现仍有未决问题（openSUSE 2024-08 状态页记录部分非确定性来源）；另于 2025-10 通过 Synacktiv/ANSSI CSPN 评估 | https://keepassxc.org/blog/2020-07-07-2.6.0-released ；https://lists.opensuse.org/archives/list/factory@lists.opensuse.org/thread/2XMBAI6G4ZFAS7LFWM2XPYHZYMG5C2GF ；https://keepassxc.org/assets/pdf/Synacktiv-ANSSI-CSPN-KeePassXC-RTE-v1.3.pdf |
| Bitwarden | **未见官方可复现构建计划**（截至核实日，官方文档与发布渠道无相关承诺；其安全故事依赖审计 + 渠道） | 反向核实：bitwarden.com/help 系列页无 reproducible 条目（搜索确认） |
| 1Password | **无（闭源，结构性不可能）** | 第 4.2 节 |
| KeePass 2.x | 无官方可复现构建；依赖作者个人 Authenticode 签名 | 官方下载页 https://keepass.info/download.html |
| Vaultwarden | 开源可自编译，但官方发布的容器镜像无逐字节复现承诺 | 仓库 + 第 6 节 |

- **对版本投毒检测的意义**：可复现构建是**唯一让"这个二进制 = 这份源码"可被第三方独立验证**的机制；没有它，开源密码管理器发布渠道上的安装包与源码之间的等价性仍要靠签名链（信任打包者）。KeePassXC 是主流开源密码管理器中唯一走通这条路（含 CSPN 评估）的；这也应成为自研项目的核心设计目标（第 10 节）。

---

## 8. 量子计算远期威胁专节

### 8.1 对称加密（AES-256）与 KDF 的量子余量（已证实，权威来源）

- **Grover 算法效应**：对密钥穷举提供二次加速，等效把密钥强度减半——AES-128 → ~64 位量子安全，**AES-256 → ~128 位量子安全**（~2^128 次量子运算）。来源：NIST PQC 常设会议论文《On the Practical Cost of Grover for AES Key Recovery》（https://csrc.nist.gov/csrc/media/Events/2024/fifth-pqc-standardization-conference/documents/papers/on-practical-cost-of-grover.pdf ）；PQShield 解释文 https://pqshield.com/explainer-articles/aes-quantum-resistant 。
- **NIST 官方立场**：PQC FAQ 明确"现有应用可继续使用 128/192/256 位 AES"，且"AES-192 与 AES-256 将在很长时间内保持安全"：https://csrc.nist.gov/projects/post-quantum-cryptography/faqs 。
- **工程现实**：上述 NIST 会议论文指出 Grover 的实际代价远超理论上限（需要海量纠错量子比特与串行查询），2^128 量子攻击在可预见未来不可行。**结论：AES-256 保险库加密无量子焦虑；需要关注的是主密码熵**——若主密码人类可记（低熵），KDF 穷举的量子加速才是真实短板，且 Argon2id 的内存困难性在量子攻击模型下仍显著抬高并行穷举成本（对 KDF 的 Grover 加速无法绕过内存带宽瓶颈，此为学界共识，见 NIST FAQ 及 PQShield 综合）。

### 8.2 密码管理器中的 PQC 采用现状（核实 2026-10-25）

- **主流密码管理器尚未全面采用 PQC**：保险库对称层（AES-256）本就量子余量充足；真正需要 PQC 替换的是**分享/密钥交换等非对称组件**（RSA/ECC 会被 Shor 算法击穿）。Bitwarden 官方社区帖对此的定位一致（https://community.bitwarden.com/t/quantum-resistant-encryption/58534 ，社区讨论）。
- 生态动向：Proton Mail 2026-05 起为 Proton 用户间邮件提供抗量子加密，但 **Proton Pass 截至核实日未上线 PQC**（用户社区在请求：https://www.reddit.com/r/ProtonPass/comments/1uvm3u8/proton_pass_needs_postquantum_cryptography ；Proton PQC 术语页：https://proton.me/learn/encryption/glossary/post-quantum-cryptography ）。云基础设施侧 ML-KEM 混合 TLS 已默认化（AWS 2026-04），但密码管理器产品层面明显滞后。
- 对自研的启示：**非对称部分（未来若做分享/同步密钥协商）应直接选 ML-KEM 混合方案，而非等生态**；对称层与 KDF 保持 AES-256 + Argon2id 即已达量子余量上限。

---

## 9. 三级信任模型对比（每产品一节的投毒风险评估）

| 信任级别 | 产品示例 | 厂商针对特定区域作恶（投毒）可否被发现 | 单版本被投毒的检测手段 | 残余风险 |
|---|---|---|---|---|
| **L1 闭源 SaaS/闭源官方分发** | 1Password | **不能**（无代码、无可复现构建，只能信审计与声誉） | 无用户侧手段；仅厂商自证 + 渠道 + 赏金 | 作恶不可检测（LastPass 2022-12 公告翻车示范了"只能信公告"的脆弱性） |
| **L2 开源 + 官方编译分发** | Bitwarden（官方渠道）、KeePass 2.x | 源码可审，但**发布二进制 ≠ 源码**无法逐字节证明；真被投毒只能事后靠赏金/研究者发现 | 签名校验（仅证明"是官方发的"）、Cure53 类审计、npm 类渠道事故显示包渠道尤其脆弱（CVE-2026-42994 实例） | "源码干净但构建被动手脚"在技术上无法排除（无 reproducible builds 时） |
| **L3 纯本地开源 + 可复现构建** | KeePassXC（有承诺 + CSPN）、自研可达到 | **可被任何第三方验证**：任何人用公开源码自建，与官方发布逐字节比对 | 可复现构建比对 + 独立签名 + 分散分发 | 仍是目前唯一可"结构性证伪"厂商投毒的模型；残余风险为依赖（编译工具链）与用户是否真的执行比对 |

- 横向结论：**用户威胁模型是"厂商故意投毒"时，L1 不可选、L2 只能缓解、L3 才是可验证解**。历史上真实发生的投毒（KeePass 假冒站、bw CLI npm 投毒、Shai-Hulud）全部发生在**分发渠道**而非官方源码——所以验证链条必须覆盖"从源码到我运行的二进制"全程。

---

## 10. 自研 SuperPasswdKey 安全设计要求清单（基于以上事实）

1. **版本签名与校验**：发布物（安装包/二进制）必须带可离线验证的签名（Minisign/ed25519 或 sigstore），公钥预置在多个独立位置（文档、源码仓库、公证渠道）；校验失败拒绝运行。
2. **可复现构建为第一优先级目标**：锁定工具链版本、剔除时间戳/路径等非确定源，发布时公开复现指引（参照 KeePassXC #4411 做法）；每次发布至少邀请第三方复现并公示比对结果。做不到逐字节复现前，至少提供"源码 → 构建 → 哈希"的公开验证流程。
3. **分发渠道纪律**：官方渠道之外（尤其是 npm/PyPI 等包管理器、搜索广告投放）一律不作为获取途径；吸取 CVE-2026-42994（npm 渠道 1.5 小时投毒）与 KeePass 假冒站教训；不做可能被劫持为投放点的竞价广告。
4. **无遥测**：主程序零网络上报（崩溃报告须显式开关且内容可审）；同步流量目标为用户提供的中立存储（如自选网盘），元数据（URL/标题）一律纳入加密（LastPass 教训）。
5. **主密码永不出本地进程（架构红线）**：KDF 与加解密全部本地完成；不落盘明文、不进入任何 CI/云进程（第 5.2 节：凡进过构建环境的密钥默认视为泄露）；对内存中的主密码与派生密钥做主动清理并缓解内存残留（吸取 CVE-2023-32784 教训：不同 API 路径 + 随机串污染）。
6. **KDF 全员强制高参数、无降级路径**：Argon2id（内存 ≥ 64 MiB 起步、随硬件上调）或 PBKDF2 ≥ 600,000（兼容场景），禁止"遗留账户低迭代"模式（LastPass 教训）；加密采用 AEAD（XChaCha20-Poly1305 或 AES-256-GCM），杜绝无完整性保护模式。
7. **开源可审计**：全部代码开源（含构建脚本），接受第三方审计；任何"区域定制/条件编译"必须公开并在可复现构建中可检验——这是对抗"针对国内用户插入恶意代码"的可验证防线。
8. **量子余量**：对称层 AES-256/XChaCha20 + Argon2id 已满足量子余量（NIST FAQ）；若未来引入分享/密钥协商，直接采用 ML-KEM 混合方案。
9. **供应链卫生**：依赖锁定 + 定期审计（对照 Shai-Hulud 案例：依赖传递投毒）、CI 使用最小权限、签名密钥离线保管、构建环境与密钥物理隔离。
10. **事件披露纪律**：发现投毒/漏洞时按 LastPass 反例的教训执行——一次性说清泄露范围与加密实现缺陷，不用"无证据"话术拖延。

---

## 附：主要来源核实状态一览（核实日期均为 2026-10-25）

| URL | 状态 |
|---|---|
| https://nvd.nist.gov/vuln/detail/CVE-2023-32784 | NVD API 实测（全文摘录见 2.1） |
| https://api.github.com/advisories/GHSA-999x-2wm3-jfx8 | GitHub Advisory API 实测（全文摘录见 3.3） |
| https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=CVE-2023-24044 | API 实测，确认为 Plesk |
| https://api.github.com/repos/dani-garcia/vaultwarden/releases/latest | API 实测：1.37.4 (2026-10-05) |
| https://api.github.com/repos/keepassxreboot/keepassxc/releases/latest | API 实测：2.8.0 (2026-10-21) |
| https://api.github.com/repos/bitwarden/clients/releases/latest | API 实测：desktop-v2026.10.1 (2026-10-22) |
| https://cure53.de/pentest-report_bitwarden.pdf | HTTP 200 实测 |
| https://1password.com/blog/why-we-moved-to-256-bit-aes-keys | HTTP 200 实测 |
| https://blog.lastpass.com/posts/notice-of-recent-security-incident | 存在性经多方引用确证；直访被反爬 403 |
| https://blog.lastpass.com/posts/security-inccident-update-recommended-actions | 同上 |
| https://community.bitwarden.com/t/bitwarden-statement-on-checkmarx-supply-chain-incident/96127 | 被官方 advisory 引用；直访 403 |
| https://www.withsecure.com/en/resources-hub/w-labs/keepass-trojanised-in-advanced-malware-campaign | 搜索引擎确认存在（WithSecure 官方） |
| https://www.malwarebytes.com/blog/threat-intel/2023/10/clever-malvertising-attack-uses-punycode-to-look-like-legitimate-website | 搜索引擎确认存在 |
| https://arstechnica.com/security/2023/10/google-hosted-malvertising-leads-to-fake-keepass-site-that-looks-genuine | 搜索引擎确认存在 |
| https://unit42.paloaltonetworks.com/malicious-packages-in-pypi | 搜索引擎确认存在（2023-07-11） |
| https://www.wiz.io/blog/shai-hulud-npm-supply-chain-attack | 搜索引擎确认存在（2025-09-16） |
| https://www.stepsecurity.io/blog/sha1-hulud-the-second-coming-zapier-ens-domains-and-other-prominent-npm-packages-compromised | 搜索引擎确认存在（2025-11-23） |
| https://csrc.nist.gov/projects/post-quantum-cryptography/faqs | 搜索引擎确认存在（NIST 官方） |
| https://csrc.nist.gov/csrc/media/Events/2024/fifth-pqc-standardization-conference/documents/papers/on-practical-cost-of-grover.pdf | 搜索引擎确认存在（NIST 会议论文） |
| https://keepassxc.org/blog/2020-07-07-2.6.0-released | 搜索引擎确认存在 |
| https://1password.com/blog/okta-incident | 搜索引擎确认存在（1Password 官方） |
| https://sec.okta.com/articles/2023/11/unauthorized-access-oktas-support-case-management-system-root-cause | 搜索引擎确认存在（Okta 官方 RCA） |
| https://hackerone.com/1password | 搜索引擎确认存在 |
| https://github.com/dani-garcia/vaultwarden | GitHub 实测（release API 同源） |
| https://keepass.info/download.html | 搜索引擎确认存在（官方下载页，最新 2.61.1） |
| https://www.sophos.com/en-us/blog/supply-chain-attacks-hit-checkmarx-and-bitwarden-developer-tools | 搜索引擎确认存在 |

**已证伪/需修正的网传说法汇总**：
1. "Bitwarden 存在 CVE-2023-24044" —— 实为 Plesk（NVD API 实测证伪）。
2. "1Password 2023 年将 Secret Key 从 128 位升级到 256 位" —— 256 位 AES 迁移在 2013 年，Secret Key 至今为 128 位熵设计（官方博客与支持文档证伪）。
3. "1Password 漏洞赏金在 Bugcrowd" —— 2022 年已迁 HackerOne。
4. "KeeLoader 由 Proofpoint 披露" —— 首报为 WithSecure（2025-05-08）。
