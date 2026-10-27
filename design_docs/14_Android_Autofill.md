# 14 Android M1：系统 Autofill 对接（SP-F-802/803）

- 状态：v0.1 骨架（2026-10-27，DR-009 转正后启动；实现中随设计定稿升 v1.0）
- 上游依赖：design_docs/13（UniFFI 绑定与 actor 线程模型，已转正）
- 范围：M1 = **填充读路径**（Autofill 响应库内条目）；Autofill 保存场景（新密码入库）backlog 单列

## 1. 目标与验收锚点

- SP-F-802：解锁会话（超时锁库、凭据清零、不落 SharedPreferences/日志）
- SP-F-803：AutofillService 按 web 域名/包名匹配条目，返回用户名+密码数据集；不匹配返回空集不猜测
- 验收：模拟器主链路（Chrome/演示 App）填充成功；真机精度实测为验收前置（DR-009）

## 2. 组件结构（android/ 壳内增量）

| 组件 | 职责 |
|---|---|
| `SpAutofillService`（android.service.autofill） | 接收 FillRequest → 提取 web 域名/包名 → 匹配 → 构造 Dataset（RemoteViews 不回显明文） |
| `UnlockActivity` | 凭据收集（主密码+可选 keyfile，文件选择器取路径）；解锁成功仅内存回传，不落盘 |
| `VaultSession`（单例） | 持有 kffi `Vault` + 最后活跃时间戳；空闲超时（默认 300s，对齐 SP-F-703）调 `lock()`；所有调用经协程 Dispatchers.IO（actor 阻塞调用不占主线程） |
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

1. **M1a**：match_url 抽取进 sp-core（重构 PR）+ FFI 面暴露 rank 匹配（或 Android 侧经绑定调用）→ 主机测试全绿。
2. **M1b**：VaultSession + UnlockActivity + SpAutofillService 骨架 → 模拟器演示 App 填充链路。
3. **M1c**：Chrome 真链路 + 真机验证包（KDF 复测 SP-N-301/302、Autofill 精度清单、后台行为）——**前置：用户提供真机**。
4. backlog：Autofill 保存场景（新密码入库）、宽匹配、Keystore/生物识别（独立 DR）。
