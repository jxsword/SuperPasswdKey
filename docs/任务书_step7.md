# 任务书_step7：桌面 Windows 批次（Tauri 托盘 + 会话双超时 + 自启 + NM host 嵌入）

## 项目背景（自包含，无需外部上下文）
- 仓库：GitHub 私有仓 jxsword/SuperPasswdKey，main=develop，工作树干净
- 项目：Rust 密码管理器（sp CLI），KDBX4 兼容（keepass_ng 0.11），Argon2id+双因子
- 步骤 6 已完成（PR #28 合入 develop，CI 六 job 绿）：
  - Android M2 保存场景闭环：sp-kffi `save_session`（DR-012 A1，凭据因子留驻 actor）+
    SaveInfo 修复 + SaveConfirmActivity + e2e 断言（锁库拒/保存闭环）；
    design_docs/14 v1.2、SRS SP-F-805、追溯矩阵全同步
  - 决策：DR-013 凭据驻留策略（2026-10-29 拍板）——Android 批次 2 = Keystore 包裹(B2+
    每 boot 主密码，任务 2 真机置后)；**桌面批次 1 = Windows 主线（本任务书）**
  - 桌面评估：docs/reviews/桌面阶段1_穿插评估_2026-10-29.md（P2 协议加密 ≈3 天维持
    backlog；keepass-ng 0.11.12 get_key_elements 仍 pub(crate)，密钥复用开库无动作）
- 桌面现状：sp CLI（crates/sp-cli）+ sp-nmhost（crates/sp-nmhost，native messaging host，
  长驻+进程内会话缓存 300s 空闲锁，DR-006 方案 A）；浏览器扩展（extension/）可用；
  **无 GUI**——用户需求"软件开机自启+首次主密码+过期重新验证"需要交互面
- DR-013 已拍板的桌面口径：Tauri 托盘 GUI（用户选 A）；每次进程启动首次主密码；
  idle/absolute 双超时可配（如 30min 到期重新验证）；无生物、无落盘（不触碰红线 #5 豁免）

## 设计闸门（已决，实现时如发现冲突先改 design_docs 再改码）
- **单进程架构**：Tauri 壳内嵌 NM host——sp-desktop.exe 同时是托盘 GUI 进程与 native
  messaging host（注册表 manifest 指向它）：会话进程内闭环，免跨进程会话共享/IPC，
  收敛 DR-006 遗留"跨进程持久化缓存 backlog"；sp-nmhost 的 protocol.rs（信封编解码/
  action 分发）库化复用，不复制协议代码
- **前端零 Node 依赖**：Tauri v2 + 静态 HTML/JS（frontendDist 指向静态目录，不引 npm
  构建），托盘+解锁窗+设置窗足够；技术选型保持与基线 M4"Tauri 桌面"一致
- 会话模型：解锁 → CredentialFactors 留驻桌面进程（Zeroizing，Rust 侧，无 JNI 问题）；
  idle 超时与 absolute 超时独立可配（默认 idle=300s 对齐 DR-006，absolute=1800s）；
  到期锁库（drop 清零），再次使用须重新输入主密码；托盘菜单显示会话状态/立即锁定/设置
- 机密纪律：主密码/因子只在 Rust 侧（Tauri command 边界即用即弃或进 Zeroizing 状态），
  前端 JS 不接触明文凭据；设置文件只存非机密（超时配置/自启开关），不落任何凭据

## 任务 0：环境自检（会话开始快速自检，任何异常先修再开工）
- 本机：Windows x64；WebView2 Runtime（Win11 自带，`reg query "HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv` 或让 Tauri 构建时自检）；JDK 17；Rust MSVC（rustup show）
- Tauri CLI：`cargo install tauri-cli --locked`（若未装；≈3-5min 编译）——静态前端路线
  不需要 Node/npm
- 自检命令（Git Bash）：
  cargo --version && rustc --version
  reg query "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" /v pv 2>/dev/null | grep -i pv || echo "WebView2 未检出（Tauri 构建自检兜底）"
  cargo test --workspace 2>&1 | grep -cE "^test result: ok"
  git status -sb && git log --oneline -3

## 任务 1：sp-desktop 脚手架（M7a-1）
- 新建 `desktop/`（独立 cargo workspace，**不并入根 workspace**——tauri/wry/tao 仅
  MSVC 可构建，避免 linux CI 失败；根 Cargo.toml 若为显式 members 则天然隔离，须验证）
- Tauri v2：cargo tauri init（identifier com.superpasswdkey.spdesktop）；静态前端
  desktop/ui/；窗口策略：主窗口隐藏、托盘常驻、解锁窗按需弹出
- 会话管理器（Rust 侧状态机）：unlock(password, keyfile?) → open_db + 因子留驻；
  touch()；idle/absolute 双定时；lock()（drop 清零）；状态查询 command 供前端
- 出口：cargo tauri dev 起托盘；解锁真实 kdbx（复用 sp CLI 建的库）；锁库/超时行为正确

## 任务 2：设置持久化 + 开机自启（M7a-2）
- 设置文件：%APPDATA%/SuperPasswdKey/settings.json（仅非机密：idle_ms/absolute_ms/
  autostart）；读写用 serde_json；损坏时回落默认值不崩
- 自启：HKCU\Software\Microsoft\Windows\CurrentVersion\Run 写/删（用户级免管理员，
  tauri-plugin-autostart 或手写 registry——手写优先，少一个依赖面）
