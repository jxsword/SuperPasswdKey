# android/ — SuperPasswdKey M1 壳层（SP-F-802/803/804；DR-009/DR-011）

Android 壳层：UniFFI 绑定复用 Rust 内核 + 解锁会话 + 系统 Autofill 对接。
M1 正式化后结构：MainActivity（产品壳入口）、UnlockActivity（解锁/首建双模式）、
SpAutofillService（填充 + 认证回路）、VaultSession（会话单例）、TestActivity
（e2e harness）、SpApplication（进程级加载 cdylib）。
设计见 `design_docs/13_Android_UniFFI.md` 与 `design_docs/14_Android_Autofill.md`。

## 构建步骤（Windows 主机，x86_64 模拟器口径）

```bash
# 1. 交叉编译 Rust cdylib（模拟器 x86_64；发布目标另见 aarch64 命令）
cargo ndk -t x86_64-linux-android --platform 28 -- build -p sp-kffi
cp target/x86_64-linux-android/debug/libsp_kffi.so \
   android/app/src/main/jniLibs/x86_64/

# 2. （若改动 FFI 面）重新生成 Kotlin 绑定并覆盖 app/src/main/java/uniffi/sp_kffi/
./target/debug/uniffi-bindgen.exe generate \
  --library target/debug/sp_kffi.dll --language kotlin \
  --out-dir android/app/src/main/java --no-format

# 3. 生成测试库 fixture（凭据走环境变量，不落源码）
mkdir -p build
SP_TEST_DB_PASSWORD=... SP_TEST_ENTRY_PASSWORD=... \
  SP_TEST_ENTRY_URL=https://com.superpasswdkey.spike \
  cargo run -q -p sp-kffi --example make_spike_vault -- build/test_vault.kdbx
#   （SP_TEST_ENTRY_URL 传应用包名做裸 host 匹配 → Autofill 包名路径演示链路可用；
#    web 演示另需 127.0.0.1 口径的 fixture，二者取一）

# 4. 构建 APK
cd android && ./gradlew.bat :app:assembleDebug
```

## e2e 与验收流程（Pixel_7_API34 模拟器）

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb push build/test_vault.kdbx /data/local/tmp/
adb shell run-as com.superpasswdkey.spike mkdir -p files
# fixture 即壳层库文件（VaultSession.VAULT_NAME = vault.kdbx）：
adb shell run-as com.superpasswdkey.spike cp /data/local/tmp/test_vault.kdbx files/vault.kdbx
adb logcat -c
adb shell am start -n com.superpasswdkey.spike/.TestActivity \
  --es db_password ... --es entry_password ... --es entry_username demo-user \
  --es entry_url https://com.superpasswdkey.spike \
  --ez init_vault true --ei idle_timeout_ms 2000
adb logcat -d -s SPIKE
```

- 全套断言：开库/全字段读回/摘要/TOTP/NotFound/错密码/KDF 复测 + 首建校准链路
  （init_vault=true，断言参数落 DR-010 边界）+ 超时锁库注入（idle_timeout_ms 缩短
  至秒级，断言锁定且机密操作拒绝——SP-F-802 可注入时钟验收）。
- `TestActivity` 为 exported=true（API 34 起 shell 无法启动非导出组件，凭据 extras
  直驱需要）；演示/解锁 Activity 均为非导出，经 MainActivity 按钮触达。
- 首次初始化 UI 流程：删除 `files/vault.kdbx` 后经 MainActivity → "解锁 / 首次
  初始化库"，首建表单走 calibrate_kdf 校准建库（工作线程数秒，UI 全程提示），
  成功回显校准参数（DR-010/011 终态）。
- Autofill 链路回归：系统"自动填充服务"选 SuperPasswdKey 后，演示登录页聚焦
  字段触发；锁定态走认证回路（认证数据集 → UnlockActivity 解锁 → 回传填充）。

（Git Bash 下 adb 路径参数需 `MSYS_NO_PATHCONV=1`，否则 `/data/...` 被本地路径转换。）

## aarch64 发布目标（真机可选，非 M1 必需）

```bash
cargo ndk -t aarch64-linux-android --platform 28 -- build -p sp-kffi --release
# 产物：target/aarch64-linux-android/release/libsp_kffi.so
```

## 注意

- `app/src/main/jniLibs/**/*.so` 已 gitignore：二进制不入库，由步骤 1 复制。
- `local.properties`（sdk.dir）不入库；`gradle/wrapper` 与生成绑定 sp_kffi.kt 入库。
- 凭据只经环境变量/intent extra/FFI 调用栈传递；不落 SharedPreferences、不进日志
  （红线约束，SP-F-804 代码审查口径）。
- 需要 ANDROID_HOME 指向 Android SDK（本机 E:\dev\android-sdk）与 JDK 17。
