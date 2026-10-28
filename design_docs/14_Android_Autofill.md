# 14 Android M1：系统 Autofill 对接（SP-F-802/803）

- 状态：v1.1（2026-10-28，M1 正式化定稿：壳层重组/首建校准/超时注入验收/认证回路修复；v0.1 骨架 2026-10-27）
- 上游依赖：design_docs/13（UniFFI 绑定与 actor 线程模型，已转正）
- 范围：M1 = **填充读路径**（Autofill 响应库内条目）；Autofill 保存场景（新密码入库）backlog 单列

## 1. 目标与验收锚点

- SP-F-802：解锁会话（超时锁库、凭据清零、不落 SharedPreferences/日志）
- SP-F-803：AutofillService 按 web 域名/包名匹配条目，返回用户名+密码数据集；不匹配返回空集不猜测
- 验收：模拟器主链路（Chrome/演示 App）填充成功；真机精度实测为验收前置（DR-009）

## 2. 组件结构（android/ 壳内增量）

| 组件 | 职责 |
|---|---|
| `SpAutofillService`（android.service.autofill） | 接收 FillRequest → 提取 web 域名/包名 → 匹配 → 构造 Dataset（RemoteViews 不回显明文）；锁定态 Dataset 挂 UnlockActivity 认证回路（presentation 复用条目样式，API 33+ 强制非空） |
| `UnlockActivity` | 凭据收集（主密码+可选 keyfile 路径文本框）；双模式：库存在→解锁 / 缺失→首建（双密码校验 → VaultSession.createVault 走 calibrate_kdf 校准建库，DR-010/011 终态，界面全程提示、成功回显校准参数）；FfiException 分类错误呈现；解锁成功仅内存回传，不落盘 |
| `VaultSession`（单例） | 持有 kffi `Vault` + 最后活跃时间戳；空闲超时（默认 300s，对齐 SP-F-703）调 `lock()`；懒检查兜底；依赖纪律=标准库 Executors（壳层不引 coroutine）；注入点：idleTimeoutMs/nowMs（SP-F-802 可注入时钟验收）；`createVault`=calibrate→create→unlock |
| 匹配引擎 | **复用** `sp-nmhost/src/match_url.rs`（parse_url/score/rank，SP-F-706 语义与测试）——见 §3 结构决策 |

## 3. 结构性决策：URL 匹配引擎位置

现状：match_url 位于 sp-nmhost（host 专用 crate），Android 需要同一语义（对齐 SP-F-706/803，防两套实现漂移）。

- 方案（选定向）：**抽取 match_url → sp-core**（纯函数、零依赖，天然属 core 域）；sp-nmhost 与 sp-kffi 同源引用；`cargo test` 单测随迁。属重构 PR：只搬移不改语义，T-706 用例跟着走。
- 备选（弃用）：Android 侧复制实现（漂移风险，两套反例测试，违背单一权威原则）；独立 sp-match crate（过度拆分，core 已含 entry/strategy 等纯函数域）。
- Autofill 上下文映射：`FillContext` 的 webDomains（Chrome 等浏览器提供）→ parse_url；非浏览器 App 无 web 域名时仅按 packageName 精确匹配条目 url 的 host==包名（宽松匹配列为 backlog，不做子串猜测）。

## 4. 生命周期与机密边界（SP-F-802/804）

```
Autofill 请求 ──► VaultSession（未解锁?）──► UnlockActivity（凭据仅内存）
     │                    │ 解锁成功: open_db → Vault 驻留
     │                    ▼
     └──► 匹配(rank) ──► find_entry 单条取回 ──► Dataset（RemoteViews 不回显明文）
                              │
              空闲 300s ──► vault.lock()（actor drop，凭据随库零化）
```

- 凭据只经 intent/内存传递；解锁态只存在于 VaultSession 单例；进程被杀即失锁（无持久化恢复，backlog 的 Keystore 会话缓存另行 DR）。
- 日志：任何机密值/凭据不进 Logcat（延续 spike 壳纪律，代码审查项 SP-F-804）。

## 5. 风险清单（v0.1）

