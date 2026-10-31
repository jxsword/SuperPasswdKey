# STAGE-PLAN — 分阶段执行计划（v0.4.x 路线）

> **使用方式**：每个阶段 = 一个新会话。新会话第一件事：读 [HANDOFF-2026-10-31.md](HANDOFF-2026-10-31.md)（最新版）+ 本文档对应阶段节，然后**复制该阶段的"新会话提示词"**开始执行。阶段完成：勾选本文件进度标记 + 在 docs/plans/ 追加该阶段交付说明（作为下阶段输入）。
>
> 纪律（全阶段通用）：Git Flow（feature/fix/docs/* → PR → CI 七 job 全绿 → 合并 develop）；禁止直推 develop/main；机密/口令不经 Agent；源码/示例/测试零真实凭据字面量；Mimosa 拦截按 HANDOFF §3.7 处置。

## 0. 总览

| 会话 | 阶段 | 内容 | 交付物（给下阶段） | 状态 |
|---|---|---|---|---|
| #1 | **S0 卫生与同步** | push develop、.gitignore 微调、AGENTS.md 扩充、CI 缓存 6 项、develop→main 合并 | 同步的 main/develop + 扩充的 AGENTS.md + 缓存 CI | ✅ 全部完成（交付说明：[S0-DELIVERY-2026-10-30.md](S0-DELIVERY-2026-10-30.md)） |
| #2 | **S1a GUI 设计+MVP** | design_docs/16（Vue3+Vite/DR-014/信息架构/安全边界）+ 脚手架 + 解锁集成/列表/搜索/详情 | design_docs/16 + GUI MVP | ✅ 全部完成（交付说明：[S1a-DELIVERY-2026-10-30.md](S1a-DELIVERY-2026-10-30.md)） |
| #3 | **S1b GUI 写路径** | **拆两批**：S1b-1=详情交互增强（显示/放大显示）+ 条目创建/编辑/删除（custom_fields 保真）；S1b-2=标签全栈（KDBX4 原生 Tags，CLI+GUI）→ 附件 add/list/get → 自定义字段编辑 UI | GUI 写路径完整 | ✅ 全部完成（交付说明：[S1b-DELIVERY-2026-10-31.md](S1b-DELIVERY-2026-10-31.md)） |
| #3.5 | **验收测试与反馈修复（插入轮）** | 验收清单 v1.2 入库（GUI 28 项 + CLI 22 项 + 注入 15 项）；反馈修复：GUI 编辑防锁/keyfile 路径记忆+浏览/保存即时刷新；CLI 参数校验前置（item create/read/clip/run URI）；同名撞车注记；CI 分区触发（DR-015） | 验收清单 v1.2 + 修复全合并 | ✅ 完成（PR #57–#69；状态见 [HANDOFF-2026-10-31.md](HANDOFF-2026-10-31.md) §1 验收状态行；**待用户回归**：GUI B 组剩余项 + A10.4 LLM 实战） |
| #4 | **S3 Mimosa 基线审计** | 全仓审计（CLI+扩展+桌面+**完整 GUI 含写路径/标签/附件** + 验收轮修复）+ 修复 | 审计报告 + 修复 | ✅ 完成（Mimosa 回执 0 发现/inconclusive；人工复审 9 修复+12 组豁免，报告：[s3_mimosa_基线审计_2026-10-31.md](../reviews/s3_mimosa_基线审计_2026-10-31.md)，交付说明：[S3-DELIVERY-2026-10-31.md](S3-DELIVERY-2026-10-31.md)） |
| #5 | **S1c 收尾+发布** | 导入导出向导/注入启动器/.env 工作流文档 + 发布前 Mimosa 复审 + **v0.4.0** 发版 | v0.4.0 | ✅ 全部完成（交付说明：[S1C-DELIVERY-2026-11-01.md](S1C-DELIVERY-2026-11-01.md)；发布前增量复审 docs/reviews/s1c_发布前增量复审_2026-11-01.md 0 发现；v0.4.0 draft Release 已构建待维护者签名发布；交接：[HANDOFF-2026-11-01.md](HANDOFF-2026-11-01.md)） |
| #6 | **S4 Android 批次 2** | 真机前置 + 任务书先行 | Android Keystore/生物/每 boot 主密码 | ☐ |
| — | Backlog（不排期） | P2 协议加密、密钥复用开库、1pux、指纹去重三级、ssh-agent/shell 包装、服务器轨 | — | — |

发布节奏：S1c 完成 → v0.4.0（桌面 GUI）；S4 完成 → v0.5.0。

---

## S0 卫生与同步（会话 #1）

**目标**：仓库卫生、Agent 约束入库、CI 环境部署缓存化、main/develop 同步。

**任务**：
1. ✅ push 本地 develop（含 .gitignore 更新提交 2313705）——实测远端已同步（fetch 后 0 0，push 回显 Everything up-to-date）
2. ✅ .gitignore 微调：根级补 `dist/`；删除冗余行 `/target`（被 `**/target/` 覆盖）与错位行 `desktop/target/`（实际路径 desktop/src-tauri/target 由 `**/target/` 覆盖）
3. ✅ 未跟踪的 AGENTS.md 按 §AGENTS 规则草案扩充后提交入库（9 条：备份/分支/破坏性操作/签名/门禁/交接/发布/Mimosa/凭据卫生）
4. ✅ CI 缓存优化按 §CI 缓存配方逐项实施——钉版 tauri-cli 2.12.4 / cargo-ndk 4.1.2 / KeePassXC 2.7.12（官方便携 zip 替代 choco）；tauri-cli、cargo-ndk 二进制级缓存；android-e2e 补 gradle 缓存 + 模拟器 SDK 组件缓存 + AVD 快照两步流（官方 README 配方）；windows job 的 pnpm 缓存经首跑实测**主动放弃**（PR 运行不执行 pnpm install，store 目录缺失致 setup-node 后置保存红——收益仅 tag 构建 30-60s，见交付说明）；deny 按配方不加缓存（路径过滤改造未做），rustup 可选行未做
5. ✅ develop→main PR 合并（把本批与此前 4 个未同步提交一并带上）——PR #55 已合并；合并后 `origin/main...origin/develop = 12 0`，main 侧 12 个差异提交全为 merge 节点、零内容差异，符合验收口径

**验收**：CI 七 job 全绿；`git rev-list --left-right --count origin/main...origin/develop` 趋近 0（仅剩 merge 节点差异）；AGENTS.md 入库。

**新会话提示词**：

```text
读取 docs/plans/HANDOFF-2026-10-30.md 与 docs/plans/STAGE-PLAN.md 作为上下文，执行 S0（仓库卫生与同步）：
① push 本地 develop（含 .gitignore 更新提交）；
② .gitignore 微调：根级补 dist/，删除冗余行 /target 与错位行 desktop/target/；
③ 未跟踪的 AGENTS.md 按本文档 §AGENTS 规则草案扩充后提交入库；
④ 按 §CI 缓存配方逐项实施 6 项缓存优化；
⑤ develop→main 走 PR 合并。
纪律：Git Flow（feature/* → PR → CI 七 job 全绿 → 合并）；禁止直推 develop/main；每完成一项更新 STAGE-PLAN.md 进度标记。
```

---

## S1a 桌面 GUI：设计 + MVP（会话 #2）

**目标**：桌面壳从"托盘+解锁"扩展为全功能 GUI（对标 1Password 桌面端形态），先落设计与只读 MVP。

**任务**：
1. design_docs/16_桌面GUI.md：
   - **DR-014 决策记录**：GUI 前端选型 Vue 3 + Vite（修订 design_docs/15"桌面零 Node 依赖"闸门；理由：全功能 GUI 的组件化/状态/表单效率；构建链影响评估：桌面 workspace 引入 pnpm 构建、CI 已有 pnpm 基础）
   - 信息架构：主窗（条目列表+搜索框+详情侧栏）、条目编辑器（字段/自定义字段/TOTP/附件）、设置（已有）、导入导出向导、注入启动器（S1c）
   - Tauri command 面映射（CLI 命令 ↔ GUI 操作 ↔ kffi/Vault 调用）：list/get/create/edit/delete/attachment/totp/clip ↔ 列表/详情/编辑/附件/动态码/复制
   - 安全边界：主密码/密码值不落前端存储（SP-F-707 口径）、自动锁与会话集成（复用现有 session.rs 双超时）、剪贴板策略、WindowsApps 之外零网络
2. GUI 脚手架：Vue 3 + Vite 接入 Tauri v2（替换现有静态 ui/，保留托盘/会话/NM 嵌入不动）
3. MVP：解锁集成（复用 session.rs）+ 条目列表 + 搜索 + 详情只读视图（含 TOTP 动态码/复制按钮）
4. 测试：组件测试 + 现有 session 测试回归；CI 七 job 全绿

**验收**：MVP 可解锁/浏览/搜索真实库；design_docs/16 入库；DR-014 记录。

**新会话提示词**：

```text
读取 docs/plans/HANDOFF-2026-10-30.md 与 docs/plans/STAGE-PLAN.md（§S1a、§DR-014、§GUI 设计要点）作为上下文，执行 S1a：
① 撰写 design_docs/16_桌面GUI.md（Vue3+Vite 选型与零 Node 闸门修订=DR-014 决策记录、信息架构、组件树、Tauri command 面映射、安全边界：密码不落盘/自动锁/剪贴板策略/零网络）；
② 桌面 GUI 脚手架（Vue3+Vite 接入 Tauri v2，替换现有静态 ui/，托盘/会话/NM 嵌入逻辑保持不动）；
③ MVP：解锁集成（复用 session.rs 双超时会话）+ 条目列表+搜索+详情只读视图（含 TOTP 动态码与复制按钮）；
④ 测试与 CI 七 job 全绿后经 PR 合并 develop。
纪律：Git Flow；禁止直推；机密不进前端存储（SP-F-707 口径）；完成后更新 STAGE-PLAN 进度标记并在 docs/plans/ 追加 S1a 交付说明。
```

---

## S3 Mimosa 基线审计（会话 #4）

**目标**：完整 GUI（含写路径/标签/附件）+ 验收轮修复落地后的干净基线上，做一次完整安全审计并处置全部发现。

**任务**：运行 Mimosa 完整审计（覆盖 CLI/扩展/桌面/**完整 GUI 含写路径**、sp-core::fsperms、kffi Rust-only 面等新增面）→ 逐项处置（修复 / 如实记录豁免理由）→ 复审 → 审计报告入库 docs/reviews/。

**验收**：审计结论全部闭环；新增 HIGH/CRITICAL 发现零遗留或带豁免理由。

**新会话提示词**：

```text
读取 docs/plans/HANDOFF-2026-10-31.md 与 docs/plans/STAGE-PLAN.md（§S3）作为上下文，执行 Mimosa 完整安全审计（/mimosa-security-scan，覆盖 CLI/扩展/桌面/完整 GUI 含写路径/标签/附件全部代码与配置，重点新增面：desktop/ui 前端、sp-kffi Rust-only 面（find_entry_full/delete_entry/list_attachments）、sp-core::fsperms、sp run 注入路径）：逐项处置发现（修复或带理由豁免），审计报告与处置记录入库 docs/reviews/，CI 七 job 全绿后经 PR 合并 develop。注意 HANDOFF §3 坑位清单的 Mimosa 拦截面（cp/sed/git add 目录均拦，用 Write/Edit + 显式路径）。
```

---

## S1b GUI 写路径（会话 #4）

**任务**：条目创建/编辑/删除（含同名策略与 history 语义）、附件 add/list/get（下载到文件/管道）、TOTP 编辑；表单校验与错误呈现对齐 CLI 语义；测试。

**验收**：GUI 完成一条真实条目的全生命周期操作；与 CLI/KeePassXC 互操作回归全绿。

**新会话提示词**：

```text
读取 docs/plans/HANDOFF-2026-10-30.md 与 docs/plans/STAGE-PLAN.md（§S1b）作为上下文，执行 S1b（GUI 写路径）：条目创建/编辑/删除（同名策略与 history 语义对齐 CLI）、附件 add/list/get（含 ACL 限权下载）、TOTP 编辑与展示；表单校验与错误呈现对齐 CLI 退出码语义；组件与集成测试；CI 七 job 全绿后经 PR 合并 develop，更新 STAGE-PLAN 进度标记。
```

---

## S1c 收尾 + 发布（会话 #5）

**任务**：导入/导出 GUI 向导（复用 CLI import/export 语义与三重防护）、注入启动器（从 GUI 以 `sp run` 语义启动应用）、.env 工作流使用文档；v0.4.0 发布前 Mimosa 增量复审；版本统一 bump 0.4.0 + **develop→main PR 同步**（main 自 v0.3.0 后未合入 S1a/S1b/验收轮）→ 从 main 打 tag → draft Release → 维护者签名 → 发布。

**新会话提示词**：

```text
读取 docs/plans/HANDOFF-2026-10-31.md 与 docs/plans/STAGE-PLAN.md（§S1c）作为上下文，执行 S1c：① 导入/导出 GUI 向导（对齐 CLI import/export 语义与三重防护）；② 注入启动器（从 GUI 以 sp run 语义启动应用，秘密不落盘）；③ .env 工作流使用文档（手册 + GUI 内帮助）；④ v0.4.0 发布前 Mimosa 增量复审；⑤ 版本统一 bump 0.4.0、develop→main PR 同步后从 main 打 tag 触发 draft Release、提醒维护者签名发布。纪律同全阶段。完成后更新 STAGE-PLAN 进度标记。
```

---

## S4 Android 批次 2（会话 #6，真机前置）

**任务**：先出任务书（Keystore 包裹、每 boot 主密码 BOOT_COUNT、生物 CryptoObject 绑定、失效回退、SP-F-803 web 真机验证）→ 用户确认 → 分批执行。

**前置**：用户真机可用；SP-F-803 待非 MIUI 真机 + Chrome 复核。

**新会话提示词**（2026-11-01 更新指向最新 HANDOFF）：

```text
读取 docs/plans/HANDOFF-2026-11-01.md 与 docs/plans/STAGE-PLAN.md（§S4、§DR-013、§进度标记）作为上下文，执行 S4（Android 批次 2）。

先决条件核对（不满足则任务书先行、执行挂起）：① 用户真机可用（型号/Android 版本；SP-F-803 复核要求非 MIUI 真机）；② Chrome 可用。

任务：
① 撰写 S4 任务书（docs/plans/ 下，参照任务书_step7 体例）：sp-kffi Keystore 包裹接口（DR-013 批次 2）、每 boot 首次主密码 BOOT_COUNT、生物 CryptoObject 绑定、失效回退主密码、SP-F-803 web 真机验证方案、批次拆分与每批验收标准。
② 绑定面评估：新增 FFI 接口 → UniFFI 绑定重生两步走（HANDOFF §3.6）；给出与 S1d-2 类型化自定义字段（FieldKind，backlog）同批/不同批绑定重生的取舍建议。
③ 任务书交用户确认（重大决策走决策矩阵 + DR 记录），确认后按批次执行。

纪律：Git Flow（feature/* → PR → CI 七 job 全绿 → 合并 develop）；机密/口令不经 Agent；vendor/keepass-ng 与两份 deny.toml 同步维护；Mimosa 拦截按 HANDOFF §3.7 处置。
```

---

## §AGENTS 规则草案（S0 写入根 AGENTS.md）

在现有第 1 条（系统配置/注册表修改前先备份）基础上追加：

2. **分支纪律**：一切变更走 `feature/*|fix/*|docs/*` → PR → CI 七 job 全绿 → 合并；禁止直推 develop/main（2026-10-30 两次事故教训）。
3. **破坏性操作守卫**：读-改-写用户全局状态（注册表/环境变量）的代码，"读取为空/失败"必须中止并报告，禁止按"首次使用"分支覆盖；此类测试前先落盘完整状态备份（2026-10-30 PATH 事故教训）。
4. **签名纪律**：minisign 私钥与口令不经过 CI/Agent/会话记录；签名由维护者在交互窗口执行 `scripts/sign-release.ps1`；口令遗失走 README 轮换流程。
5. **质量门禁**：PR 前 cargo fmt/clippy -D warnings/test 全绿（根+desktop 双 workspace）；文档变更同步 CLI 手册/追溯矩阵/SRS。
6. **会话交接**：新会话先读 docs/plans/HANDOFF-\*.md 最新版与 STAGE-PLAN.md 对应阶段节；阶段完成更新进度标记并追加交付说明。
7. **发布流程**：push tag `v*` → CI 构建 draft Release 并上传资产 → 维护者本地签名后发布；发布打包/签名改动只经 PR。
8. **Mimosa 拦截处置**：源码/配置写入被拦 → 按其建议改写合规形态或内容交用户手动落盘并披露；不静默绕过。
9. **凭据卫生**：源码/示例/测试零真实凭据字面量（fixture 用 sp-test-only 惯例值）；配置只从环境变量或密钥服务读取。

## §CI 缓存配方（S0 实施；依据探索核实 2026-10-30）

| Job | 项 | 每次耗时（估） | 方案 |
|---|---|---|---|
| desktop | cargo install tauri-cli | 8–15 min | `actions/cache` 路径 `~/.cargo/bin`（或仅 tauri-cli.exe），key `tauri-cli-<版本>-<runner 镜像>`；命中跳过安装（rust-cache 虽含 ~/.cargo/bin 但 key 含 Cargo.lock 哈希易失效） |
| windows（tag） | cargo install tauri-cli | 8–15 min | 同上 |
| windows | choco install keepassxc | 2–3 min | 改 KeePassXC 官方 Portable ZIP + `actions/cache` 解压目录，key `keepassxc-<版本>`；`SP_KEEPASSXC_CLI` 指向解压路径 |
| android / android-e2e | cargo install cargo-ndk | 2–5 min | 同 tauri-cli 方案（版本化 key） |
| android-e2e | emulator + system-images 下载 | 2–5 min | `actions/cache`：`/usr/local/lib/android/sdk/system-images*`、`.../emulator`、`.../platform-tools`；key 含 API/target/arch 与 runner 镜像版本 |
| android-e2e | AVD 冷启动 | 2–4 min | emulator-runner 快照两步流：缓存 `~/.android/avd/*`、`~/.android/adb*`，首步生成（无 `-no-snapshot`）、次步 `-no-snapshot-save` + `force-avd-creation: false` |
| android-e2e | Gradle 依赖 | 2–5 min | setup-java 加 `cache: gradle` |
| windows（tag） | pnpm install | 30–60s | setup-node 补 `cache: pnpm` + `cache-dependency-path: extension/pnpm-lock.yaml` |
| deny | 容器+crates 下载 | 2–4 min | 不加缓存：按 cargo-deny-action README 对 `**/Cargo.lock`、`**/Cargo.toml`、`**/deny.toml` 变更过滤触发 |
| 全部 Rust job | rustup 工具链 | 30–90s | 可选（dtolnay `cachekey` 作 key 缓存 `~/.rustup/toolchains`）；收益中等，避免过度工程可不做 |

已被 rust-cache 覆盖（勿重复工程）：cargo registry/git 依赖与 target（各 Rust job）、`~/.cargo/bin`（但受 key 失效影响，见上表）、extension pnpm store（setup-node cache: pnpm）、NDK/build-tools/platforms（runner 预装）。

## §DR-014 预告（S1a 首个交付）

决策点：桌面 GUI 前端从"零 Node 静态 HTML/JS"改为"Vue 3 + Vite"。理由：全功能 GUI（大列表/状态/表单/路由）手写维护成本高；构建链影响：桌面 workspace 引入 pnpm（CI 已有基础）；安全面不变（机密不进前端存储）。备选（Vanilla/Svelte）已在规划轮比较后弃用。S1a 落地时按此模板补全上下文与影响面，写入决策记录。

---

## §进度标记

- [x] S0 卫生与同步（会话 #1）
- [x] S1a GUI 设计+MVP（会话 #2）
- [x] S1b GUI 写路径（会话 #3；S1b-1 + S1b-2（标签/附件/自定义字段 UI）全部完成）
- [x] 验收测试与反馈修复（插入轮，2026-10-31；清单 v1.2 + 修复 PR #57–#69；GUI B 组剩余项与 A10.4 LLM 实战待用户回归）
- [x] S3 Mimosa 基线审计（会话 #4，审计范围含完整 GUI 写路径；Mimosa 扫描 + 人工复审，9 修复 + 12 组豁免，报告入库 docs/reviews/）
- [x] S1c 收尾+v0.4.0 发布（会话 #5，2026-11-01；GUI 导入导出/注入启动器/.env 文档/版本 bump；发布流程：develop→main PR → tag → draft Release → 维护者签名）
- [ ] S4 Android 批次 2（会话 #6，真机前置）
