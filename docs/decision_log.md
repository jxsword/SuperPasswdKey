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

## DR-007 2026-10-27 发布签名选型 minisign 原生生成 + 四渠道公钥冗余 [状态: 生效]
- 背景: 红线 #1 分阶段承诺 v0.2 接入发布物公钥签名（08 §3：Minisign）。单人 Windows 项目、发布低频。
- 选项与权衡: 生成方式——minisign 原生(Ed25519+口令加密私钥，Windows 预编译可得，选定); OpenBSD signify(同格式系但 Windows 无官方二进制，弃用); 复用 SSH ed25519(签名密钥与高频 SSH 密钥耦合、攻击面大，弃用); YubiKey/HSM(minisign 不原生支持 PIV、单人项目成本收益比低，列为后续升级项)。分发渠道——仓库提交+Release 资产为基线（与代码同生共死、可被账号接管连带替换），独立 HTTPS 页(GitHub Pages)=唯一异源强冗余，Web Archive 快照=第三方时间锚，线下打印/U 盘=自我灾备，四路并用（比对矩阵见 2026-10-27 会话记录；URL 三件套均实测可达）。
- 结论: 离线主机 `minisign -G` 生成（强口令）；私钥离线保管（加密 U 盘+打印备份，不经 CI/代理/Agent）；公钥 `minisign.pub` 四渠道分发（仓库根提交、Release 资产、GitHub Pages 冻结指纹、Web Archive 快照）；签名动作=**本地脚本** `scripts/sign-release.ps1` 对 CI 产物 SHA256SUMS 签名后随 Release 发布（修正会话中"CI 内签名"表述：私钥不进 runner）；README 提供验签说明。降级声明：防公钥静默替换，不防账号与全部异源渠道被协同接管（超出单人模型，Release 说明如实声明）。
- 理由: 单验证者信任锚=维护者本人，策略取"多渠道冗余+首发冻结指纹"而非单点强保护；确定性签名与可复现构建叙事对齐。
- 影响: scripts/sign-release.ps1、README 验签节、Release 流程、channel A/C/D 待密钥生成后执行；后续升级项=HSM/Sigstore。
- 记录时间 / 会话: 2026-10-27 / SuperPasswdKey 步骤3 开发会话

## DR-008 2026-10-27 阶段1(v0.2.x 桌面增强)与阶段3(Android)资源分配：Spike 先行、时间盒分流 [状态: 生效]
- 背景: 步骤 3 完成后的路线图资源分配。Android 承载路线图最大未验证假设组（UniFFI 绑定/keepass_ng Android 可用性/移动端 KDF 参数/系统 Autofill 精度），其中内核复用触及 DR-002 选型承诺；桌面扩展刚发布处于 1–2 周保活观察期，不宜叠加变更。
- 选项与权衡: 先做阶段1(桌面增强确定性收益，但 Android 风险后置，发现内核不可复用时桌面投入沉没，弃用); 先做阶段3 全量(风险过早全押、spike 前无法评估，弃用); **Spike 先行时间盒 2–3 天再分流(选定)**——spike 产出结构性事实决定两阶段成本曲线，沉没成本可控，观察期与 spike 窗口天然重合。
- 结论: 阶段0收尾(Release 签名，DR-007) → Android Spike(时间盒 2–3 天，产出 design_docs/13 骨架+e2e 结论) → 按验收标准分流：UniFFI Kotlin 绑定编译通过+Android 模拟器 sp-kdbx 打开 KDBX4 e2e+64MiB Argon2id 目标设备 ≤3s 全过→阶段3(M1 系统 Autofill)为主线、阶段1 降穿插(≤20%)；任一核心受阻→回阶段1 主线(P2 加密+密钥复用开库)并启动内核复用评估(DR-005 回退路径)。任务书见 docs/任务书_step4.md。
- 理由: 信息价值不对称——spike 事实先行使后续资源分配基于实测而非猜测；桌面各项彼此独立可随时恢复，机会成本近零。
- 影响: docs/任务书_step4.md；后续 DR-009 起（spike 结论/移动端 KDF 参数/生物识别解锁均需独立 DR）
- 记录时间 / 会话: 2026-10-27 / SuperPasswdKey 步骤3 开发会话

