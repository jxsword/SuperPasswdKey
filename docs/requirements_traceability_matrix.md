# 需求追踪矩阵（Requirements Traceability Matrix）

- SRS 版本：V1.6（2026-10-31；V1.0 定稿 2026-10-26）｜设计列在设计文档定稿后回填｜测试列在实现阶段回填
- 状态图例：🟢 已覆盖｜🟡 部分（后置项注明）｜⚪ 计划中

| 需求编号 | 需求标题 | 来源（基线/DR） | 设计文档 | 测试用例 | 状态 |
|---|---|---|---|---|---|
| SP-F-101 | 库初始化 | B1/B2/B3, DR-003 | 01, 02 | sp-kdbx 单测/roundtrip 对拍 + `sp init` CLI 实测（2026-10-29 本机建库全流程，含 KDF 校准/keyfile 生成/配置回写） | 🟢 |
| SP-F-102 | 条目 CRUD（全字段加密） | B1 | 01, 02, 03 | sp-kdbx 单测（add/find/list/delete）+ KeePassXC roundtrip 对拍 + CLI item 族实测（2026-10-29/30） | 🟢 |
| SP-F-103 | 修改历史 | B1 | 02 | sp-kdbx edit_entry 单测（update_history 压栈）；CLI 暂无 edit 入口（FFI/Android 面已用：SP-F-805 更新路径 2026-10-29）；**GUI edit 入口落地（SP-F-909，S1b-1：edit_entry 单源复用，history 语义随 kdbx 单测）** | 🟢 CLI 入口缺 |
| SP-F-104 | 附件（add/list/get 命令族） | B21 | 02, 手册 | e2e：1MB add→list→get 双通道往返无损 + 同名覆盖 + 错误码（3/3/2）（2026-10-29，`sp attachment`）；CLI 冒烟实测通过；KeePassXC 对拍（roundtrip.rs 既有） | 🟢 |
| SP-F-105 | 解锁（双因子，无跨进程会话） | B3, 冲刺复审#1 | 01, 03 | 错密码/keyfile 缺失 → 退出码 2（session 单测）；zeroize 审查；每命令独立解锁实测 | 🟢 |
| SP-F-106 | 单库与默认库配置 | 冲刺复审#4 | 03 | config.toml default_db/keyfile 解析单测 + init 回写实测（2026-10-29） | 🟢 |
| SP-F-107 | 库兼容与损坏处理 | derived | 02 | 损坏库三类样例测试 + KDBX4 兼容（keepassxc roundtrip）+ 基线哈希外部修改检测（integrity 单测） | 🟢 |
| SP-F-201 | 命令族 | B5 | 03, 手册 | `sp --help` 全命令面单测化核对（手册覆盖脚本 24 flag 零缺失，2026-10-30）+ 全命令 CLI 实测 | 🟢 |
| SP-F-202 | 单值读取（ACL 0600） | B5 | 03 | read_value 字段解析单测 + --out-file ACL（icacls 展开修复 2026-10-29 实测生效） | 🟢 |
| SP-F-203 | 剪贴板（超时清除） | B20 | 03 | clip 实测（阻塞/超时清除/Win+V 提示文案）；arboard 单测 | 🟢 |
| SP-F-204 | 退出码与错误模型 | 冲刺复审#5 | 03 | 退出码映射单测（0/1/2/3/4 + run 子进程透传）+ e2e 错误码断言（attachment_e2e 3/3/2） | 🟢 |
| SP-F-205 | 版本信息（build.rs 注入） | 冲刺复审#6 | 08 | `sp --version` = 0.1.0 与 Cargo 一致（实测） | 🟢 |
| SP-F-301 | sp run 注入 | B6, 基线§3.2 | 04 | f8 单测 + run_stdin e2e（NUL 记录/不进子进程环境/遮蔽 `<redacted>`）+ 退出码透传 | 🟢 |
| SP-F-302 | sp inject 模板 | B6 | 04 | f8 单测（占位符渲染/保留名校验/输出校验）+ ACL 限权实测 | 🟢 |
| SP-F-303 | 威胁边界文档 | 基线§3.2#8 | 00(README) | README 威胁边界节 + design_docs/09§4（文档评审，步骤 3） | 🟢 |
| SP-F-401 | 四源解析器+发现引擎 | B16, 基线§4.1/4.2 | 05 | importer 单测（dotenv/csv/bw-json/env/files）+ CLI 实测；**局限如实声明**：候选仅含值，源元数据（标题/用户名/URL）不写入（手册 §10.1） | 🟢 附局限 |
| SP-F-402 | 三级去重 | B16, 基线§4.3 | 05 | build_candidates 单测（L1 值哈希/L2 预留/L3 指纹冲突 + REVIEW 行）；L2 待源元数据启用（代码注记） | 🟢 L2 预留 |
| SP-F-403 | 分类/导入/dry-run | B16, 基线§4.4 | 05 | import 命令实测（dry-run 零写入路径单测 + 报告行格式） | 🟢 |
| SP-F-501 | 加密归档导出（默认） | B17, 基线§5 | 06 | export 单测（zip+age 结构/口令两次确认）+ age 互操作口径（官方工具可解） | 🟢 |
| SP-F-502 | 明文导出（三重防护） | B17 | 06 | export 单测（--plaintext 重输主密码 + stdout 恒定不落临时文件） | 🟢 |
| SP-F-503 | 字段损失明示 | 基线§5#5 | 06 | 导出摘要行明示（不含 passkey/回收站）+ 手册 §11 注意事项 | 🟢 |
| SP-F-601 | TOTP 生成 | B14 | 06 | entry_totp_now 单测 + KeePassXC 对拍（roundtrip.rs）+ CLI `-t` 实测 | 🟢 |
| SP-N-301 | 读写性能 | 基线§9 | 07 | 本地计时（步骤 1–3 验收，CI `#[ignore]` 不阻塞——SP-N-301 口径）；大附件保存耗时口径见手册 §14 | 🟢 本地 |
| SP-N-302/303 | KDF/解锁路径 | 基线§6 | 01 | init 校准实测（~700ms 目标/±30% 复核）+ 解锁路径计时单测 | 🟢 |
| SP-N-304 | 零网络 | 红线#4 | 07 | deny bans（根 workspace：reqwest/hyper/tokio 等禁入）+ desktop deny 差异披露（design_docs/15 §7：tokio=tauri 运行时、补偿控制 CSP/无 updater/零 Node）+ 全代码无出站请求 | 🟢 |
| SP-N-305 | 内存卫生 | 红线#5 | 01 | secrecy/Zeroizing 强制 + debug 断言测试 + SP-F-105/804 审查口径 | 🟢 |
| SP-N-306 | 遮蔽完整性 | 基线§3.2#5 | 04 | run_stdin e2e：子进程输出秘密值 100% `<redacted>` 替换断言 | 🟢 |
| SP-N-307 | 测试矩阵 | SP-C-102 | 07 | CI 七 job（fmt/clippy -D warnings/test/deny/e2e/extension/desktop）全绿基线 | 🟢 |
| SP-N-308 | 可移植性 | F5 | 02 | WSL 构建/运行验证（✅ v0.1.2-linux 发布）；CI linux job 常驻 | 🟢 |
| SP-N-309 | 输出语言 | assumption | 03 | CLI 英文输出（帮助/错误）+ 中文文档；手册 SP-N-309 口径一致 | 🟢 |
| SP-N-310 | 进程加固（VirtualLock/缓解策略/反调试） | design_docs/09§4 | 01, 09 | **回填核查 2026-10-30：未实现**（全仓无锁页/缓解策略/反调试代码，design_docs/09§4 已标注）——README 威胁边界同步如实化；列入 backlog | 🟡 实现缺口 |
| SP-N-311 | 文件完整性基线 + sp doctor | design_docs/09§5 | 09 | 基线哈希（打开成功静默更新/失败提示）integrity.rs 实现并接线（open_with_config）；**doctor 子命令未实现**（CLI 无此命令）——列 backlog | 🟡 部分实现 |
| SP-N-312 | 自动备份轮换 | design_docs/09§5 | 09 | rotate_backup 已实现并接线全部写命令（save_and_backup）；单元测试补齐见同 PR（11 次→10 份滚动+只读断言） | 🟢 |
| SP-C-101~106 | 约束（红线/门禁/许可证/LF/向量） | 基线§6, DR-004 | 00, 07, 08 | CI 门禁（clippy -D warnings/deny/test 矩阵）+ .gitattributes LF + 测试向量来源审查 | 🟢 |
| SP-F-701 | native messaging host（origin 双重校验/注册） | B4, DR-006 | 10, 11 | origin 单测（bound_origin/check_caller_origin/launched_by_browser）+ register.ps1 实测（chrome/edge 双注册 + desktop 模式）+ 本机 Chrome 实测（2026-10-29/30） | 🟢 |
| SP-F-702 | 协议（帧格式/动作集/P1P2 适配器） | B4, DR-006 | 11 | protocol 单测（帧往返/越界/坏输入不 panic）+ fuzz nm_protocol + save-login 向前兼容实测（B3a） | 🟢 |
| SP-F-703 | 会话状态机（缓存/超时锁库/MV3 保活） | B4, DR-006 | 10, 11 | session 单测（缓存/超时锁库/因子零化）+ 保活 alarms 实测（扩展 popup 解锁后） | 🟢 |
| SP-F-704 | 表单探测与填充（单页） | B4 | 12 | detect 单测 12 用例 + Chrome 实测（demo 页 + 真实页，2026-10-29/30 用户验收填充链路 ✓） | 🟢 |
| SP-F-705 | popup 列表与搜索 | B4 | 12 | Chrome 实测（列表/搜索/填充按钮；用户验收截图 2026-10-29） | 🟢 |
| SP-F-706 | URL 匹配引擎（host 侧） | B4 | 12 | match_url 单测（精确/base domain/裸 host/端口/localhost 语义）+ 填充匹配实测 | 🟢 |
| SP-F-707 | 扩展端机密边界（不落存储） | 红线#4#5 | 10, 12 | 代码审查（含 save-login 暂存队列纯内存复查 2026-10-30） | 🟢 |
| SP-F-708 | 登录自动保存（捕获/浮层/sp.save-login） | step8 | 11 §3, 12 §8, 15 §9 | host 侧模拟握手 34 测试（PR #35）+ 扩展 vitest 19（PR #36）均绿；**Chrome 手工清单 9 项用户验收全过（2026-10-30，docs/reviews/chrome_manual_checklist_保存流_2026-10-30.md）** | 🟢 |
| SP-F-801 | Android 内核绑定（UniFFI/actor 线程模型） | DR-002, DR-008/009 | 13 | sp-kffi e2e, 模拟器 e2e（✅ spike；M1 正式化复跑全绿 2026-10-28） | 🟢 |
| SP-F-802 | Android 解锁会话（超时锁库/凭据清零） | 红线#5#7, DR-009 | 13, 14 | 超时可注入测试 e2e PASS（TestActivity idle_timeout_ms=2000 → 锁定+机密操作拒绝，2026-10-28）；真机 300s 空闲锁实测 ✓（步骤 4）；首建校准/错误呈现/认证回路 UI 回归全通（M1 正式化） | 🟢 |
| SP-F-803 | 系统 Autofill 对接（M1） | DR-009 | 14 | 原生/包名路径真机+模拟器 e2e 全通 ✅；认证回路全链路回归 PASS（2026-10-28，含 presentation 修复）；web 路径待非 MIUI 真机（R14-1/3） | 🟡 |
| SP-F-804 | Android 机密边界（不落 SharedPreferences/日志） | 红线#4#5 | 13, 14 | 代码审查（日志仅记值长度/类别，无机密落盘路径；含首建流程复查 2026-10-28；含保存路径 intent extra 边界复查 2026-10-29）；logcat 实测零凭据值泄漏 | 🟢 |
| SP-F-805 | Android Autofill 保存场景（M2，DR-012 A1） | DR-012 | 14 §7 | 模拟器演示链路全通（新凭据→系统保存建议→确认入库→进程重启重开库→数据集可读；拒绝路径零写入，2026-10-29）；e2e 断言（锁库后 saveSession 拒绝 + 保存闭环重开读回）进 TestActivity 全绿；sp-kffi 主机侧测试 save_session 语义覆盖（73 测试基线） | 🟢 |
| SP-F-901 | 桌面托盘壳（Tauri 常驻/关窗隐藏/菜单） | DR-013, DR-014 | 15, 16 | `cargo tauri build` 本地实测 + CI desktop job（fmt/clippy/test/build）；菜单行为手工验收 | 🟡 手工项待用户 |
| SP-F-902 | 桌面解锁会话（每进程首次主密码/因子留驻 actor） | DR-013, 红线#5 | 15 | unlock/错密码/锁库单测（复用 sp-kffi actor 语义，批次 1 已建）；真实 kdbx 解锁（步骤 7 本地实测） | 🟢 |
| SP-F-903 | 桌面双超时（idle/absolute 独立可配，到期锁库） | DR-013 | 15 | idle=0 注入单测（首访问即锁）+ 重解锁重置计时单测（批次 1）+ NM watcher sp.event 通知（步骤 7） | 🟢 |
| SP-F-904 | 桌面设置持久化（settings.json 非机密/损坏回落） | DR-013 | 15 | roundtrip/损坏回落/BOM 单测（2026-10-29）；落盘内容人工核对 | 🟢 |
| SP-F-905 | 桌面开机自启（HKCU Run 写删/偏好对账） | DR-013 | 15 | 独立值名注册表 roundtrip 单测（2026-10-29）；`reg query` 验证为用户手工验收项 | 🟡 手工项待用户 |
| SP-F-906 | NM host 单进程嵌入（协议库化复用/locked 弹窗/EOF 退出） | DR-013 | 15 | 模拟 NM 握手测试全链路（serve+适配器：ping→解锁→get-logins→get-secret→lock→locked，2026-10-29）；register.ps1 -Target desktop 双轨；旧 sp-nmhost 回归（30 测试不变）；手工 Chrome 验证待用户 | 🟡 手工项待用户 |
| SP-F-907 | 桌面浏览器填充链路（URL 匹配/单条取密，纪律单源） | DR-013 | 15 | 匹配命中+子串反例+响应无密码+锁定拒绝断言（模拟握手测试内，2026-10-29） | 🟢 |
| SP-F-908 | 桌面 GUI 条目浏览（列表/搜索/详情只读/TOTP/复制，S1a MVP） | DR-014 | 16 | 组件测试（vitest：列表渲染/搜索过滤/详情/TOTP 轮询/复制调用）+ Rust session 测试扩展（entry_list/entry_get/entry_totp/clip_copy 惰性锁库语义）；CI desktop job 全绿；真实库手工验收待用户 | 🟢（自动化）/🟡 手工项待用户 |
| SP-F-909 | 桌面 GUI 条目写路径（创建/编辑/删除 + 显示/放大显示，S1b-1） | DR-014, SP-F-103 | 16 | kffi find_entry_full/delete_entry 单测（custom_fields 保真 roundtrip + 删除 NotFound）；Rust session 测试（创建/编辑改名/删除/锁定拒绝/错 otp 拒绝）；vitest（表单校验对齐 CLI/保真回传/删除确认/放大显示模态/显示切换）；真实库手工验收待用户 | 🟢（自动化）/🟡 手工项待用户 |

\* 测试用例编号在实现阶段（H3/H4/H5/H6/H7）按此前缀回填；round-trip 对拍测试覆盖 SP-F-101/102/103/104/601 的互操作维度。
