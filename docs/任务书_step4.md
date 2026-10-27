# SuperPasswdKey 步骤 4：Android（UniFFI 复用内核，Spike 先行）

> 本文件是给 Windows 工作区 ZCode 会话的首次启动提示词。直接把下方代码块内容
> 粘贴为该会话的第一条消息即可。生成于 2026-10-27（Windows 会话，v0.2.0-ext 完成后，
> DR-008 已定；**开发环境已部署完整**，部署报告见 docs/reviews/android_env_部署报告_2026-10-27.md）。

```text

## 项目背景（自包含，无需外部上下文）
- 本机仓库 = GitHub 私有仓 jxsword/SuperPasswdKey 的 clone，main 已含 v0.2.0-ext（CLI 全功能 + 浏览器扩展 F1：crates/sp-nmhost + extension/）
- 项目：Rust 密码管理器（`sp` CLI），KDBX4 兼容（keepass_ng 0.11），Argon2id+双因子
- Cargo workspace：crates/sp-core（密钥策略/条目模型/错误模型）、sp-kdbx（KDBX4 适配，含 list_entry_summaries）、sp-cli、sp-nmhost（native messaging host）
- 关键决策：DR-002（Rust 内核经 UniFFI 复用桌面/Android——本步骤的核心验证对象）、DR-005（keepass_ng 适配层 + 回退预留）、DR-008（本步骤=Spike 时间盒 2–3 天先行，按验收标准分流）
- 必读：docs/SRS.md（§3.7 SP-F-7xx 为扩展已实现态；§2.3 非目标含 Android/iOS——本步骤是 Spike 不改非目标，转正需用户确认）、design_docs/00/01/02（内核 API 面）、09（§3 复杂度：Argon2id 移动端内存约束）、docs/decision_log.md（DR-001~008）、design_docs/08 §5
- 开发模型：Git Flow（feature/android-spike 分支 → PR → develop）；Conventional Commits；设计不符先改 design_docs 再改码

## 任务 0：环境核实（已部署完成，会话开始时快速自检）
本机环境已由 2026-10-27 部署（报告：docs/reviews/android_env_部署报告_2026-10-27.md）。事实清单：
- SDK 根：`E:\dev\android-sdk`（ANDROID_HOME/ANDROID_SDK_ROOT 用户级已设，PATH 含 platform-tools/emulator/cmdline-tools）
- 组件：platform-tools 37.0.1｜platforms android-34/35/36｜build-tools 36.0.0｜**NDK r28.2.13676358**｜emulator 37.1.11｜system-images;android-34;google_apis;x86_64｜AVD `Pixel_7_API34` 已建
- Rust：仓库工具链 1.99.1 与默认 stable 均已装 `aarch64-linux-android` + `x86_64-linux-android` target；cargo-ndk 4.1.2
- **架构口径（重要）**：Windows x64 主机用 **x86_64 模拟器镜像**做 e2e；**aarch64 仅作交叉编译发布目标**（真机验证可选，非 spike 必需）
- 已知约束：cargo-ndk 多 target 时 crate-type 须含 `cdylib`（bin 双 target 会报 "No usable artifacts"）

会话开始自检（4 条只读命令，任何一条异常先修环境再开工）：
```bash
"$ANDROID_HOME/platform-tools/adb.exe" version | head -1
"$ANDROID_HOME/ndk/28.2.13676358/ndk-build" --version 2>&1 | head -1 || ls "$ANDROID_HOME/ndk"
rustup target list --installed | grep android
"$ANDROID_HOME/emulator/emulator.exe" -list-avds
```

## 任务 1：Spike 目标（时间盒 2–3 天，禁止超时扩scope）
产出物（全部入库）：
1. **UniFFI 绑定**：新 crate `crates/sp-kffi`（或 sp-core 内 uniffi feature）导出最小面：create_db/open_db/find_entry/list_entry_summaries/entry_totp_now；生成 Kotlin 绑定并在 **x86_64 模拟器（Pixel_7_API34）** 上编译运行通过
2. **e2e 验证**：模拟器上用 Kotlin 代码打开步骤 3 的测试库 fixture（sp-test-only，可用 `cargo run -p sp-nmhost --example make_test_vault` 生成），断言条目读回一致（对照 crates/sp-kdbx/tests 已有语义）；同时 `cargo ndk -t aarch64-linux-android build` 验证发布目标交叉编译通过
3. **KDF 实测**：64MiB Argon2id 在模拟器/真机的耗时与内存峰值（对标 ≤3s 分流线，写实测数字；模拟器数字需标注"非真机"）
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

```