## DR-009 2026-10-27 Android 转正：阶段3(M1 系统 Autofill)为主线，桌面阶段1 降穿插 [状态: 生效]
- 背景: Spike（PR #14）按 DR-008 三项全过——①UniFFI Kotlin 绑定编译通过（APK 构建成功）②模拟器 KDBX4 e2e 8 断言全绿 ③64MiB Argon2id 1045~1124ms、VmHWM delta≈68MiB（**非真机**）；aarch64 release 交叉编译通过。结构性发现：keepass_ng Database 为 !Send+!Sync，UniFFI 对象强制 Send+Sync → 线程封闭 actor 解法可行（design_docs/13 §2）。
- 选项与权衡: A(Android 主线+桌面穿插≤20%，遵循 DR-008 预设承诺、最大假设组已消解大半、环境与上下文热，劣=真机口径未验，选定); B(回桌面阶段1，确定性收益无真机依赖、工期短，但与 DR-008 预设相悖、Android 热度流失、风险后置，弃用); A 变体(穿插项固定为桌面阶段1，未选——保持穿插择一灵活性)
- 结论: 阶段3（M1 系统 Autofill）为主线；桌面阶段1（P2 加密+密钥复用开库）降穿插≤20%；SRS 升 V1.2 新增 SP-F-8xx；**真机验证列为 M1 前置任务**（SP-N-301/302 真机回填，需用户提供真机）。
- 理由: DR-008 分流规则是决策时已确认的承诺且 spike 客观全过；剩余风险属"可在 M1 内回收验证"而非结构性受阻（spike 使命即排除结构性失败）。
- 影响: SRS §2.3/§3.8/§8、追溯矩阵（SP-F-8xx）、design_docs/13 转正 v1.0；后续：FFI 面扩容、Autofill 设计（design_docs/14）、Android CI job；Keystore/生物识别仍单列 backlog（独立 DR）。
- 记录时间 / 会话: 2026-10-27 / SuperPasswdKey 步骤4 spike 会话

## DR-010 2026-10-27 移动端 KDF 默认参数：init 校准为终态，t=4/p=4 过渡 [状态: 生效]
- 背景: 真机回填（Redmi Turbo 3/骁龙 8s Gen 3，android_真机安装测试方案 §7）：t=8/p=4 解锁 2.2–2.5s，超出用户定稿的 ≤2s 解锁体验线；m=64MiB 为红线 #6 硬下限不动；Argon2id 参数随库文件存储，改默认仅影响新建库，旧库/互操作不受影响。
- 选项与权衡: A(维持 t=8/p=4，安全余量最大，但违背 ≤2s 线、低端机预计 5–10s，弃用); B(固定 t=4/p=4，一行常量、中端 ~1.1–1.3s 达标，但低端 2.5–5s 仍可能超线，弃用为终态、保留为过渡); C(init 校准 t∈[2,8]、m=64MiB/p=4 固定、目标 ~1s，选定——唯一同时满足中端与低端的方案)
- 结论: 终态=移动端 init 校准（复用 sp-core calibration.rs 策略移植）；过渡=M1b 期间 FFI 默认 t=4/p=4（仅改 sp-kffi::FfiKdfParams::default，不动桌面 sp-core 默认）；校准 t 下限 2（强度仍高于 OWASP 最低配置）。
- 理由: 解密时间目标校准是行业主流口径（KeePassXC 官方用户指南建议 ~1s 解密时间；桌面 SP-N-302 已有同策略，一致性最好）；低端机自适应是 A/B 均不具备的能力。
- 影响: sp-kffi 默认值（过渡）、M1b 校准移植任务、SP-N-302 真机口径闭环。参考依据（2026-10-27 实测可达）：Bitwarden 默认 32MiB/6 次/4 线程（bitwarden.com/help/kdf-algorithms，注明超出 OWASP 建议）；KeePassXC 用户指南"decryption time 滑标建议 1 秒"（keepassxc.org/docs）。
- 记录时间 / 会话: 2026-10-27 / SuperPasswdKey 步骤4 spike 会话

## DR-011 2026-10-28 移动端 KDF 终态切换落地：Android 首建默认走 calibrate_kdf [状态: 生效]
- 背景: DR-010 预留的终态（init 校准 t∈[2,8]/m=64MiB/p=4，目标 ~1s）在步骤 5 M1 正式化中接入壳层；过渡默认 t=4/p=4 完成使命。
- 选项与权衡: A(首建强制校准，失败即报错不静默兜底，选定——校准失败条件与建库失败同源，静默降级只会掩盖问题); B(校准失败静默回落 FfiKdfParams::default，弃用——用户无感知得到非校准库，违背校准初衷); C(维持过渡默认不再校准，弃用——低端机无自适应能力，DR-010 已否决)。
- 结论: VaultSession.createVault = calibrate_kdf(1000) → create_db(校准参数) → unlock；FfiKdfParams::default（t=4/p=4）降级为"跳过校准"兜底（fixture/测试口径）；校准参数随 KDBX4 头存储，无需另行持久化。
- 理由: 引用 DR-010（终态选项 C）、真机数据（Redmi Turbo 3：t=8/p=4 解锁 2.2–2.5s 超线；HyperOS 夜间降频 8.2–18.3s 方法论发现）、本任务模拟器实测（校准建链路 3.6–3.7s；快机 t 触底=2 符合预期；建库后 UI 回显 m=64MiB/t=2/p=4）。
- 影响: sp-kffi FfiKdfParams::default 注释与 e2e 测试语义（mobile_default_params_dr011_fallback）、android VaultSession.createVault、UnlockActivity 首建模式、TestActivity init_vault 断言；SP-N-302 移动端口径闭环。
- 记录时间 / 会话: 2026-10-28 / SuperPasswdKey 步骤5 M1 正式化会话

