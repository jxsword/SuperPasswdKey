# 设计文档复审记录

## 第一轮（文档 00–08 初稿复审，2026-10-26）

- 对象：design_docs/00–08（9 份）｜依据：SRS V1.0 + 追溯矩阵
- 结论：**复审通过**。发现 5 项问题，全部处置；PlantUML 双轨策略验证成功。

| # | 文件 | 发现 | 处置 |
|---|---|---|---|
| 1 | 02/05 | PlantUML `trait` 关键字解析失败 | 改 `interface`（Rust trait ≈ 接口），md 内源码同步修正，重渲染通过 |
| 2 | 00 | 写入路径"临时文件+原子替换"与 02 `save_atomic` 重复定义风险 | 确认为同一机制的两处描述（00 总述/02 落点），02 为权威——无冲突，接受 |
| 3 | 01 vs 02 | 条目模型归属：01 组件图把"条目模型"画在 sp-core，02 映射表用 EntryModel | 确认归属 sp-core（01 为准），02 的 EntryModel 是 sp-core 类型在适配层签名中的引用——一致，接受 |
| 4 | 04 | 遮蔽过滤器 flush 保证论证：尾缓冲 ≥ 最长值-1 是否充分 | 论证成立（跨边界值至少留 1 字节无法完成匹配）；原文已含说明，通过 |
| 5 | 03/06 | `sp item get --totp` 与命令树未列 `--totp` flag | 03 命令树 IGET 已含 `[--totp]`——确认一致；06 引用无误，通过 |

- 与 SRS 追溯核对：SP-F-101~601 / SP-N-301~309 / SP-C-101~106 全量覆盖，无孤立需求、无无源设计 ✅
- 遗留（实现期）：① `Result<DbHandle>` 泛型括号渲染为文本，不影响语义；② 遮蔽过滤器若 Aho-Corasick 依赖体积过大（deny 审查），回退线性多模式查找

---

## 第二轮（六问安全方案复审 → 09 文档落盘，2026-10-26）

- 触发：用户六问（加密算法/存储格式/密钥文件异同/复杂度资源/进程保护/文件保护）
- 过程：基于分报告 A§1.4/B§1.4/C§1（2026-10-25 实测核实数据）对比分析 → 两轮复审修正 13 项 → 落盘《09_安全实现方案.md》

| 轮 | # | 修正 | 类别 |
|---|---|---|---|
| R1 | 1 | 删除 CFG（Rust 生态非 CFG-aware，避免虚假安全声明）；缓解策略收敛为 ExtensionPointDisable + DynamicCode | 消除虚假声明 |
| R1 | 2 | VirtualLock 限制如实声明（内存压力可强制换出/休眠残留/失败降级警告） | 边界诚实 |
| R1 | 3 | 完整性指纹定位修正："变更检测"非"安全边界"（协同篡改不可防） | 边界诚实 |
| R1 | 4 | keyfile 语义精确化：raw 32B 生成 + 任意文件 SHA-256 派生 + keepass 复合密钥公式 | 互操作正确性 |
| R1 | 5 | 备份必须在原子替换后复制；云盘同步场景用户知情声明 | 事务正确性 |
| R1 | 6 | 反调试定位降级"低成本摩擦+事后告警" | 边界诚实 |
| R1 | 7 | 资源数字标注"估算，H1 spike 实测回填 SP-N-301/302" | 防虚假承诺 |
| R2 | 1 | keepass_ng XML DOM 残留未必全量零化——如实声明（与 KeePassXC 同级，优于 .NET 原版） | 边界诚实 |
| R2 | 2 | keyfile 熵源锁定 getrandom（OS 熵源） | 实现规范 |
| R2 | 3 | 完整性基线更新策略：打开成功静默更新（多设备防误报），告警仅在打开失败+哈希不符；`sp doctor` 手动核查 | 设计落地缺陷修正 |
| R2 | 4 | age 归档"官方工具独立可解密"列为 F10 验收标准（逃逸通道保证） | 验收增强 |
| R2 | 5 | 备份轮换策略明确：10 份滚动、UTC 时间戳命名、只读属性 | 实现规范 |
| R2 | 6 | 杀软误报说明 / SP-F-303 范围扩展 / `sp kdf-upgrade` backlog | 收尾 |

- 落盘影响：新增 09；修订 01（MemoryGuard）/07（T-N310~312 测试）/SRS（SP-N-310~312）/追踪矩阵（三行）；SP-F-303 范围扩展
- 结论：**09 定稿，设计文档整体定稿（待用户最终确认）**

---

## 原第一轮内容归档前的核验清单（保留）

- 与 SRS 的追溯核对（第一轮已做全量）：✅

---

## 步骤 2：Linux（WSL ubuntu2604）移植轮（2026-10-27，feature/linux-port）

- 对象：design_docs/02/03/04 + sp-kdbx/sp-cli 实现｜环境：Ubuntu 26.04, rustc 1.99.1, keepassxc-cli 2.7.10

| # | 发现 | 处置 |
|---|---|---|
| 1 | 02 §5 声明 trait `FilePerms`/`ClipboardBackend`，实现实为 cfg 编译期分支（无 trait） | 按实现修正 02 §5：trait 回退方案仅保留为 DR-005 预留；权限双平台实现 + 单测；剪贴板 arboard 跨平台（X11/Wayland） |
| 2 | 03 §5 平台表"POSIX（步骤 2）"已过时；剪贴板行"后端 trait 预留"与实现不符 | 更新为"v0.1.2-linux 已实现"；0600 单测覆盖（commands::restrict_perms_sets_mode_0600）；新增 run 机密传递行 |
| 3 | 04 §4 `--stdin` 列为"POSIX 增强项"未定语义 | 新增 04 §4.1 定稿：对齐 gopass `env --stdin`（官方 man page 语义核实），NUL 分隔 `KEY=VALUE\0` 记录走 stdin，机密不进子进程环境；e2e 测试覆盖（crates/sp-cli/tests/run_stdin_e2e.rs） |
| 4 | roundtrip.rs keepassxc-cli 路径硬编码 Windows，且未实现 SRS 声明的 `SP_KEEPASSXC_CLI` 覆盖 | 路径解析统一：env 覆盖 → `cfg!(windows)` 平台默认（Windows Program Files / Linux /usr/bin），Linux 对拍全通过 |
| 5 | deny advisories 失败：zstd/zstd-safe 7.3.0 三条 RUSTSEC（2026-0391/0398/0403，内存安全类），修复版 zstd-safe ≥8.2.0 经 zip 4.6.1 仍不可达 | F10 仅写 Deflated、不读 zip → zip 改 `default-features = false, features = ["deflate"]`（4.6.1），zstd/bzip2/lzma/aes 全链移出依赖树，deny 全绿；攻击面与体积同步缩小 |
| 6 | sp-cli 为纯 bin crate，fuzz 目标无法引用解析器 | 新增 lib.rs（模块 pub 化，bin 入口不变）；fuzz/ 独立 workspace，env_parser 目标（.env 解析器），结果见 docs/reviews/ |

- 门禁：cargo test --workspace 全绿（27 测试 + roundtrip 对拍）/ clippy -D warnings 零告警 / fmt / deny 全绿 / build 零警告
- CI：新增 linux job（fmt/clippy/test，cargo-fuzz 不进 CI，07 §3 同步）