- 出口：改设置重启生效；自启开关注册表可验证（reg query）

## 任务 3：NM host 嵌入（M7a-3）
- sp-nmhost 重构：protocol.rs/信封分发抽为 lib（crates/sp-nmhost 加 lib target，
  bin 薄壳保留兼容现有注册与 CI）；sp-desktop 依赖该 lib，stdin/stdout NM 循环跑在
  专用线程，与会话管理器同进程共享（DR-013 单进程架构）
- 注册脚本：native-host/ 增加 sp-desktop 注册模式（manifest 指向 desktop 产物），
  现有 sp-nmhost 注册保持可用（迁移期双轨）
- 解锁交互：扩展请求到达且会话锁定 → NM 回 locked 语义 → 托盘气泡/弹解锁窗；
  首次启动（开机自启）主密码验证在解锁窗完成（用户需求"首次打开必须主密码"）
- 出口：扩展 → sp-desktop 全链路填充可用；锁定态请求行为正确；旧 sp-nmhost 回归

## 任务 4：门禁与出口
- 本地：cargo fmt --all -- --check；cargo clippy --workspace --all-targets -- -D
  warnings（含 desktop crate——若其独立 workspace 则单跑）；cargo test --workspace
  （73+ 基线不回退）；cargo deny check；desktop 独立 cargo check/test
- CI：新增 desktop job（windows-only：tauri build + clippy + test）；六 job 保持绿；
  android-e2e 不受影响
- 出口标准：① 托盘 GUI 解锁/锁定/双超时/自启/设置持久化验收绿；② 扩展经 sp-desktop
  填充链路通（模拟 NM 握手测试 + 手工 Chrome 验证）；③ design_docs/15（桌面形态）+
  SRS SP-F-9xx + 追溯矩阵同步；④ CI 全绿
- 开发模型：Git Flow（feature/step7-* 分支 → PR → develop）；Conventional Commits；
  设计不符先改 design_docs 再改码

## Android 批次 2（本任务书不含，另出任务书）
- 范围：sp-kffi Keystore 包裹接口（Kotlin 侧包裹/取回，Rust 侧喂因子）、每 boot 首次
  主密码（BOOT_COUNT）、生物 CryptoObject 绑定、失效回退；前置=用户真机可用（任务 2
  已置后）+ 批次 1 出口

## 约束
- 红线十条（docs/SRS.md §7）不回退；桌面批次不触碰 DR-013 红线 #5 豁免（那是 Android
  生物路径专属）；不改 sp-core/sp-kdbx 既有公开 API（sp-nmhost 增量/库化允许）
- 机密不落设置文件/日志；前端 JS 不接触明文凭据（Tauri command 边界即用即弃）
- 不做：macOS 批次（延后）、Android 生物（批次 2）、P2 协议加密（backlog）、密钥复用
  开库（等 upstream）、宽匹配、iOS、多库

## 已知坑位清单（步骤 4/5/6 会话实测，直接复用结论）
- 本机（Windows）：Git Bash 调 adb 加 MSYS_NO_PATHCONV=1；SDK E:\dev\android-sdk；
  cargo-ndk 4.x `-p` 须放 `--` 之后
- CI：targets 用 `rustup target add` 装到锁定工具链（dtolnay action 的 targets 参数
  会装到 stable）；android-e2e job 须先清 dotnet/ghc/CodeQL/旧 NDK 并 docker prune，
  android-emulator-runner 显式 disk-size=2048M + cores=4，**script 逐行独立 `sh -c`
  执行——不能反斜杠续行/多行 for-if 结构，长命令单行写**
- UniFFI Kotlin：Record 字段 snake→camelCase；错误枚举→sealed FfiException.*；Rust
  `impl Default` 文档注释不进生成绑定
- Android UI 自动化：锁定态聚焦密码框触发本应用认证数据集抢键（先关 autofill 服务或
  先解锁）；**UIAutomator 逐键注入在 Gboard 组字态下丢字符（敏感字段先 settings put
  secure autofill_service null 再整串 input text）；Gboard 对 keyevent MINUS 错译
  字符（含 `-` 的口令禁用逐键 keyevent）**；FillResponse.setAuthentication 的
  presentation 在 API 33+ 强制非空；**填充响应不带 SaveInfo 则系统永不发起 onSaveRequest
  （保存链路硬前提，SP-F-805）**；API 34 起 adb shell 无法启动非导出组件
- Android（步骤 6 新增）：UnlockActivity noHistory——BACK 直接收掉页面，勿用返回键导航；
  保存演示写入 vault.kdbx 后 TestActivity（断言 2 条目）需先重推 fixture
- 模拟器遗留状态：autofill 服务已配置；files/ 残留 result.txt/test_vault.kdbx（无害）
- 跨平台：apkm 下载需过 Cloudflare；x86_64 模拟器跑 arm 应用需 playstore 镜像
- Tauri（step7 预研口径，实现时补充）：静态前端路线无 Node 依赖；tauri v2 Windows
  仅 MSVC 工具链；desktop/ 独立 workspace 防 linux CI 失败
- 任何 Mimosa 拦截：内容交用户手动落盘，不静默绕过（步骤 6 曾误伤 intent extra 键名
  字符串——非机密常量重命名即合规，须在报告披露）
