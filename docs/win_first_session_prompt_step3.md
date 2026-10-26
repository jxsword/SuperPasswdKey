# SuperPasswdKey 步骤 3：浏览器扩展（F1，native messaging + 本填充引擎）

> 本文件是给 Windows 工作区 ZCode 会话的首次启动提示词。直接把下方代码块内容
> 粘贴为该会话的第一条消息即可。生成于 2026-10-27（WSL 侧会话，步骤 2 完成后）。

## 项目背景（自包含，无需外部上下文）
- 本机仓库 = GitHub 私有仓 jxsword/SuperPasswdKey 的 clone，基于 main（v0.1.2-linux 已打 tag）
- 项目：Rust 密码管理器 CLI（`sp`），KDBX4 兼容（keepass_ng 0.11），Argon2id+双因子
- Cargo workspace：crates/sp-core（密钥策略/条目模型/错误模型）、sp-kdbx（KDBX4 适配）、sp-cli（命令族 sp init/item/read/clip/run/inject/import/export；含 lib target 供外部复用）
- 已完成：v0.1.1 Windows 全绿（F5/F8/F9/F10）；v0.1.2-linux Linux 全绿 + `sp run --stdin`（gopass 语义）+ CI 三 job（windows/linux/deny）+ zip deflate-only 依赖收紧
- 本任务是路线图步骤 3（SRS §8 后续路线 / design_docs/08 §5）：浏览器扩展 F1 —— native messaging host 复用 sp-core + 本填充引擎
- 必读：docs/SRS.md（注意 §2.3 目前把 F1 列为非目标/仅接口预留，需先升级为正式需求）、design_docs/00（平台差异集中在 platform 模块）、03、09（安全实现方案）、docs/decision_log.md（DR-001~005）、docs/research/分报告D（CLI 与机密管理竞品）、基线 B4/F1 验收项
- 开发模型：Git Flow（feature/browser-ext 分支 → PR → develop）；提交 Conventional Commits；发现设计文档与实现不符：先改 design_docs 再改码，提交注明 docs(design)

## 任务 0：环境核实与补齐（Windows 侧）
1. 工具链：rustc/cargo（rust-toolchain.toml 锁 1.99.1，Windows 侧已就绪）、Node ≥20 + pnpm（扩展构建）、Chrome 与/或 Edge（native messaging 实测）、web-ext（lint，可选）
2. 缺什么补什么；确认 `cargo test --workspace` 在 Windows 仍全绿（基线回归）
3. 检查 git 身份与 gh auth status（PR 需要）
4. ⚠️ 若本会话装有 Mimosa 安全钩子：写入含 Command::new(变量) 的文件会被拦截——先例：spawner.rs 由用户手动落盘；遇同样拦截把内容交给我，不静默绕过

## 任务 1：设计先行（design_docs/1x 系列，SRS 扩展）
1. 新增 design_docs/10_浏览器扩展_总体.md、11_native_messaging协议.md、12_填充引擎与表单匹配.md（编号续 1x，见 design_docs/08 §5）
   - 协议参照 KeePassXC-Browser 公开协议（keepassxc-protocol.md，公开文档 URL 实测可达后引用）
   - host 进程：复用 sp-core/sp-kdbx（候选形态：`sp` 新子命令长驻 stdin/stdout JSON-RPC vs 复用一次性命令——两方案给决策矩阵，用户拍板后写 DR-006）
   - 安全边界（对齐 09/SP-F-303）：native messaging manifest 的 allowed_origins 绑定扩展 ID（防任意网页应用调用）；host 不提供"导出全库"能力给扩展端；解锁状态机沿用 03 §3（无跨进程缓存的现状下，扩展端每命令解锁 or 引入会话缓存 → 与 v0.2 backlog"会话缓存决策（keyring/加密 state）"合并为一个 DR）
   - manifest v3 约束（service worker 生命周期）对协议长连接的影响必须写明
2. SRS 更新：F1 从非目标转为正式需求，新增 SP-F-1xx/7xx 编号段（先查追溯矩阵取空段），更新 requirements_traceability_matrix.md；重大决策按全局规则写 decision_log（DR-006 起）
3. 设计自查过 review.md 补录（沿用既有格式）

## 任务 2：实现（feature/browser-ext 分支）
1. `native-host/`（或 crates/sp-nmhost）：Rust native messaging host（stdin/stdout 长度前缀 JSON），复用 sp-core 解锁与条目读取；Windows 注册表注册脚本（HKCU\Software\Google\Chrome\NativeMessagingHosts）+ 注销
2. `extension/`：TypeScript WebExtension（manifest v3）——content script 表单探测与填充（用户名/密码/TOTP 可选填充）、popup 列表与搜索、background service worker 与 host 通信
3. 填充引擎最小验收（基线 B4）：登录表单单页填充可用；iframe/SPA 场景与"自动保存"列 backlog 不进本期
4. 全程遵守红线十条（docs/SRS.md §7 SP-C-101）：机密不落盘临时文件、不进 localStorage；扩展端内存值用后即清

## 任务 3：测试与 CI
1. host：单测（协议编解码/长度前缀边界/非法输入不 panic）+ fuzz 短跑（fuzz/ 新目标，结果入 docs/reviews/）
2. extension：vitest 单测（表单探测规则）+ Chrome 实测手工清单（写入 docs/reviews/）
3. CI：host 属 Rust workspace 则自动进三 job；extension 构建进 CI（pnpm build + lint），不发商店
4. 出口：本地全绿 + GitHub Actions 三 job 全绿 + Chrome 手工填充验收单签字

## 验收（完成后汇报）
- design_docs/1x 定稿 + SRS/追溯矩阵更新 + DR-006 记录
- Chrome 上完成一次真实登录填充（demo 站或本地静态页）
- push 后 CI：windows + linux + deny 全绿；新 tag `v0.2.0-ext`（develop → main 后打，发布动作请先向我确认）

## 约束
- 红线十条（docs/SRS.md §7）不回退；测试向量来源锁定（SP-C-106）
- v0.2 backlog 的 1pux 解析器、指纹去重第三级增强：不在本步骤范围
- 不做：服务器轨、Android、多库、iOS（SRS §2.3 非目标）
- 任何 Mimosa 拦截：内容交用户手动落盘，不静默绕过
