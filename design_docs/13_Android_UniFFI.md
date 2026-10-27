# 13 Android（UniFFI 复用内核）— Spike 骨架

- 状态：v0.1 骨架（2026-10-27，Spike 产出，DR-008 时间盒内；**转正需用户确认并另行 DR-009**，SRS §2.3 非目标暂不改动）
- 追溯：DR-002（内核复用承诺）、DR-008（spike 分流）、SP-N-301/302（KDF 实测回填）、SP-C-106（测试向量）
- 实测报告：docs/reviews/android_spike_结论报告_2026-10-27.md（本骨架引用其数字，不重复展开）

## 1. 绑定边界（spike 实现面）

| 层 | 内容 | 说明 |
|---|---|---|
| `crates/sp-kffi` | UniFFI 0.32 procmacro 导出 | 纯类型桥接，无业务/加密逻辑；不改 sp-core/sp-kdbx 公开 API（独立 crate 隔离） |
| 函数面 | `create_db` / `open_db` | 语义 = `sp_kdbx::create_db` / `open_db`；双因子经 `keyfile_path: Option<String>` 完整透传（红线 #7） |
| 对象面 | `Vault`（已解锁库句柄） | `find_entry` / `list_entry_summaries` / `entry_totp_now`，语义 = sp-kdbx 同名函数 |
| 记录面 | `FfiEntry`（含密码）/ `FfiEntrySummary`（不含密码，SP-F-706 语义）/ `FfiKdfParams` | 摘要面与桌面扩展响应面同构：密码/TOTP 仅单条显式取回 |
| 错误面 | `FfiError`（Credentials/NotFound/Format/Io/Parameter）→ Kotlin `FfiException.*` sealed 类 | 凭据语义不细化（SP-F-107）；映射表见 sp-kffi/src/lib.rs |
| Android 壳 | `android/`（Gradle 8.13 + AGP 8.7.3 + Kotlin 2.1.0 + JNA 5.17.0） | 最小 Activity，e2e 断言 + KDF 计时；生成绑定 sp_kffi.kt 随库提交 |

## 2. 生命周期与线程模型（spike 结构性结论）

**事实**：keepass_ng 0.11 的 `Database` 内部为 `Rc<RefCell<dyn Node>>` 树，`!Send + !Sync`；而 UniFFI 0.32 对所有导出对象在编译期强制 `Send + Sync`（`FfiConverterArc` bound）。两者直接冲突——任何把 `Database` 塞进 UniFFI Object 字段的写法都编译不过。

**spike 解法 = 线程封闭 actor**（crates/sp-kffi/src/lib.rs）：
- 每个 `Vault` 拥有一个专用工作线程；`Database` 仅在该线程内被创建与触碰（move 语义上也不跨线程）。
- 方法调用 = 命令（含 oneshot 回执 channel）→ 工作线程串行执行 → 阻塞返回。
- `Vault` 被 Kotlin GC 回收 → JNA dispose → Arc 释放 → channel 断开 → 工作线程退出并 drop `Database`（密钥材料随 drop 零化）。
- 好处：无 `unsafe impl Send/Sync`；Kotlin 侧可任意线程调用（并发请求自然串行化）；生命周期由 channel 归属静态保证。

**代价与后续选项**（转正时定稿，DR-009）：
- 每 Vault 一个常驻线程（空闲栈 ~16KB 级）。多库/多会话场景需改线程池或单 actor 多库复用。
- `open_db` 的 KDF 发生在工作线程；Android 主线程调用会阻塞 UI——正式 App 应经协程（`Dispatchers.IO`/`Default`）调用，本 spike 壳直接主线程调用仅为简化。
- 备选路径：上游 keepass_ng 将 Rc 改 Arc+Mutex（外来改动，不确定可接受）；或整库序列化快照传递（复杂度高）——均不如 actor 现实。

## 3. 风险清单（spike 时点）

| # | 风险 | 等级 | 缓解/去向 |
|---|---|---|---|
| R13-1 | JNI String 无法零化：`FfiEntry.password` 经 JNA 在 JVM 堆产生不可清零副本 | 中 | 记录边界并写用户文档；正式版评估只回退 password 的专用通道（如 byte[] + 调用后覆写仍不可靠——如实声明"Java 层不可保证"） |
| R13-2 | 模拟器 KDF 数字≠真机（x86_64 + 桌面级 CPU） | 高误导性 | 所有数字标注"非真机"；真机验证列入转正前置（非 spike 必需） |
| R13-3 | 每 Vault 常驻线程 + Autofill 高频唤醒场景的线程开销 | 低-中 | 转正时按 Autofill 会话模型设计（单库单线程足够） |
| R13-4 | 生成 Kotlin 绑定（sp_kffi.kt）随库提交，可能与 Rust 面漂移 | 低 | CI 增加"重新生成后 diff 为空"检查（转正清单） |
| R13-5 | UniFFI 大版本升级（0.32→1.x）可能改变对象线程要求/生成代码 | 低 | 锁版本；升级随转正评估 |
| R13-6 | Keystore/生物识别集成缺位（spike 明确不做） | — | backlog 单列（任务书 §约束）；不进本骨架 |
| R13-7 | 主线程调用阻塞（actor 同步等待） | 低 | 正式壳层用协程包一层；spike 壳已知简化 |

## 4. 待转正清单（不改 SRS/追溯矩阵正文，转正 DR-009 一并处理）

1. SRS §2.3 非目标移除 Android（或改"Android(阶段3 计划中)"）+ 新增 SP-F-8xx 功能面（开库/列表/取回/TOTP/Autofill 对接）。
2. 追溯矩阵补 Android 行（DR-002 承诺闭环、SP-N-301/302 真机回填）。
3. CI：Android 交叉编译 job（cargo-ndk x86_64+aarch64）+ 绑定漂移检查 + （可选）模拟器 e2e job。
4. KDF 参数策略：移动端默认值（64MiB/t/p 组合在真机校准，SP-N-302）+ 校准入口是否暴露给 UI。
5. FFI 面扩容评估：add/edit/save/attachments（桌面已实现，绑定层纯增量）。
6. 真机验证（KDF 数字、Autofill 精度、后台行为）。

## 5. spike 验收结论（摘录，详见结论报告）

| DR-008 标准 | 结果 |
|---|---|
| ① UniFFI Kotlin 绑定编译通过 | ✅（x86_64 模拟器 APK 构建成功） |
| ② 模拟器 sp-kdbx 打开 KDBX4 e2e | ✅（8 项断言全过：全字段读回/摘要/TOTP/NotFound/Credentials） |
| ③ 64MiB Argon2id ≤3s | ✅ 模拟器 1.0–1.1s、内存峰值 delta ≈68MiB（**非真机**） |
| aarch64 发布目标交叉编译 | ✅（release 1.9MB .so） |
