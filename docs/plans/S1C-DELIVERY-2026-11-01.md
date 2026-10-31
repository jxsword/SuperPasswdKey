# S1c 交付说明（2026-11-01）— GUI 导入导出向导 / 注入启动器 / .env 文档 / v0.4.0

> 本文是 S1c 阶段交付记录。需求口径：SRS V1.12 SP-F-911/912；STAGE-PLAN §S1c。

## 1. 交付内容

1. **GUI 导入向导（SP-F-911）**：两阶段——扫描（复用 sp-cli importer 单源管道，
   不触库不解锁）→ 确认提交（会话 actor 逐条写入 + 单次 save_session）。
   候选值不回传前端（表格仅标题/来源/密码长度，SP-F-707 口径）。五种来源
   （.env/CSV/Bitwarden JSON/目录扫描/环境变量）与 CLI `sp import` 同管道。
2. **GUI 导出对话框（SP-F-911）**：age 加密归档——**导出核心自 sp-cli 下沉
   sp-kdbx::export 单源**（collect/zip/age/自校验），CLI 保留交互壳；GUI 经
   kffi Rust-only 面 `Vault::export_encrypted` 走会话凭据（DR-012 A1 同源，
   零凭据入参），保存对话框 + fsperms 限权收敛。明文导出仅限 CLI 三重防护。
3. **GUI 注入启动器（SP-F-912）**：选条目 + 勾选注入字段 + 程序/参数 →
   `sp run` 语义启动。进程生成经 **sp-cli::spawner 单源**（F8 唯一进程生成点，
   argv 直 exec 无 shell）；env 键 posix_name 净化、受保护变量拒绝（SP-F-301）；
   stdio 继承无遮蔽（对齐 `--no-masking` 口径）；秘密仅存在于子进程环境。
   对话框内含 CLI `.env` 工作流等价写法帮助。
4. **.env 工作流文档**：手册新增 §15（三步工作流 + 要点 + GUI 等价能力）。
5. **版本统一 bump 0.4.0**：根 Cargo.toml / desktop（Cargo.toml + tauri.conf.json）/
   extension（manifest + package.json）。

## 2. 架构决策（随批，详见提交与 DR 引用）

- 导出核心下沉 sp-kdbx：GUI 导出必须拿到 Database——kffi actor 持库 + 会话
  凭据（DR-012 A1）是唯一零密码路径，故 export 核心移入 sp-kdbx（age/zip 依赖
  随之进入 sp-kdbx，Android 侧构建体积略增，功能无影响）。
- 注入启动复用 sp-cli::spawner（design_docs/04"唯一进程生成点"），不在桌面
  另起 Command 构造面。
- GUI 导入不做库内查重（与 CLI import 同口径，DR-016 同名允许）。

## 3. 验证

- 根 workspace 103 / 桌面 37（+3：import/export/launch 门禁与行为）/ GUI 40
  （+3 组件测试）/ 扩展 21 全绿；扩展 lint 通过。
- 新增测试覆盖：导入写入与持久化（锁定重开可见）、导出密文性（不含口令字面量）、
  启动缺字段/受保护变量拒绝/正常 spawn 返回 PID、口令校验、字段选择透传。
- 发布前 Mimosa 增量复审：见 docs/reviews/（本批完成后执行）。

## 4. 发布流程（本批合并后执行）

版本已 bump → PR 合并 develop → **develop→main 同步 PR**（main 自 v0.3.0 后
未合入 S1a/S1b/验收轮/S3/S1d-1/本批）→ 从 main 打 tag `v0.4.0` → CI 构建
draft Release 并上传资产 → **维护者本地签名后发布**（minisign 口令不经 Agent）。

## 5. 遗留与提醒

- 验收清单待用户回归项：GUI B 组剩余项、A10.4 LLM 实战、A2b/A2c（item edit/
  受保护字段）、本批 GUI 导入导出/注入启动（手工项）。
- 类型化自定义字段（FieldKind）= S1d-2，排 S4 之后。
