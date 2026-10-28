# SuperPasswdKey 步骤 6：Autofill 保存场景 + 真机回填（M2 方向启动）

> 本文件是给 Windows 工作区 ZCode 会话的首次启动提示词。直接把下方代码块内容
> 粘贴为该会话的第一条消息即可。生成于 2026-10-28（M1 出口合入后，develop=a9407d9；
> 本任务书随 CI e2e job 的 PR 一并入库，会话开始时以 git log 实际 HEAD 为准）。

```text

## 项目背景（自包含，无需外部上下文）
- 仓库：GitHub 私有仓 jxsword/SuperPasswdKey，main=develop，工作树干净
- 项目：Rust 密码管理器（sp CLI），KDBX4 兼容（keepass_ng 0.11），Argon2id+双因子
- 步骤 5 已完成（M1 正式化出口，PR #25 全绿合入；出口报告
  docs/reviews/m1_出口报告_2026-10-28.md）：
  - crates/sp-kffi：UniFFI 0.32 绑定（create_db/open_db/find_entry/list_entry_summaries/
    entry_totp_now/add_entry/edit_entry/save/add_attachment/get_attachment/lock/
    rank_entry_urls/calibrate_kdf）。keepass_ng Database !Send+!Sync → 线程封闭 actor
    （design_docs/13 §2）。FfiKdfParams::default 已降级为"跳过校准"兜底（DR-011）
  - android/（Gradle 8.13 + AGP 8.7.3 + Kotlin 2.1.0 + JNA 5.17.0@aar，无 AndroidX）：
    MainActivity（产品壳入口+状态面板）/ UnlockActivity（解锁/首建双模式，首建走
    VaultSession.createVault=calibrate_kdf→create_db→unlock，DR-010/011 终态）/
    SpAutofillService（webDomain+包名匹配 Dataset、认证回路 presentation 已修——
    API 33+ 强制非空）/ DemoLoginActivity / DemoWebActivity / TestActivity（e2e
    harness：全套 FFI 断言 + init_vault 校准链路 + idle_timeout_ms 超时注入）/
    SpApplication（进程级加载 cdylib）/ VaultSession（单例：300s 空闲锁库+懒检查，
    idleTimeoutMs/nowMs 注入点；库路径单一来源 VAULT_NAME="vault.kdbx"）
  - CI：六 job（windows/linux/deny/android/extension + android-e2e 模拟器 KVM 跑
    TestActivity 断言集，fixture 口令 sp-test-only 属 SRS §5 允许入仓的测试专用值）
  - 文档：design_docs/13 v1.0 / 14 v1.1、SRS V1.2（SP-F-801~804）、DR-009~011、
    追溯矩阵（SP-F-802 🟢 / 803 🟡 web 路径开放）
  - 模拟器实测（Pixel_7_API34，非真机口径）：e2e 全绿；校准建链路 3.6s（快机
    t=2 触底）；2000ms 注入锁定 PASS；UI 回归（解锁/首建/错误呈现/认证回路）全通
- 遗留开放：
  - R14-1/3：web 字段 autofill 需 Play Chrome + 非 MIUI 真机（Chrome on MIUI 零
    onFillRequest=ROM 门禁；证据链 android_真机安装测试方案 §8–§12）
  - 真机校准分布：中低端机 t 落点未回填（模拟器/快机均触底=2）
  - R13-1：JNI String 不可零化（用户文档边界声明维持）

## 任务 0：环境自检（会话开始快速自检，任何异常先修再开工）
- 本机：Windows x64；SDK E:\dev\android-sdk（显式 export 或绝对路径）；NDK
  r28.2.13676358；JDK 17；cargo-ndk 4.1.2；AVD：Pixel_7_API34、Pixel_7_play
- 自检命令（Git Bash）：
  export PATH="$PATH:/e/dev/android-sdk/platform-tools"
  adb version | head -1
  rustup target list --installed | grep android
  E:/dev/android-sdk/emulator/emulator.exe -list-avds
  cargo test --workspace 2>&1 | grep -cE "^test result: ok"
  git status -sb && git log --oneline -3

## 任务 1：Autofill 保存场景（M2 主线，设计先行——含 DR-012 决策闸门）
- 现状：SpAutofillService.onSaveRequest 直接 onSuccess 吞掉；add_entry/save FFI 已有
- **设计闸门（重大决策，先出矩阵后写码，结论写 DR-012）**：save_atomic 需要凭据因子，
  而 VaultSession 只持 Vault 句柄不存主密码（红线 #5）。两个方向：
  A(会话内缓存 CredentialFactors——照搬 sp-nmhost DR-006 方案 A 的 SecVec+空闲清零
    模型，Android 端为内存对象非跨进程；保存零输入，UX 最好，但红线边界扩到移动端
    需论证威胁模型等价性); B(保存时要求用户输入主密码确认——零新增攻击面，UX 多一次
    输入)。用户拍板后才进入实现。
- 实现面（设计定稿后）：onSaveRequest 解析 FillContext 表单值 → 保存确认 Activity
  （显示 host/包名+用户名，密码不回显）→ 归属判定（webDomain/包名，同解锁匹配口径）
  → 去重（同 host+用户名已存在：更新/跳过策略）→ add_entry + save → 完成回调
- 验收：模拟器演示链路（DemoLoginActivity 输入新凭据 → 系统保存建议 → 确认入库 →
  重开库能读到）；不保存路径（FLAG_DO_NOT_SAVE/拒绝）行为正确；e2e 断言进 TestActivity
- 文档：design_docs/14 §6.4 backlog 升正式节（v1.2）、SRS SP-F-805 或并入 803（按
  software-requirements-engineer 拆分习惯）、追溯矩阵

## 任务 2：真机会话（条件任务——用户提供非 MIUI 设备后执行，无设备则保持开放）
- web 路径验证（R14-1/3）：Play Chrome + adb reverse + build/demo-site 服务器，清单
  沿用 android_真机安装测试方案 §8 方法论（MIUI shell 门禁绕行结论直接复用）
- 校准分布回填（DR-010/011 真机闭环）：真机首建流程实测 t 落点与解锁耗时，记录电源
  状态（HyperOS 夜间降频教训）；结果回填出口报告附录与 DR-010/011 证据链
- 真机安装：aarch64 release 交叉编译（CI 已验通过性）；MIUI 门禁绕行见方案 §8

## 任务 3：Keystore/生物识别解锁 DR 草案（仅决策材料，不拍板不实现）
- 范围：Keystore 会话缓存（进程被杀后恢复解锁态）vs 当前纯内存模型；生物识别
  （BiometricPrompt）作为解锁的附加因子还是替代；与任务 1 DR-012 的凭据缓存决策
  联动（两者都触碰"凭据驻留"边界，宜一次论证）
- 产出：docs/reviews/keystore_biometric_决策材料_YYYY-MM-DD.md（威胁模型、选项
  矩阵、行业对标 Bitwarden/KeePassDX、推荐倾向）——真正拍板留给用户

## 任务 4：桌面阶段 1 穿插评估（DR-009 承诺 ≤20%；可选，会话预算不足则跳过）
- P2 协议加密（TweetNaCl box，DR-006 预留适配器切换点）+ 密钥复用开库（keepass_ng
  复合密钥直开可行性重估）
- 仅做设计要点与工作量评估，不启动实现（避免与任务 1 抢上下文）

## 任务 5：门禁与出口
- 本地：cargo fmt --all -- --check；cargo clippy --workspace --all-targets -- -D
  warnings；cargo test --workspace（72+ 基线不回退）；cargo deny check；CI 六 job 绿
- 出口标准：① DR-012 拍板且保存场景实现+验收绿；② design_docs/14 v1.2 与 SRS/矩阵
  同步；③ （若有设备）真机回填结果入库；④ CI 六 job 绿
- 开发模型：Git Flow（feature/step6-* 分支 → PR → develop）；Conventional Commits；
  设计不符先改 design_docs 再改码

## 约束
- 红线十条（docs/SRS.md §7）不回退；不改 sp-core/sp-kdbx/sp-cli/sp-nmhost 既有公开
  API（sp-kffi 增量允许）；机密不落 SharedPreferences/日志/剪贴板历史
- 不做：iOS、多库、宽匹配（R14-2 维持 backlog）
- 任何 Mimosa 拦截：内容交用户手动落盘，不静默绕过

## 已知坑位清单（步骤 4/5 会话实测，直接复用结论）
- cargo-ndk 4.x：`-p` 须放 `--` 之后（cargo ndk -t x86_64-linux-android --platform 28
  -- build -p sp-kffi）
- CI 的 android job：targets 必须用 `rustup target add` 装到锁定工具链（dtolnay action
  的 targets 参数会装到 stable）
- UniFFI Kotlin：Record 字段 snake→camelCase（has_otp→hasOtp）；错误枚举→sealed
  FfiException.*；Rust `impl Default` 文档注释不进生成绑定（改注释无漂移）
- Kotlin/Android API：密码判定=TYPE_CLASS_TEXT|TYPE_TEXT_VARIATION_PASSWORD；
  Dataset.Builder 的 presentation 走构造器；**FillResponse.setAuthentication 的
  presentation 在 API 33+ 强制非空（三参 null 抛 IllegalArgumentException）**；
  API 34 起 adb shell 无法启动非导出组件（TestActivity 因此保留 exported）
- UI 自动化（模拟器）：锁定态聚焦任意密码框会触发本应用认证数据集弹窗抢走
  `input text` 按键——先 `settings put secure autofill_service null` 或先解锁会话；
  软键盘弹出引起布局平移，点击前先收键盘；`input text` 末字符可能滞留 IME 组字态，
  换焦点提交后再断言；MIUI 真机 shell 门禁绕行见 android_真机安装测试方案 §8
- 跨平台：apkm 下载需过 Cloudflare（curl 被拒，浏览器手动可行）；x86_64 模拟器跑
  arm 应用需 playstore 镜像（ndk_translation），Chrome 不兼容翻译层（SIGSEGV）
- Git Bash 调 adb：路径参数加 MSYS_NO_PATHCONV=1；AVD 创建用 ANDROID_SDK_ROOT
  环境变量（--sdk_root 后置无效）
- CI 模拟器（android-e2e job，步骤 5.5 实测）：hosted runner 装完工具链后 home 分区
  余量不足以创建 API 34 userdata 分区（emulator FATAL "not enough space to create
  userdata partition" 后空轮询到 boot-timeout）——必须先清 dotnet/ghc/CodeQL/旧 NDK
  （保留最新一个，cargo-ndk 取最新）并 docker prune，且 android-emulator-runner 显式
  disk-size=2048M + cores=4（默认 2 核有官方性能警告）
- Chrome on 模拟器判离线：全局代理 10.0.2.2:10808（宿主代理开时）+ captive portal
  指向本地 gen_204（build/demo-site/server_204.py）；演示页服务器 build/demo-site/
  python http.server 8080 + adb reverse tcp:8080
- 模拟器遗留状态：autofill 服务选择已配置（com.superpasswdkey.spike/.SpAutofillService）；
  files/ 下可能残留 result.txt/init_probe.kdbx（测试产物，无害）
```
