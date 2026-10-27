# android/ — SuperPasswdKey Android Spike 壳（DR-008）

最小 Android 工程仅为验证 UniFFI 绑定在模拟器上可用（e2e 断言 + KDF 计时），
不是产品壳层。设计与结论见 `design_docs/13_Android_UniFFI.md` 与
`docs/reviews/android_spike_结论报告_2026-10-27.md`。

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
  cargo run -q -p sp-kffi --example make_spike_vault -- build/test_vault.kdbx

# 4. 构建 APK
cd android && ./gradlew.bat :app:assembleDebug
```

## 运行 e2e（Pixel_7_API34 模拟器）

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb push build/test_vault.kdbx /data/local/tmp/
adb shell run-as com.superpasswdkey.spike mkdir -p files
adb shell run-as com.superpasswdkey.spike cp /data/local/tmp/spike_test_vault.kdbx files/test_vault.kdbx
adb logcat -c
adb shell am start -n com.superpasswdkey.spike/.MainActivity \
  --es db_password ... --es entry_password ... --es entry_username demo-user
adb logcat -d -s SPIKE        # 或: adb shell run-as com.superpasswdkey.spike cat files/result.txt
```

（Git Bash 下 adb 路径参数需 `MSYS_NO_PATHCONV=1`，否则 `/data/...` 被本地路径转换。）

## aarch64 发布目标（真机可选，非 spike 必需）

```bash
cargo ndk -t aarch64-linux-android --platform 28 -- build -p sp-kffi --release
# 产物：target/aarch64-linux-android/release/libsp_kffi.so
```

## 注意

- `app/src/main/jniLibs/**/*.so` 已 gitignore：二进制不入库，由步骤 1 复制。
- `local.properties`（sdk.dir）不入库；`gradle/wrapper` 与生成绑定 sp_kffi.kt 入库。
- 凭据只经环境变量/intent extra 传递；不落 SharedPreferences、不进日志（红线约束）。
- 需要 ANDROID_HOME 指向 Android SDK（本机 E:\dev\android-sdk）与 JDK 17。
