# Android 开发环境部署报告（步骤 4 Spike 前置）

- 日期：2026-10-27｜执行：ZCode 会话（DR-008 阶段 0/规划）｜性质：环境部署，不涉及代码
- 结论：**Android 开发与测试环境已部署完整，Spike 可启动**

## 一、部署前现状（探测：2026-10-27）

| 组件 | 部署前 |
|---|---|
| Java | ✅ 已有 Temurin 17.0.20（scoop），满足 AGP/SDK 工具要求 |
| Android SDK（E:/dev/android-sdk） | ✅ 已有（用户预装），组件齐全：cmdline-tools latest、platform-tools 37.0.1、platforms 34/35/36、build-tools 36.0.0、NDK r28.2.13676358、emulator 37.1.11、system-images;android-34;google_apis;x86_64 |
| AVD | ✅ 已有 Pixel_7_API34（x86_64 镜像，与本机 Windows x64 匹配） |
| ANDROID_HOME/ANDROID_SDK_ROOT/PATH | ❌ 未设置 |
| Rust android targets | ❌ 未安装 |
| cargo-ndk | ❌ 未安装 |

## 二、本次部署动作

1. 用户级环境变量（setx）：`ANDROID_HOME`=`ANDROID_SDK_ROOT`=`E:\dev\android-sdk`；PATH 追加 platform-tools、emulator、cmdline-tools\latest\bin（新 shell 生效，已复核）。
2. `rustup target add aarch64-linux-android x86_64-linux-android`——**注意双工具链**：仓库 rust-toolchain.toml 锁 1.99.1，首次 add 落在仓库工具链上；默认工具链 stable 已单独补装（Spike 的 kffi crate 也在仓库工具链下工作，无需额外动作，但独立目录构建需注意）。
3. `cargo install cargo-ndk`（v4.1.2）。

## 三、验证结果

| # | 验证 | 结果 |
|---|---|---|
| V1 | `sdkmanager --list_installed`：9 包齐全（build-tools;36.0.0 / cmake;3.22.1 / emulator 37.1.11 / ndk;28.2.13676358 / platform-tools 37.0.1 / platforms;android-34/35/36 / system-images;android-34;google_apis;x86_64） | ✅ |
| V2 | NDK 链接探针：临时 cdylib crate 经 `cargo ndk -t x86_64-linux-android -t aarch64-linux-android build --release`，双 target `libndk_probe.so` 均产出（clang/lld 链接链通；cargo-ndk 复制目录用 ABI 名 out/x86_64 与 out/arm64-v8a） | ✅ |
| V3 | `emulator -list-avds`：Pixel_7_API34 可见（启动留给 Spike 会话） | ✅ |
| V4 | 探针 crate 已清理，无残留 | ✅ |

## 四、给 Spike 会话的环境事实（写入任务书）

- SDK 根：`E:\dev\android-sdk`（ANDROID_HOME/SDK_ROOT 已设）；NDK r28.2.13676358
- e2e 模拟器：AVD `Pixel_7_API34`（android-34;google_apis;x86_64）——Windows x64 主机用 x86_64 镜像；**aarch64 仅作交叉编译发布目标**，真机验证可选
- Rust：仓库工具链 1.99.1 已含双 android target；cargo-ndk 4.1.2
- 注意：cargo-ndk 多 target 时 crate-type 须含 `cdylib`（bin 双 target 会报 "No usable artifacts"）

## 五、遗留与提示

- E 盘剩余 39G（96% 使用率）：本轮未新增大组件；后续如需加装系统镜像/NDK 新版本需留意空间
- WSL 侧无 Android 工具（无需——交叉编译在 Windows 侧完成）
- 模拟器启动首次可能需要接受许可/初始化，属正常
