# Android UniFFI Spike 结论报告（DR-008 时间盒验收）

- 日期：2026-10-27 ｜ 会话：SuperPasswdKey 步骤4（Windows 工作区）
- 分支：`feature/android-spike`（自 develop）｜ 范围：Spike，**不打 tag、不发布**
- 输入：docs/任务书_step4.md、docs/reviews/android_env_部署报告_2026-10-27.md、DR-002/005/008
- 产出：crates/sp-kffi（新增）、android/（新增壳工程）、design_docs/13（新增骨架）、本报告

## 1. 结论先行：三项分流标准全过，建议 Android 主线（待用户拍板 → DR-009）

| DR-008 分流标准 | 判定 | 证据（本报告 §3/§4） |
|---|---|---|
| ① UniFFI Kotlin 绑定编译通过 | ✅ 通过 | sp_kffi.dll 生成 1824 行 Kotlin 绑定；`android/` APK 构建成功（Gradle 8.13 + AGP 8.7.3 + Kotlin 2.1.0 + JNA 5.17.0） |
| ② Android 模拟器 sp-kdbx 打开 KDBX4 e2e | ✅ 通过 | Pixel_7_API34（x86_64）8 项断言全 PASS：全字段读回、摘要面（不含密码）、TOTP、NotFound、错密码 Credentials |
| ③ 64MiB Argon2id ≤3s | ✅ 通过（**非真机**） | 模拟器 3 次开库 1045/1124/1101 ms，均 << 3s；内存峰值 delta ≈68MiB，与 m=64MiB 理论吻合 |
| 附：aarch64 发布目标交叉编译 | ✅ 通过 | `cargo ndk -t aarch64-linux-android -- build -p sp-kffi --release`，产物 1.9MB |

**建议**：按 DR-008，三项全过 → **阶段3（M1 系统 Autofill）为主线，阶段1（桌面 P2 加密+密钥复用开库）降穿插（≤20%）**。
转正属重大决策，待用户确认后写 DR-009（含 KDF 移动端参数定稿、SRS 非目标修订）。

## 2. Spike 期间的结构性发现（比"通过"更有价值的部分）

1. **keepass_ng `Database` 是 `!Send + !Sync`**（`Rc<RefCell<dyn Node>>` 内部树），而 UniFFI 0.32 对导出对象编译期强制 `Send + Sync`——直接持有写法不可行。
   解法 = **线程封闭 actor**（crates/sp-kffi/src/lib.rs）：每 `Vault` 一个工作线程，`Database` 全程单线程触碰，方法经 channel 派发。类型安全（无 unsafe），Kotlin 任意线程可调。已写入 design_docs/13 §2（含代价与转正时需定的线程模型选项）。
2. **UniFFI Kotlin 侧细节**（转正前已知）：记录字段 snake_case → camelCase（`has_otp`→`hasOtp`）；错误枚举 → sealed `FfiException.*` 类；需 JNA 5.17.0@aar；`--library` 绑定生成需 host 可加载产物（Windows 用 .dll，与 Android .so 元数据同源）。
3. **JNI String 不可零化**（R13-1）：`FfiEntry.password` 在 JVM 堆留副本，红线 #5 在 FFI 边界外不完整——这是移动端固有限制（1Password/Bitwarden Android 同样存在），如实进风险清单与用户文档。

## 3. e2e 断言明细（模拟器实测，2026-10-27 16:27）

fixture：`make_spike_vault`（64MiB/t=8/p=4 默认参数；条目 "Demo Login" + "Demo OTP"[RFC 6238 参考密钥]；凭据经环境变量/intent extra，不落源码/SharedPreferences/日志）。

```
PASS: openDb #1 (KDF 64MiB Argon2id t=8 p=4) 1045 ms; VmHWM 92228->161724 kB (delta 69496 kB) [非真机: x86_64 emulator Pixel_7_API34]
PASS: findEntry('Demo Login') 全字段读回一致            // username/password/url/otp 与写入一致
PASS: listEntrySummaries() 2 条；密码不出现在摘要面       // SP-F-706 语义
PASS: entryTotpNow('Demo OTP') -> 6 位数字码            // RFC 6238 参考密钥
PASS: findEntry(missing) -> FfiException.NotFound       // SP-F-204 语义
PASS: 错密码 -> FfiException.Credentials                // 凭据语义不细化
PASS: openDb #2 (reopen) 1124 ms; VmHWM 163780 kB
PASS: openDb #3 (reopen) 1101 ms; VmHWM 163860 kB
== ALL E2E CHECKS PASSED ==
```

对照基准：主机侧 `cargo test -p sp-kffi` 4/4 绿（同一断言面 + keyfile 双因子 + 64MiB 硬下限拒绝）；语义对照 crates/sp-kdbx/tests 既有测试。

## 4. KDF 实测（SP-N-301/302 回填口径）

| 指标 | 数值（x86_64 模拟器，**非真机**） | 说明 |
|---|---|---|
| 开库总耗时（含 KDF+解密+解析） | 1045 / 1124 / 1101 ms（3 次） | 库 1.1KB、2 条目；KDF 主导 |
| 64MiB Argon2id（t=8 p=4）内存峰值 | VmHWM delta ≈68 MiB | 与 m=64MiB 理论值吻合；进程总 HWM ≈160MB（含 App 基线 ~90MB） |
| 对标 ≤3s 分流线 | 余量 ~2.7× | x86_64 模拟器跑在桌面 CPU 上，数字系统性优于真机——**真机复核列入转正前置**（design_docs/13 §4.6） |

## 5. 门禁与流程

- 本地：`cargo test --workspace` 全绿（16 套件，含 sp-kffi 4 项 e2e，spike 未破坏桌面既有测试）；`cargo clippy --workspace --all-targets -- -D warnings` 全绿；`cargo fmt --all` 已执行；`cargo deny check` 全绿（advisories/bans/licenses/sources）。
  - 环境注：cargo-deny 的 advisory 库经 HTTPS 拉取被本机失效代理（.gitconfig 对 github.com 配置的 socks5://127.0.0.1:10808，代理客户端未运行）阻断，本次以 SSH 克隆替换 `CARGO_HOME/advisory-dbs/` 下的缓存库并改其 origin 为 SSH 后通过；用户代理恢复后可自行还原。
- CI：push 后四 job（windows/linux/deny/extension）全绿。
- Git Flow：`feature/android-spike` → PR → develop；Conventional Commits；**不打 tag**。

## 6. 范围与偏差记录

- 未做（按任务书约束）：iOS、服务器轨、多库、生物识别、Keystore、真机验证（可选非必需）。
- 偏差 1：任务书示例 fixture `make_test_vault` 不含 OTP 条目，而导出面含 `entry_totp_now`——
  新增 `crates/sp-kffi/examples/make_spike_vault.rs`（在 make_test_vault 语义上增加 OTP 条目 + 默认 KDF 参数 + 可选 keyfile），sp-nmhost 未改动。
- 偏差 2：任务书写 `crates/sp-kffi` 名字已采用；主机生成绑定用 `.dll`（UniFFI `--library` 模式需可加载库），与 Android `.so` 同一 Rust 面。
- 环境注：本会话 shell 未继承用户级 ANDROID_HOME（部署报告的变量为会话后生效），全程以显式路径/导出使用，功能无影响。

## 7. 待用户拍板事项（回复后写 DR-009）

1. 是否接受"三项全过 → Android 主线（M1 Autofill）"分流。
2. 移动端 KDF 默认参数是否维持 64MiB/t=8/p=4（真机校准后可上调，SP-N-302）。
3. 转正清单（design_docs/13 §4）优先级确认。
