# SuperPasswdKey 步骤 2：Linux（WSL ubuntu2604）移植与测试

> 本文件是给 WSL ubuntu2604 中 ZCode 会话的首次启动提示词。直接把下方代码块内容
> 粘贴为该会话的第一条消息即可。生成于 2026-10-26（Windows 侧会话）。

## 项目背景（自包含，无需外部上下文）
- 本机仓库 = GitHub 私有仓 jxsword/SuperPasswdKey 的 clone，基于 develop（v0.1.1）
- 项目：Rust 密码管理器 CLI，KDBX4 兼容（keepass-ng 0.11），Argon2id+双因子
- Cargo workspace：crates/sp-core（密钥策略/条目模型/错误模型）、sp-kdbx（KDBX4 适配）、sp-cli（命令族 sp init/item/read/clip/run/inject/import/export）
- v0.1.1 已在 Windows 全绿发布（24+ 测试）；本任务是路线图步骤 2：Linux 全测试通过 + CI 增加 linux job
- 必读：docs/SRS.md（需求 V1.0）、design_docs/02+03+04（适配层/CLI/注入设计）、design_docs/07（测试与CI）、docs/decision_log.md（DR-001~005，注意 DR-005 的三个互操作坑）
- 开发模型：Git Flow（feature/linux-port 分支 → PR → develop）；提交 Conventional Commits

## 任务 0：环境核实与补齐
1. `cat docs/env/WSL_ubuntu2604_环境检查报告.md` 了解既知状态；检查工具链：
   rustup（rust-toolchain.toml 会自动装 1.99.1）、gcc/make/pkg-config、keepassxc-cli（apt 源 2.7.10）、age、minisign、cargo-deny、cargo-fuzz+nightly、docker
2. 缺什么就执行幂等脚本：`sudo bash docs/env/wsl_ubuntu2604_setup.sh [段名|all]`
   （网络慢时 WSL 代理 = http://127.0.0.1:10808）
3. 检查 git 身份（git config user.name/email）与 gh auth status（推送 PR 需要；未登录则提示我手动 gh auth login）
4. ⚠️ 若本会话也装有 Mimosa 安全钩子：写入含 Command::new(变量) 的文件会被拦截
   ——先例与处置：Windows 侧 spawner.rs 由用户手动落盘；遇同样拦截就把内容交给我

## 任务 1：Linux 适配修复（feature/linux-port 分支）
1. crates/sp-kdbx/tests/roundtrip.rs：keepassxc-cli 路径硬编码 Windows 值
   → 按 cfg!(windows) 选择 /usr/bin/keepassxc-cli，保留 SP_KEEPASSXC_CLI 语义
2. crates/sp-cli/src/commands.rs restrict_perms：POSIX 0600 分支实测（chmod），
   design_docs/03 平台表逐项核对
3. f8 run 增加 --stdin 变体（gopass 语义：机密改走 stdin，规避 /proc/environ，
   design_docs/04 §4）——Linux 专属增强
4. 检查 .gitattributes 生效（全部 LF）；cargo build 零警告零错误
5. 发现设计文档与实现不符处：先改 design_docs 再改码，提交注明 docs(design)

## 任务 2：测试全绿
- cargo test --workspace 全绿（round-trip 对拍用 keepassxc-cli 2.7.10）
- 性能测试保持 #[ignore]；clippy -D warnings；cargo fmt；cargo deny check
- proptest/fuzz：cargo-fuzz 短跑解析器（nightly 已装）—— fuzz 结果记录进
  docs/reviews/，发现崩溃即修复

## 任务 3：CI 增加 linux job
- .github/workflows/ci.yml 增加 linux job（ubuntu-latest：apt install keepassxc、
  rust-cache、fmt/clippy/test/deny 同 windows；cargo-fuzz 不进 CI）
- feature 分支 PR → develop；develop → main 后打 tag v0.1.2-linux

## 验收（完成后汇报）
- 本地 cargo test --workspace 全绿（含 round-trip 对拍）
- push 后 GitHub Actions：windows + linux + deny 三 job 全绿
- 文档同步：design_docs/02/03 平台表更新，review.md 补录

## 约束
- 红线十条（docs/SRS.md §7 SP-C-101）不回退；测试向量来源锁定（SP-C-106）
- 任何 Mimosa 拦截：内容交用户手动落盘，不静默绕过
- 不做：F1 扩展、服务器轨、Android、多库（SRS §2.3 非目标）
