# S1a 交付说明 — 桌面 GUI：设计 + MVP（会话 #2，2026-10-30）

> 输入：[HANDOFF-2026-10-30.md](HANDOFF-2026-10-30.md) + [STAGE-PLAN.md](STAGE-PLAN.md) §S1a / §DR-014 / §GUI 设计要点。
> 本文档是 S3（Mimosa 基线审计）会话的输入之一；下会话仍以 HANDOFF 为第一入口。

## 执行摘要

| 任务 | 状态 | 说明 |
|---|---|---|
| ① design_docs/16_桌面GUI.md | ✅ | DR-014 决策记录（Vue3+Vite 选型与零 Node 闸门修订）、信息架构、组件树、Tauri command 面映射、安全边界（密码不落盘/自动锁/剪贴板策略/零网络） |
| ② GUI 脚手架 | ✅ | desktop/ui 静态页 → Vue3+Vite+TS（package.json/vite/tsconfig/src/components）；tauri.conf.json 接 frontendDist/devUrl/beforeDev/beforeBuild + 窗口 1024×720；desktop Cargo.toml 加 arboard |
| ③ MVP | ✅ | 解锁集成（复用 session.rs 双超时）+ 条目列表 + 搜索 + 详情只读（TOTP 动态码 30s 轮询 + 复制按钮）；新增 Rust command：entry_list/entry_get/entry_totp/clip_copy |
| ④ 测试 + CI | ✅ | Rust session 测试 20 绿（新增 7 个 GUI 方法测试）+ vitest 组件测试 15 绿；CI desktop job + windows(tag) job 补 pnpm install/test |

## ① 设计文档（design_docs/16）要点

- **DR-014 决策记录**：前端选型 Vue 3 + Vite + TypeScript（修订 design_docs/15"静态零 Node"闸门）。理由：全功能 GUI 的组件化/状态/表单效率；构建链影响可控（CI 已有 pnpm 基础）；Vue 为 Tauri 官方模板一等公民。安全面不变（机密不进前端存储，SP-F-707 口径延续）。
- **信息架构**：单窗口（托盘常驻，关窗=隐藏），解锁态=主界面（列表+搜索+详情侧栏），锁定态=解锁面板，顶栏=设置入口。视图清单标注 S1a/S1b/S1c 分期。
- **组件树**：App → UnlockPanel / MainView（SearchBox+EntryList+DetailPanel）/ SettingsPanel / TopBar。MVP 用组件内 ref/computed + useSession composable，不引入 Pinia。
- **Tauri command 面映射**：session_status/unlock/lock_now/touch/get_settings/save_settings（复用）+ entry_list/entry_get/entry_totp/clip_copy（新增）↔ sp-kffi Vault 调用。
- **安全边界**：密码不落前端存储（零 localStorage/sessionStorage/IndexedDB/日志）；自动锁复用 session.rs 双超时（惰性检查）；剪贴板走 Rust clip_copy（arboard+30s 超时清空，对齐 CLI clip）；零网络（CSP default-src 'self' 不变）。

## ②③ 实现要点

- **Rust**：session.rs 新增 `EntrySummaryDto`/`EntryDetailDto`（serde Serialize，字段与前端 api.ts 镜像）+ `Session::entry_list/entry_get/entry_totp`（经 `active_vault()` 惰性锁库，锁定态返回"会话已锁定，请重新解锁"）；main.rs 新增 4 个 command 并注册。`active_vault` 的 `#[allow(dead_code)]` 移除（GUI+NM 共用）。
- **前端**：api.ts（invoke 封装 + 类型镜像）、useSession composable（5s 轮询 + touch）、App.vue（视图切换）、UnlockPanel/MainView/SearchBox/EntryList/DetailPanel/SettingsPanel。密码字段掩码展示（●●●●）+ 复制按钮（不回显明文）。TOTP 30s 轮询（DetailPanel 内 setInterval，unmount 清理）。
- **剪贴板**：clip_copy 用 arboard 写入 + 后台线程 30s 清空（不阻塞 UI）；前端不直接用 navigator.clipboard。
- **构建链坑**：tauri beforeBuildCommand 的 cwd = `dirs.frontend`（= src-tauri 父目录 `desktop`），故路径用 `pnpm --dir ui build`（非 `../ui`）；pnpm 11 默认拦截 esbuild 构建脚本，需 `pnpm-workspace.yaml` 的 `allowBuilds: esbuild: true`（对齐 extension 同款配置）。

## ④ 测试与 CI

- **Rust**（desktop workspace，cargo test 20 绿）：新增 entry_list_after_unlock_returns_summaries / entry_list_locked_returns_locked_error / entry_get_returns_detail_with_password / entry_get_missing_returns_not_found / entry_totp_generates_code / entry_totp_without_otp_errors / entry_list_expired_session_locks_lazily。Fixture 增补 OtpEntry（otpauth:// URL）。
- **前端**（vitest 15 绿）：EntryList（渲染/选中/TOTP 标记/空态/URL 提取）/ MainView（加载/搜索过滤/点击详情）/ DetailPanel（字段渲染/密码掩码/TOTP 轮询/复制/锁定错误）/ UnlockPanel（空密码/解锁/失败）。
- **CI**：desktop job 补 pnpm/action-setup + setup-node（cache: pnpm，cache-dependency-path: desktop/ui/pnpm-lock.yaml）+ pnpm install --frozen-lockfile + pnpm test；windows(tag) job 在桌面 bundle 构建前补 desktop/ui pnpm install（beforeBuildCommand 需 node_modules）。

## 验收对照

- [x] MVP 可解锁/浏览/搜索真实库（组件测试 + Rust 测试覆盖；真实库手工验收待用户）；
- [x] design_docs/16 入库；DR-014 记录入库（docs/decision_log.md）；
- [x] SRS V1.5（SP-F-901 修订 + 新增 SP-F-908）+ 追溯矩阵同步；
- [x] CI 七 job 全绿后经 PR 合并 develop（合并记录见 PR）。

## 备注

- 真实库端到端手工验收（解锁真实 kdbx → 列表 → 搜索 → 详情 → TOTP 复制）待用户执行；自动化测试已覆盖语义链路。
- GUI 写路径（条目 CRUD/附件/TOTP 编辑）在 S1b；导入导出/注入启动器在 S1c；Mimosa 完整审计在 S3（GUI MVP 落地后的干净基线）。
- pnpm-lock.yaml 与 pnpm-workspace.yaml 入库（CI --frozen-lockfile 依赖）。

## 下一阶段指针

S3（Mimosa 基线审计）按 STAGE-PLAN §S3 执行，覆盖 CLI/扩展/桌面/GUI MVP 全部代码与配置。
