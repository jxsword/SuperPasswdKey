# SuperPasswdKey 技术选型与排期（Rust 栈）

- 版本：v1.0（2026-10-25，经两轮复审修正后定稿）
- 决策依据：DR-001（路线 B 自研）、DR-002（Rust）、DR-003（KDBX4 兼容 + CLI 先行）、DR-004（AGPL-3.0），见 `docs/decision_log.md`
- 需求基线：《自研需求基线_SuperPasswdKey.md》（P0 边界 = B1–B7/B14/B16–B17/B20 + 十条红线；里程碑 = 基线 §9 M1–M8）
- 已确认的开发约束：全职投入（6–8h/天）、单人 + AI 辅助编码、主力环境 Windows（Git Bash）

---

## 1. 决策摘要

| 决策项 | 结论 | 弃用方案 |
|---|---|---|
| 总路线 | 自研 SuperPasswdKey | 直接使用现有软件组合（F8/F9/F10 空白无法补齐） |
| 核心语言 | Rust 全家桶 | Go（GC 清零不可靠）、TS/Electron（内存卫生最差 + 可复现构建不可行） |
| 数据格式 | KDBX4 兼容 | 自有格式（+2~4 个月，MVP 不可行） |
| MVP 范围 | CLI 内核先行（M1–M3） | GUI 同步开发（首版延后至 4–5 个月） |
| 许可证 | AGPL-3.0 | GPL-3.0（不约束网络服务端）、MIT/Apache（无 copyleft） |

## 2. 技术栈（Cargo workspace，单仓库）

| 模块 | 技术选型 | 用途 / 关键依赖 |
|---|---|---|
| `sp-core` | Rust + RustCrypto 生态 | 三层密钥体系（账户→库→条目）、条目模型、合并引擎；`argon2`（内存 ≥64MiB 起步）、`aes-gcm` / `chacha20poly1305`、`hkdf`、`zeroize` + `secrecy`（红线 #5 密钥确定性清零） |
| `sp-kdbx` | keepass-rs → keepass_ng → 自实现写路径（三档备选，见 §6 风险 R1） | KDBX4 兼容层；格式公开规范 https://keepass.info/help/kb/kdbx_4.html |
| `sp-cli` | clap + rpassword + keyring + arboard | `sp item/read/run/inject/import/export/clip` 命令族；服务令牌进 OS 密钥链；剪贴板超时自动清除（B20） |
| F8 注入 | std::process::Command + 流式输出遮蔽 | `sp run` 对标 `op run` 十项验收（基线 §3.2）；Windows/Git Bash 一等公民 |
| F9 导入 | gitleaks 规则 TOML 内嵌（MIT）+ serde/toml + 自研解析器 | 1pux(zip+xml)/Bitwarden JSON+CSV/keepass2xml/通用 CSV/.env/环境变量快照六类 P0 源；三级去重 |
| F10 导出 | zip + age 加密归档；stdout 流式一等支持 | 默认加密归档、明文导出三重防护（基线 §5 五条红线） |
| 桌面 GUI（M4） | Tauri 2（Rust 后端 + Web 前端） | 复用 sp-core；不做 Electron |
| 浏览器扩展（M4） | TypeScript WebExtension + Rust native messaging host | 参照 KeePassXC-Browser 公开协议（keepassxc-protocol.md） |
| Android（M7） | Tauri 2 移动壳或 Kotlin UI（M4 期间 1 周 spike 定）+ UniFFI/JNI 绑定 sp-core + 原生 Kotlin Autofill 服务 | Tauri 2 移动端已核实官方稳定（2024-10 起），Android 成熟度优于 iOS；Autofill 服务必须是原生 Kotlin |
| 服务器轨（M6） | axum + SQLx（SQLite/PG） | 零知识协议：服务端仅存密文+二次加盐哈希；授权边界代码重点审计（Vaultwarden CVE 教训） |
| 同步（M5） | WebDAV 客户端（reqwest 系） | 坚果云首选管道；条目级合并引擎自实现（M5a，可与 M1 并行） |
| 构建/CI | cargo-dist + rustup 工具链锁定 + cargo-audit/cargo-deny + cargo-fuzz | GitHub Actions windows-latest + ubuntu-latest 双矩阵（M1 建）；Android CI M7 前建 |
| 签名 | Authenticode（Win）+ notarization（macOS，若做）+ Minisign（全平台离线验证，公钥多位置预置） | 三层签名，见 §7 |

