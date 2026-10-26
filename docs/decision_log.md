# 决策记录（Decision Records）

## DR-001 2026-10-25 SuperPasswdKey 选择自研路线 B [状态: 生效]
- 背景: 调研（docs/research/总报告_密码管理器选型.md，URL 均实测核实）显示无单一产品同时满足 F1–F10 全部核心功能；F8 临时环境变量注入、F9 散落密钥扫描导入、F10 加密归档导出为实测核实的行业空白。
- 选项与权衡: 路线A 直接使用现有软件组合（首选 KeePassXC+坚果云 WebDAV+gopass，零费用、L3 信任级、F2/F3/F4/F6 满分，但 F8 仅 gopass 部分覆盖、F9/F10 空白无法补齐，弃用）; 路线B 自研（可补齐三大空白并达成 L3 信任模型，代价 10–12 个月全职工期与单人维护风险）
- 结论: 路线 B 自研 SuperPasswdKey
- 理由: 用户威胁模型（厂商投毒/针对国内用户作恶）要求结构性可验证信任；F9 全链路市场空白是自研正当价值点
- 影响: docs/research/自研需求基线_SuperPasswdKey.md 为需求基线；docs/技术选型与排期_Rust栈.md 为执行排期
- 记录时间 / 会话: 2026-10-25 / SuperPasswdKey 调研会话

## DR-002 2026-10-25 核心语言选型 Rust [状态: 生效]
- 背景: 威胁模型红线 #5 要求"主密码与派生密钥可确定性清零"；CVE-2023-32784 根因即 .NET GC 字符串残留。
- 选项与权衡: Rust（无 GC、密钥可清零、单二进制、内核经 UniFFI 复用桌面/Android，代价学习曲线与开发速度，弃用理由仅此）; Go（开发快，但 GC 无法可靠清零密钥、红线 #5 打折，弃用）; TypeScript/Electron（GUI 最快，但内存卫生最差、可复现构建不可行、与 L3 信任目标冲突，弃用）
- 结论: Rust 全家桶（sp-core/sp-kdbx/sp-cli workspace + Tauri 2 + WebExtension + UniFFI）
- 理由: 红线 #5 完整达成是信任模型的硬约束，语言层只有 Rust 可满足
- 影响: docs/技术选型与排期_Rust栈.md §2
- 记录时间 / 会话: 2026-10-25 / SuperPasswdKey 调研会话

## DR-003 2026-10-25 数据格式 KDBX4 兼容 + MVP 采用 CLI 内核先行 [状态: 生效]
- 背景: 基线 §8.6 预留格式重大决策；P0 边界需确定首个可用版本形态。
- 选项与权衡: KDBX4 兼容（复用 KeePass 全平台客户端群与迁移出口，MVP 工期减半，代价格式演进受制 KeePass 规范 + keepass-rs 写路径实验性风险 R1，弃用理由仅此）; 自有格式（自由度最高，但 +2~4 个月且 MVP 不可行，弃用）。MVP 范围: CLI 内核先行（M1–M3 约 2.5–3.5 个月可用）; GUI 同步开发（首版延后至 4–5 个月，弃用）
- 结论: KDBX4 兼容；MVP = M1 加密内核+CLI → M2 F8 注入 → M3 F9/F10 导入导出
- 理由: KDBX 是唯一"附件/TOTP/passkey/自定义字段"全保留格式（分报告G 速查）；CLI 先行与用户最关注的 F8 直接对齐
- 影响: 里程碑出口标准见 docs/技术选型与排期_Rust栈.md §4；M1 头号风险 R1（round-trip 互操作测试）
- 记录时间 / 会话: 2026-10-25 / SuperPasswdKey 调研会话

## DR-004 2026-10-25 项目许可证选型 AGPL-3.0 [状态: 生效]
- 背景: 第 2 轮复审补齐的决策缺口；项目含服务端轨（M6）。
- 选项与权衡: AGPL-3.0（网络 copyleft 防止服务端被闭源 fork，对齐 Vaultwarden 同款选择，代价对商业集成不友好——本项目为个人/小团队定位影响小，弃用理由仅此）; GPL-3.0（对齐 Bitwarden/KeePassXC，但不约束网络服务端使用，弃用）; MIT/Apache（无 copyleft，与信任模型叙事冲突，弃用）
- 结论: AGPL-3.0
- 理由: 服务端轨若被闭源 fork 会破坏"可结构性证伪投毒"的生态承诺
- 影响: 全仓库 LICENSE；依赖兼容性已核（RustCrypto MIT/Apache、gitleaks 规则 MIT 内嵌保留署名）
- 记录时间 / 会话: 2026-10-25 / SuperPasswdKey 调研会话