## DR-012 2026-10-29 Autofill 保存场景凭据来源：Rust actor 内保留凭据因子 [状态: 生效]
- 背景: M1 只做 Autofill 读路径；保存场景（onSaveRequest 新密码入库）调 save_atomic 必须提供凭据因子，而 VaultSession 按红线 #5 不存主密码——不解决此矛盾保存场景无法实现（任务书_step6 任务 1 设计闸门）。
- 选项与权衡: A1(sp-kffi actor 工作线程保留 CredentialFactors——Rust SecVec 可清零、保存零输入、与桌面 DR-006 方案 A 同威胁模型，选定); A2(Kotlin 侧缓存凭据——JVM String 不可零化产生额外不可清副本，弃用); B(保存时输入主密码——零扩大攻击面但每次弹框、与 Bitwarden/KeePassDX 惯例冲突、保存建议即时性丧失，弃用); C(Keystore 加密缓存——独立 DR 范围，列为 A1 的后续演进路径而非本期方案)
- 结论: 终态=A1：解锁时凭据因子随 open_db 留在 sp-kffi actor 工作线程（SecVec），save 复用之；显式锁库与 300s 空闲锁随 actor drop 清零。主密码内存驻留窗口由"FFI 调用栈内"扩大到"会话期"，在 design_docs/14 与 SP-F-804 审查口径如实声明。
- 理由: 红线 #5 字面要求"可确定性清零"完全满足（SecVec + 锁库即清）；本地内存注入者已越权的威胁模型下内存缓存不降低安全等级（DR-006 同款论证）；Kotlin 侧零新增机密持有；为 Keystore/生物识别独立 DR 留清晰升级路径（升级而非重构）。
- 影响: sp-kffi 增量（Vault 因子保留 + save 语义复用）、VaultSession/SpAutofillService 保存路径、design_docs/14 v1.2、SP-F-804 审查口径；实现于步骤 6 会话执行，本条为先于实现落盘的预决策。
- 记录时间 / 会话: 2026-10-29 / SuperPasswdKey 步骤 5.5 会话（CI e2e 转正 + step6 预备）

## DR-013 2026-10-29 凭据驻留策略：Android Keystore 包裹(B2+每boot主密码)，桌面双超时零豁免，两批次交付 [状态: 生效]
- 背景: 步骤6任务3决策材料出稿后用户拍板——Android: 每次设备重启后首次打开必须主密码，之后开启生物解锁则仅验证生物免主密码；桌面(Win/macOS): 开机自启+每次打开首次主密码+可配置过期(如30min)后重新验证；优先开发 Windows。DR-012 A1 纯内存模型无法满足 Android 免密恢复（进程死=会话失，生物解锁无凭据源可取回）。
- 选项与权衡: Q1 红线#5豁免=接受但收窄(仅 Android 生物解锁开启时 Keystore 包裹落盘，默认关闭+开启时明示确认，关闭即销毁包裹；桌面批次无免密恢复需求不触碰豁免，选定); 全量豁免(弃用: 豁免面过大); 不豁免(弃用: 与用户需求根本冲突)。Q2=确认 B2 解锁替代+主密码兜底，且比行业更严: 每 boot 首次强制主密码(Settings.Global.BOOT_COUNT 感知——Bitwarden/KeePassDX 重启后可直接生物解锁，本项目要求首次主密码，选定); 附加因子 B1(弃用: 纯 UX 损害无安全增益)。Q3=合并一个 DR 统一论证"凭据驻留"，分两批次: 批次1=Windows 主线(Tauri 托盘 GUI，用户选 A)，批次2=Android 生物解锁(置后); 拆两个 DR(弃用: 同触凭据驻留边界，分裂论证)。Q4=实现前仅定最低口径 API 28+ TEE Keystore，StrongBox 为检测到硬件时的能力增强，真机矩阵实测随任务2置后为出口前验证项(选定); 真机矩阵前置(弃用: 无真机阻塞开发)。
- 结论: 统一"主密码重新验证策略"模型——Android: 每 boot 首次主密码→生物免密(包裹取回)→主密码/生物变更即失效回退，超时锁库语义不变; 桌面: 每次进程启动首次主密码+idle/absolute 双超时可配，到期重新主密码，无生物无落盘。红线#5 定向豁免自本 DR 起仅对 Android 生物路径生效（范围与开关语义见 Q1）。
- 理由: 用户需求逐字映射；豁免最小化+默认关闭+明示确认+可逆（关即清）；行业对标（Bitwarden/KeePassDX 均为可选开启+Keystore 包裹+生物替代）且本项目每 boot 策略更严；批次拆分尊重"优先 Windows"，共享论证避免边界分裂。
- 影响: 批次1(desktop/ 新 Tauri 应用: 托盘/解锁窗/会话双超时/HKCU 自启、sp-nmhost NM 协议库化嵌入、design_docs/15、SRS SP-F-9xx)；批次2(sp-kffi Keystore 包裹接口、Android 解锁策略、design_docs/14 v1.3)。桌面单进程架构（Tauri 壳内嵌 NM host，免跨进程会话共享，收敛 DR-006 遗留跨进程缓存 backlog）随任务书_step7 设计闸门确认。
- 记录时间 / 会话: 2026-10-29 / SuperPasswdKey 步骤 6 会话（保存场景落地 + step7 规划）