| # | 风险 | 等级 | 缓解 |
|---|---|---|---|
| R14-1 | Autofill 精度：浏览器 WebView/账号表单结构差异大 | 高 | M1 验收锚定 Chrome + 演示 App 双链路；结构解析启发式先行，真机清单签字 |
| R14-2 | webDomains 缺失的 App（非浏览器）：仅包名匹配覆盖面窄 | 中 | MVP 明示"仅浏览器+包名精确匹配"；宽匹配 backlog |
| R14-3 | 厂商定制（三星/小米）Autofill 行为差异 | 中 | 真机验证包含 ≥1 台非 Pixel；问题进 backlog |
| R14-4 | 解锁 UX：Autofill 请求高频触发解锁 | 低 | 会话 300s 复用；解锁 Activity 单实例 |
| R14-5 | match_url 抽取触碰 sp-nmhost（公开 API 纪律） | 低 | 纯搬移 + 测试随迁；sp-nmhost 对外行为不变（re-export 保持） |

## 6. 里程碑拆分（M1 内部）

1. **M1a** ✅（2026-10-27）：match_url 抽取进 sp-core（纯搬移 + 测试随迁，sp-nmhost 保留同路径 glob 再导出，`crate::match_url::xxx` 引用不变）；workspace 69 测试全绿。
2. **M1b** ✅（2026-10-27）：VaultSession（空闲 300s 锁库 + 懒检查，SP-F-802）+ UnlockActivity（手动解锁 + Autofill 认证回路）+ SpAutofillService（表单启发式/webDomain/包名匹配 + Dataset + 认证回路）+ DemoLoginActivity 演示链路；KDF 校准 `calibrate_mobile`（DR-010 终态：m=64MiB/p=4 固定，t∈[2,8] 校准至 target_ms，钳制边界测试）。**模拟器验证：service 触发 → 单源匹配（包名 bare-host）→ 数据集选择 → 字段填充成功（日志仅记值长度，SP-F-804）**。workspace 72 测试全绿。
3. **M1c** ✅（2026-10-28，尾项已闭合）：KDF 真机基线回填（含 HyperOS 夜间降频 8–18s 方法论发现，测试方案 §7/§8）；**原生路径真机闭环 ✅**（解锁→包名匹配→填充）；浏览器路径实测：Quark 不上报结构（R14-1）、**Chrome 151 on MIUI 零 onFillRequest（ROM 策略门禁，R14-3）**——按"ROM 阻断"闭合，非本项目缺陷（原生路径同设备全通）；webDomain 真机验证转"非 MIUI 设备"前置。
4. **M1d** ✅（2026-10-28，步骤 5 M1 正式化）：壳层重组——e2e harness 自 MainActivity 分离至 TestActivity（凭据 extras 契约迁移 + 新增 `init_vault` 校准链路断言、`idle_timeout_ms` 超时锁库注入测试，SP-F-802"可注入时钟"验收闭合）；MainActivity 重组为产品壳入口（状态面板，不再自动跑断言）；SpApplication 进程级加载 libsp_kffi.so（Autofill 服务先于 Activity 场景兜底）；exported 最小化（演示/解锁 Activity 非导出，TestActivity 保留导出供 adb 直驱——API 34 起 shell 无法启动非导出组件）；UnlockActivity 双模式（解锁/首建校准：双密码校验、校准参数回显、FfiException 分类错误呈现、会话状态显示）；FfiKdfParams::default 降级为"跳过校准"兜底（DR-011，终态=首建强制校准不静默降级）；**回归发现并修复认证 Dataset presentation 缺失**（API 33+ 强制非空，步骤 4 未触达分支）——修复后锁定→认证→解锁→回传→填充全链路模拟器回归通过；机密边界复查（无 SharedPreferences、result.txt 无凭据值、日志仅类别/长度/非机密参数）。
5. backlog：Autofill 保存场景（新密码入库）、宽匹配、Keystore/生物识别（独立 DR）、CI 模拟器 e2e job（脚本化已具备：TestActivity 断言集 + fixture 推送流程，需 runner 装配 secrets 与 KVM 时间预算）。