## 3. 难度评估

| 模块 | 难度 | 关键难点 |
|---|---|---|
| M1 加密内核+CLI | 中（含头号风险 R1） | 标准原语组装而非自造算法；zeroize 纪律贯穿；keepass-rs 写路径实验性（§6 R1） |
| M2 F8 注入 | 中 | Windows 进程/环境变量细节、遮蔽流过滤、系统关键变量保护名单（对齐 infisical） |
| M3 F9/F10 | 中高 | 6+ 格式解析器（1pux zip+xml 最繁）、规则引擎、三级去重；fuzz 接入 |
| M4 桌面+浏览器扩展（F1） | **高（全项目最大）** | native messaging 全链路、表单匹配与自动保存（1Password 最深护城河）；Tauri 移动 spike |
| M5 云盘轨 | 中 | WebDAV、冲突副本检测；**条目级合并算法自实现**（KDBX 兼容只解决格式层） |
| M6 服务器轨 | 中高 | 零知识协议设计；授权边界代码须重点审计 + 100% 覆盖 |
| M7 Android | 中高 | UniFFI/JNI 绑定、系统 Autofill 框架匹配精度 |
| M8 可复现构建+审计 | 中 | 跨平台逐字节复现有未决难题（KeePassXC 先例）；降级路径="源码→构建→哈希"公开验证 |

## 4. 工期（全职 6–8h/天，单人 + AI 辅助；已含第 1 轮复审修正）

| 里程碑 | 内容（含出口标准要点） | 工时 | 累计日历（含缓冲与自用期） |
|---|---|---|---|
| M1 | 加密内核 + CLI + KDBX4。出口：KDF 测试向量与 AEAD 已知答案测试通过；KDBX 双向互操作 round-trip（sp↔KeePassXC）通过；`sp read`/`sp clip` 可用；"源码→构建→哈希"公开验证流程可用；**开始自用 dogfooding** | 3–5 周（含 keepass-rs spike 1 周） | ~1.5 个月 |
| M2 | F8 注入。出口：基线 §3.2 十项验收全过；Git Bash 下 `sp run -- printenv <KEY>` 可用；B14 TOTP（`sp read --totp`） | 2–3 周 | 2 个月 |
| M3 | F9/F10。出口：六类源导入+三级去重；默认加密归档含附件；明文三重防护+stdout 流式；**全部解析器过 cargo-fuzz** | 3–5 周 | **MVP ≈ 2.5–3.5 个月** |
| M4 | Tauri 桌面 + 浏览器扩展（F1）+ 移动端 1 周 spike。出口：native messaging 填充可用、自动保存 | 6–10 周 | 5 个月 |
| M5a/b | 条目级合并引擎（a，可与 M1 并行开发）+ WebDAV/冲突检测（b）。出口：并发写测试通过；坚果云端到端可用 | 4–6 周 | 6.5 个月 |
| M6 | 自建服务器轨。出口：零知识服务端；HTTPS+端口收口；备份内建；授权代码 100% 覆盖 | 4–6 周 | 8 个月 |
| M7 | Android + P1 批次（B12/B13）。出口：系统 Autofill 可用 | 5–8 周 | 9.5 个月 |
| M8 | 可复现构建 + 第三方审计（审计排期另计）。出口：逐字节复现比对公示或降级路径达成 | 2–4 周 | **≈ 10–12 个月** |

- **MVP（可日常替代 KeePassXC+gopass 工作流）：2.5–3.5 个月**
- **F1 桌面自动填充可用：约 5 个月**
- **全功能：10–12 个月**（= 纯工时累加 9–10 个月 + 20% 缓冲 + 每里程碑出口 1–2 周 dogfooding；M1 出口即开始自用）
- 工时假设：AI 辅助压缩样板代码约 30–40%；加密正确性、native messaging、Autofill 适配仍需人工验证。每阶段出口复验十条红线不回退（基线 §9）。

