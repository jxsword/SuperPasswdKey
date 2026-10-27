# SuperPasswdKey 步骤 5：M1 正式化（Android Autofill 出口）

> 本文件是给 Windows 工作区 ZCode 会话的首次启动提示词。直接把下方代码块内容
> 粘贴为该会话的第一条消息即可。生成于 2026-10-28（M1 原型验证完成后，main=develop=276f84a）。

```text

## 项目背景（自包含，无需外部上下文）
- 仓库：GitHub 私有仓 jxsword/SuperPasswdKey，main=develop=276f84a，工作树干净
- 项目：Rust 密码管理器（sp CLI），KDBX4 兼容（keepass_ng 0.11），Argon2id+双因子
- 步骤 4 已完成（M1 原型验证，PR #16–#22 全部合入）：
  - crates/sp-kffi：UniFFI 0.32 绑定（create_db/open_db/find_entry/list_entry_summaries/
    entry_totp_now/add_entry/edit_entry/save/add_attachment/get_attachment/lock/
    rank_entry_urls/calibrate_kdf）。结构性事实：keepass_ng Database 为 !Send+!Sync →
    线程封闭 actor（每 Vault 一个工作线程，命令 channel 派发），见 design_docs/13 §2
  - android/：最小壳（Gradle 8.13 + AGP 8.7.3 + Kotlin 2.1.0 + JNA 5.17.0@aar，无 AndroidX）
    组件：VaultSession（300s 空闲锁库+懒检查）/ UnlockActivity（手动解锁+Autofill 认证回路）/
    SpAutofillService（表单启发式+webDomain/包名匹配+Dataset）/ DemoLoginActivity /
    DemoWebActivity / MainActivity（e2e harness + 入口按钮）
  - 文档：design_docs/13（Android_UniFFI v1.0）、14（Android_Autofill）、SRS V1.2（§3.8 SP-F-801~804）、
    DR-009（Android 主线）、DR-010（移动端 KDF：终态=init 校准 t∈[2,8]/m=64MiB/p=4 目标~1s，
    过渡=FFI 默认 t=4/p=4）
  - 真机实测（Redmi Turbo 3/HyperOS）：原生路径 autofill 端到端全通；KDF 64MiB/t=8/p=4
    下午 2.2–2.5s、夜间降频 8.2–18.3s（HyperOS 午夜睡眠，实测必须记录电源状态）
  - 未决（不在本任务内）：web 字段 autofill 委派需 Play Chrome + 非 MIUI 真机（R14-1/3 开放，
    证据链见 docs/reviews/android_真机安装测试方案_2026-10-27.md §8–§12）

## 任务 0：环境自检（已部署完成，会话开始快速自检，任何异常先修再开工）
- 本机：Windows x64；SDK E:\dev\android-sdk（用户级 ANDROID_HOME 已设，但旧 shell 可能
  未继承，命令中显式 export 或用绝对路径）；NDK r28.2.13676358；JDK 17；cargo-ndk 4.1.2
- AVD：Pixel_7_API34（google_apis x86_64）、Pixel_7_play（google_apis_playstore x86_64，
  含 ARM 翻译 abilist=x86_64,arm64-v8a）
- 自检命令（Git Bash）：
  export PATH="$PATH:/e/dev/android-sdk/platform-tools"
  adb version | head -1
  rustup target list --installed | grep android
  E:/dev/android-sdk/emulator/emulator.exe -list-avds
  cargo test -p sp-kffi 2>&1 | tail -3
  git status -sb

## 任务 1：KDF 校准接入（DR-010 终态切换）
- sp-core::calibration::calibrate_mobile（target_ms=1000，t∈[2,8]/m=64MiB/p=4）已实现并有测试；
  FFI 已有 calibrate_kdf。任务：Android 壳层加"首次初始化"流程——创建库时调 calibrate_kdf
  取参建库（工作线程执行，耗时数秒需 UI 提示），校准结果随库文件存储
- 落地后：过渡默认 FfiKdfParams::default（t=4/p=4）仅保留为"跳过校准"的兜底；测试同步更新
- 写 DR-011：终态切换确认（引用 DR-010、真机数据、本任务实测）

## 任务 2：壳层正式化（SP-F-802/804 完整化）
- 演示/测试代码与产品壳分离：MainActivity 的 e2e harness 与入口按钮重组（或拆出 TestActivity）
- UnlockActivity 打磨：错误呈现（FfiException 分类提示）、锁定状态显示、Autofill 认证回路回归
- 机密边界代码审查（SP-F-804）：凭据不落 SharedPreferences/文件/日志；allowBackup=false 已有；
  exported 最小化复查
- 空闲锁库验收：超时可注入测试（VaultSession 的 300s 常量留注入点）

## 任务 3：门禁与 CI
- 本地：cargo fmt --all -- --check；cargo clippy --workspace --all-targets -- -D warnings；
  cargo test --workspace（当前 72 测试全绿，不得回退）；cargo deny check
- CI 已有 android job（cargo-ndk 双 target + 绑定漂移检查），勿破坏
- 可选加分：GitHub Actions 模拟器 e2e job（ubuntu-latest 带 KVM，android-emulator-runner；
  需先把"模拟器上原生路径 e2e"脚本化——参考 MainActivity.run() 的断言集）

## 任务 4：M1 出口报告
- docs/reviews/m1_出口报告_2026-XX-XX.md：验收对照（SP-F-801~804 状态）、DR-010/011 落地证据、
  风险清单更新（R14-1/3 开放项）、后续 backlog（web 路径待非 MIUI 真机、Autofill 保存场景、
  Keystore/生物识别独立 DR）
- SRS/追溯矩阵：SP-F-8xx 状态列更新（本任务完成后）
- 不打 tag、不发布（M1 出口 ≠ 发布）

## 任务 5：验收与出口
- 出口标准（全部满足才出口）：① 校准接入且 DR-011 写入；② 壳层重组后 SP-F-802 验收绿
  （含超时锁库注入测试）；③ 全部本地门禁绿 + CI 五 job 绿；④ 出口报告入库
- 开发模型：Git Flow（feature/m1-formalization 分支 → PR → develop）；Conventional Commits；
  设计不符先改 design_docs 再改码；决策记录按分级规则（重大决策写 DR）

## 约束
- 红线十条（docs/SRS.md §7）不回退；不改 sp-core/sp-kdbx/sp-cli/sp-nmhost 既有公开 API
  （sp-kffi 增量允许）；机密不落 SharedPreferences/日志/剪贴板历史
- 不做：web 字段 autofill 验证（待非 MIUI 设备）、iOS、多库、Keystore/生物识别（独立 DR）
- 任何 Mimosa 拦截：内容交用户手动落盘，不静默绕过

## 已知坑位清单（步骤 4 会话实测，直接复用结论）
- cargo-ndk 4.x：`-p` 须放 `--` 之后（cargo ndk -t x86_64-linux-android --platform 28 -- build -p sp-kffi）
- CI 的 android job：targets 必须用 `rustup target add` 装到锁定工具链（dtolnay action 的
  targets 参数会装到 stable，导致 core 缺失）
- UniFFI Kotlin：Record 字段 snake→camelCase（has_otp→hasOtp）；错误枚举→sealed FfiException.*
- Kotlin/Android API：InputType 无 TYPE_CLASS_PASSWORD（密码判定=
  TYPE_CLASS_TEXT|TYPE_TEXT_VARIATION_PASSWORD）；Dataset.Builder 的 presentation 走构造器；
  FillResponse.setAuthentication 三参重载需显式 `null as RemoteViews?`
- 跨平台：apkm 下载需过 Cloudflare（curl 被拒，浏览器手动可行）；x86_64 模拟器跑 arm 应用
  需 playstore 镜像（ndk_translation），Chrome 不兼容翻译层（SIGSEGV）
- MIUI/HyperOS shell 门禁：install 需手机端"USB 安装"；pm clear/force-stop/input 注入/
  secure settings 写入均受限（绕行见 android_真机安装测试方案 §8）
- Git Bash 调 adb：路径参数加 MSYS_NO_PATHCONV=1；AVD 创建时 --sdk_root 放在 create avd 之后无效
  （用 ANDROID_SDK_ROOT 环境变量）
- Chrome on 模拟器判离线：系统网络"未验证"所致，解法=全局代理 10.0.2.2:10808（宿主代理开时）
  + captive portal 指向本地 gen_204（build/demo-site/server_204.py）
- 模拟器演示页服务器：build/demo-site/ 下 python http.server 8080 + adb reverse tcp:8080

```
