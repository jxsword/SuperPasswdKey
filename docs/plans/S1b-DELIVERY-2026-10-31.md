# S1b 交付说明 — 桌面 GUI 写路径（含标签/附件/自定义字段；会话 #3，2026-10-31）

> 输入：[HANDOFF-2026-10-30.md](HANDOFF-2026-10-30.md) + [STAGE-PLAN.md](STAGE-PLAN.md) §S1b（本会话按用户确认的调整计划执行：S1b 提前、S3 挪后，S1b 拆批）。
> 本文档是 S3（Mimosa 基线审计）会话的输入之一；下会话仍以 HANDOFF 为第一入口。

## 执行摘要（五个 PR，全部 CI 绿合并 develop）

| PR | 内容 | 说明 |
|---|---|---|
| #59 | **CI 分区触发改造（DR-015）** | dorny/paths-filter（SHA 钉版）六路过滤，七 job 按依赖关系条件触发；tag 全量豁免；skipped 视为通过。实测收益：纯文档 PR 仅跑 changes（#61 验证），crates/desktop 变更跳过 extension，仅 android 改动跳过全部 Rust job |
| #61 | **sp.exe 同名撞车注意事项** | CLI 手册 §1（三步核对法 + PATH 调序处置）+ design_docs/16 §8.2——本 CLI 与 win-pyvideotrans v4.17 的 GUI 应用同名 sp.exe |
| #60 | **S1b-1 详情交互增强 + 条目写路径** | 密码行 ▾ 下拉（显示/放大显示/复制）+ LargeTypeModal；entry_create/entry_edit/entry_delete（save_session 零凭据保存）；sp-kffi Rust-only 面（FfiEntryFull 含 custom_fields 读回 + delete_entry，Kotlin 绑定零漂移）；custom_fields 编辑保真 |
| #62 | **S1b-2a 标签全栈（SP-F-108）** | KDBX4 原生 Tags：EntryModel/CLI --tag/list --tag/kffi 四类型（**Kotlin 绑定重生入库**）/GUI 表单+徽标过滤+详情行+搜索匹配 |
| #63 | **S1b-2b 附件管理（SP-F-910）+ 自定义字段编辑 UI** | 详情页附件区（list/add/download：tauri-plugin-dialog Rust 侧对话框 + fsperms 限权）；EntryFormModal 字段行增删 + 保留名校验；sp-core::fsperms 限权单源下沉 |

## 测试基线（本会话末）

- 根 workspace：sp-cli 34 / sp-kffi 15（+5：full 读回/删除/附件列表/标签 roundtrip）等全绿
- desktop：29 绿（+9：CRUD/标签/附件/保真）
- vitest：29 绿（+14：表单/下拉/放大/删除/附件/标签/字段行）
- 双 workspace fmt/clippy -D warnings 全绿；tauri build 全量三连（每 PR desktop job 实测）

## 过程坑位（下会话高频注意）

1. **keepass-ng additional_attributes 为 HashMap 存储**：custom_fields 读回顺序跨平台/跨进程不确定——断言必须按名查找/集合比较（linux CI 实测与 Windows 相反）。
2. **protected 标记 v0.1 不持久化**（DR-005）：读回恒 false；GUI 自定义字段 UI 恒 false 并如实披露。
3. **UniFFI 面变更 → Android 构造点全量适配**：FfiEntryDraft 加 tags 后，Kotlin 侧全部构造点（SaveConfirmActivity/TestActivity）需补参数——android job drift check 只保证绑定一致，app 侧编译由 android-e2e 把关；本会话两次红都在此。
4. **绑定重生的字节级对齐**：Windows 本地 uniffi-bindgen 生成 + Edit 落盘与 CI linux drift check 逐字节一致（含空行数）；Mimosa 拦 cp 落盘源码文件——用 Edit 按 diff hunks 精准落盘。
5. **tauri beforeBuildCommand cwd = src-tauri 父目录**（S1a 已知，本轮未再踩）。

## 安全与语义口径

- 机密纪律未破：表单/详情密码瞬态传递、不落前端存储；写路径全部经 save_session 零凭据保存（DR-012 A1）；附件下载限权与 CLI 单源（sp-core::fsperms）。
- 校验语义与 CLI 同源：标题必填/otp otpauth:// 前缀/同名允许/标签 trim 非空/保留字段名拒绝。
- 编辑 history 语义 = sp-kdbx edit_entry 单源（SP-F-103）。

## 验收对照（对照 STAGE-PLAN §S1b）

- [x] S1b-1：交互增强 + 创建/编辑/删除（custom_fields 保真）→ PR #60
- [x] S1b-2a：标签全栈 → PR #62
- [x] S1b-2b：附件 + 自定义字段 UI → PR #63
- [x] 文档同步：SRS V1.8（SP-F-108/909/910）、矩阵、CLI 手册（--tag + 撞名注记）、design_docs/16 v1.4、DR-015
- [x] CI 全绿合并（五 PR）；STAGE-PLAN 进度标记更新
- [ ] 真实库 GUI 手工验收（含 TOTP/附件下载 icacls 核对）待用户——CLI 侧 `sp item` 面回归随 CI 全绿

## 下一阶段指针

S3（Mimosa 基线审计）按 STAGE-PLAN §S3 执行——审计范围为 CLI/扩展/桌面/完整 GUI（含写路径/标签/附件）；注意 HANDOFF §3.7 的 Mimosa 拦截处置约定（本轮新增：cp/ln 等落盘源码的 Bash 形式同样被拦，用 Write/Edit 落盘）。