## 5. P0 功能 → 里程碑落点（补全）

| P0 项 | 落点 |
|---|---|
| B1 条目模型 / B2 密钥分层 / B3 密钥文件双因子 / B20 剪贴板清除 | M1 |
| B5 CLI 命令族 / B6 F8 注入 / B14 TOTP | M1–M2 |
| B16 F9 导入 / B17 F10 导出 | M3 |
| B4 F1 桌面填充+扩展 | M4（Android M7、iOS P2 后置） |
| 云盘轨最小实现（§7.2） | M5 |
| §6 十条红线 | 全程，每出口复验 |

## 6. 风险表

| # | 风险 | 等级 | 缓解 |
|---|---|---|---|
| R1 | **keepass-rs KDBX4 写路径为 experimental**（2026-10-25 核实：读取成熟含 Argon2，写入 experimental；活跃 fork keepass_ng） | 高 | M1 第 1 周 spike：双向 round-trip 互操作测试；三档备选顺位 keepass-rs → keepass_ng → 按公开规范自实现写路径（+3~4 周，已含缓冲） |
| R2 | 单人维护=授权/加密关键路径无人复核（Vaultwarden 教训反射到自身） | 高 | 关键路径（sp-core 加密、M6 授权）100% 测试覆盖 + fuzz + 设计评审记录留档，为 M8 第三方审计降成本 |
| R3 | F1 自动填充工程量（1Password 最深护城河） | 中高 | 参照 KeePassXC-Browser 公开协议；MVP 不含；移动端靠系统 Autofill 框架 |
| R4 | 可复现构建跨平台未决难题 | 中 | 降级路径先达成（源码→构建→哈希公开验证）；参照 KeePassXC #4411 |
| R5 | 坚果云 WebDAV 服务器域名未核实（分报告E 遗留项） | 低 | M5 前实测确认 |
| R6 | gitleaks 已进维护期（作者转向 betterleaks），规则冻结 | 低 | 规则静态内嵌后维护责任自负；235 规则基线够用；可跟进 betterleaks |
| R7 | AI 辅助率假设失准 | 中 | 每里程碑出口实测校准下阶段排期 |

## 7. 分发与签名（红线 #1/#3 执行细则）

1. 官方推荐获取路径 = 签名二进制（Authenticode + Minisign）；公钥预置于文档、源码仓库、独立公证渠道三处。
2. crates.io 仅作为源码分发渠道（cargo install 本地编译，投毒需改源码）——规避 CVE-2026-42994 式预编译包管理器投毒。
3. 不做竞价广告投放（KeePass Punycode 假冒站教训）；搜索品牌词被抢注时发布公告。
4. 构建环境与密钥物理隔离；签名密钥离线保管。

## 8. 许可证：AGPL-3.0（DR-004）

- 含服务端轨，AGPL 网络 copyleft 防止服务端被闭源 fork（对齐 Vaultwarden）。
- 依赖兼容性：RustCrypto/MIT/Apache 全兼容；gitleaks 规则 MIT 可内嵌（保留署名）；age MIT/Apache 参考无冲突。

## 9. 待补核实项（继承基线 §8.5 + 本轮新增）

- M1 第 1 周：keepass-rs / keepass_ng 许可证与活跃度终核（crates.io 标注 MIT，待确认）；CLI 二进制名 `sp` 冲突终检（备选 `spk`）。
- M4 期：Tauri 2 移动壳 vs 纯 Kotlin spike 结论。
- M5 前：坚果云 WebDAV 服务器域名实测。
- 涉及 1PUX 互导时：1PUX 是否含附件实测（官方文档未明示）。

## 10. 与基线/决策记录的映射

- 需求与验收标准唯一来源：《自研需求基线_SuperPasswdKey.md》§2–§6；本文不重复定义，只做技术栈、难度、排期映射。
- 决策记录：DR-001~DR-004 见 `docs/decision_log.md`。
- 开工首任务：M1 keepass-rs spike（R1）。