## DR-005 2026-10-26 KDBX4 适配层选型 keepass-ng 0.11 [状态: 生效]
- 背景: H1 spike（R1 头号风险）。keepass-rs 的 KDBX4 写入为 experimental；活跃 fork keepass-ng（crates.io 0.11.12，MIT 许可，H1 实测核实）。
- 选项与权衡: keepass-ng 0.11（save_kdbx4 实测可用，round-trip 双向互操作 KeePassXC 2.8.0 全通过，弃用理由仅"实验性"标注）; keepass-rs（写入 experimental 同源旧版，弃用）; 按 KDBX4 规范自实现写路径（+3~4 周，仅在 keepass-ng 出现写入缺陷时启用——trait 抽象已预留，弃用）
- 结论: keepass_ng 适配层（crates/sp-kdbx），features = save_kdbx4 + totp
- 理由: 实测证据优于标签；round-trip 测试（crates/sp-kdbx/tests/roundtrip.rs）为互操作回归护栏
- 影响: 实测发现 3 个互操作坑并已文档化：①keyfile 必须官方 XML v1.0 格式（legacy raw-32B 语义分歧：keepass_ng 直接用字节、KeePassXC SHA-256(内容)）；②Windows keepassxc-cli 无管道密码能力（测试用 keyfile-only + --no-password）；③KeePassXC "otp" 属性仅接受 otpauth:// URL 或 KeeOtp 格式（seed?period= 不解析）。KDF 参数经 KdfConfig::Argon2id（memory 单位=字节）
- 记录时间 / 会话: 2026-10-26 / SuperPasswdKey 开发会话

## DR-006 2026-10-27 浏览器扩展 host 形态=长驻进程内会话缓存 + 协议加密 P1/适配器 [状态: 生效]
- 背景: 步骤 3（F1）native messaging host 选型。平台要求 host 常驻；v0.1 无跨进程缓存；KDF ~700ms。keepass_ng 0.11.12 源码核实无公开"复合密钥直开库"API（get_key_elements 为 pub(crate)），每请求重开库必含 KDF。
- 选项与权衡: A(长驻 host+凭据因子缓存+空闲超时清零+每请求重开库，明文条目不常驻、可读外部修改，代价单请求≈KDF+<50ms，选定); B(解密条目常驻内存，最快但明文驻留窗口最大、外部修改不可见，弃用); C(一次性命令复用，浏览器子进程无 TTY 无法交互解锁、传密码违红线 #5，与 native messaging 常驻模型冲突，弃用)。协议加密: P1(明文管道+allowed_origins 绑定+主密码一次传输即清，MVP 选定——对同用户内存注入威胁加密层无可防增益); P2(TweetNaCl box 照搬 KeePassXC，+2~3 天且引入 nonce/握手 bug 面，弃用理由=工期，预留适配器切换点)
- 结论: sp-nmhost 独立长驻二进制（方案 A）；会话=进程内缓存 CredentialFactors(SecVec)+空闲 300s 锁库；协议信封含 v/codec 字段，codec=plain(P1)/naclbox(P2 预留)，业务层不感知 codec；与 v0.2 backlog"会话缓存决策"合并收敛：进程内缓存由本 DR 定稿，跨进程持久化缓存（keyring/DPAPI）仍为 backlog
- 理由: 唯一在"红线 #5 主密码不出进程"与浏览器场景可用性间自洽；适配器策略按用户要求兼顾进度与后续升级 P2 的路径
- 影响: crates/sp-nmhost、extension/、native-host/ 注册脚本、design_docs/10/11/12、SRS V1.1（SP-F-7xx）、追溯矩阵；v0.2 backlog 项调整见 SRS §10
- 记录时间 / 会话: 2026-10-27 / SuperPasswdKey 步骤3 开发会话
