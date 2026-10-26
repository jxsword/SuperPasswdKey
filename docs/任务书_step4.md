# SuperPasswdKey 步骤 4：Android（UniFFI 复用内核，Spike 先行）

> 本文件是给 Windows 工作区 ZCode 会话的首次启动提示词。直接把下方代码块内容
> 粘贴为该会话的第一条消息即可。生成于 2026-10-27（Windows 会话，步骤 3 / v0.2.0-ext 完成后，DR-008 已定）。

## 项目背景（自包含，无需外部上下文）
- 本机仓库 = GitHub 私有仓 jxsword/SuperPasswdKey 的 clone，main 已含 v0.2.0-ext（CLI 全功能 + 浏览器扩展 F1：crates/sp-nmhost + extension/）
- 项目：Rust 密码管理器（`sp` CLI），KDBX4 兼容（keepass_ng 0.11），Argon2id+双因子
- Cargo workspace：crates/sp-core（密钥策略/条目模型/错误模型）、sp-kdbx（KDBX4 适配，含 list_entry_summaries）、sp-cli、sp-nmhost（native messaging host）
- 关键决策：DR-002（Rust 内核经 UniFFI 复用桌面/Android——本步骤的核心验证对象）、DR-005（keepass_ng 适配层 + 回退预留）、DR-008（本步骤=Spike 时间盒 2–3 天先行，按验收标准分流）
- 必读：docs/SRS.md（§3.7 SP-F-7xx 为扩展已实现态；§2.3 非目标含 Android/iOS——本步骤是 Spike 不改非目标，转正需用户确认）、design_docs/00/01/02（内核 API 面）、09（§3 复杂度：Argon2id 移动端内存约束）、docs/decision_log.md（DR-001~008）、design_docs/08 §5
- 开发模型：Git Flow（feature/android-spike 分支 → PR → develop）；Conventional Commits；设计不符先改 design_docs 再改码

## 任务 0：环境核实与补齐（Windows + Android 工具链）
1. rustc/cargo 1.99.1（Windows 侧已就绪）；`rustup target add aarch64-linux-android`（NDK 交叉目标）
2. Android 工具链：Android Studio 或命令行（SDK + NDK r27+）、JDK 17+、模拟器（aarch64 系统镜像）；cargo-ndk（`cargo install cargo-ndk`）
3. UniFFI：`uniffi` crate 版本选型（当前 0.29 系，核对与 keepass_ng 依赖树无冲突）
4. 若 Mimosa 安全钩子拦截写入：把内容交用户手动落盘，不静默绕过（先例：spawner.rs、service-worker.ts）

## 任务 1：Spike 目标（时间盒 2–3 天，禁止超时扩scope）
产出物（全部入库）：
1. **UniFFI 绑定**：新 crate `crates/sp-kffi`（或 sp-core 内 uniffi feature）导出最小面：create_db/open_db/find_entry/list_entry_summaries/entry_totp_now；生成 Kotlin 绑定并在 **Android 模拟器（aarch64）** 上编译运行通过
2. **e2e 验证**：Android 模拟器上用 Kotlin 代码打开步骤 3 的测试库 fixture（sp-test-only），断言条目读回一致（对照 crates/sp-kdbx/tests 已有语义）
3. **KDF 实测**：64MiB Argon2id 在目标模拟器/真机的耗时与内存峰值（对标 ≤3s 分流线，写实测数字）
4. **design_docs/13_Android_UniFFI.md 骨架**：绑定边界、生命周期（Database/Send/Sync 与 JNI 线程模型）、风险清单；SRS/追溯矩阵如需改动仅列"待转正清单"不动正文
5. **结论与分流建议**：按 DR-008 分流标准逐项给出通过/受阻判定，交用户拍板后续主线（届时写 DR-009）

## 任务 2：验收与出口
- 分流标准（DR-008）：① UniFFI Kotlin 绑定编译通过；② Android 模拟器 sp-kdbx 打开 KDBX4 e2e；③ 64MiB Argon2id 目标设备 ≤3s——全过→M1 系统 Autofill 为主线；任一核心受阻→回桌面阶段1（P2 加密+密钥复用开库）并启动 DR-005 回退评估
- 本地门禁：cargo test --workspace 全绿（Android spike 不得破坏桌面既有测试）；clippy -D warnings；deny 全绿；fmt
- push 后 CI 四 job（windows/linux/deny/extension）全绿；**不打 tag**（spike 无发布）

## 约束
- 红线十条（docs/SRS.md §7）不回退；测试向量来源锁定（SP-C-106）
- Spike 期间不改 crates/sp-core / sp-kdbx / sp-cli / sp-nmhost 的既有公开 API（新增独立 crate 或 feature 隔离）
- 机密不落 Android SharedPreferences/日志；Keystore 集成不在 spike 范围（backlog 单列）
- 不做：iOS、服务器轨、多库、生物识别（后两者需独立 DR）
- 任何 Mimosa 拦截：内容交用户手动落盘，不静默绕过
