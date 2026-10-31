# SuperPasswdKey 软件需求规格说明书（SRS）

- 版本：**V1.7（2026-10-31，V1.0 定稿三轮审核见 docs/reviews/srs_review.md；V1.1 变更=F1 扩展升级为 SP-F-7xx/DR-006；V1.2 变更=Android 从非目标升级为正式需求 SP-F-8xx/DR-009；V1.3 变更=桌面 Windows 批次升级为正式需求 SP-F-9xx/DR-013 批次 1；V1.4 变更=登录自动保存升级为正式需求 SP-F-708，step8；V1.5 变更=桌面 GUI 前端选型 Vue3+Vite（DR-014，修订 SP-F-901"静态零 Node"表述）+ 新增 SP-F-908 桌面 GUI 条目浏览 MVP；V1.6 变更=新增 SP-F-909 桌面 GUI 条目写路径（S1b-1：创建/编辑/删除 + 显示/放大显示交互）；V1.7 变更=新增 SP-F-108 标签分类（KDBX4 原生 Tags 全栈：CLI+GUI，S1b-2）；V1.8 变更=新增 SP-F-910 桌面 GUI 附件管理 + SP-F-909 扩展自定义字段编辑 UI（S1b-2b））**
- 开发模型：瀑布（三轮审核：完整性→一致性→可执行性，记录见 `docs/reviews/srs_review.md`）
- 需求来源：《docs/research/自研需求基线_SuperPasswdKey.md》（B1–B22）、决策记录 DR-001~005、2 天冲刺计划两轮复审修正
- 编号体系：`SP-F-1xx` 库与条目｜`SP-F-2xx` CLI｜`SP-F-3xx` F8 注入｜`SP-F-4xx` 导入器｜`SP-F-5xx` 导出｜`SP-F-6xx` TOTP｜`SP-F-7xx` 浏览器扩展（V1.1 新增）｜`SP-F-8xx` Android（V1.2 新增）｜`SP-F-9xx` 桌面 Windows（V1.3 新增）｜`SP-N-xxx` 非功能｜`SP-C-xxx` 约束
- 追溯：需求 ↔ 设计 ↔ 测试见 `docs/requirements_traceability_matrix.md`
- 载体：本文档随代码入仓；范围 = 步骤 1（Windows CLI 全功能），后续步骤见 §10

---

## 1. 引言

### 1.1 目的
定义 SuperPasswdKey v0.1（MVP）的功能与非功能需求，作为设计、实现、测试的唯一需求依据。

### 1.2 产品定位
本地优先、零知识、可 KDBX4 互操作的密码/机密管理器 CLI；加密与解密全部本地完成；云端/服务器只存密文；主密码与密钥文件永不出本地进程。

### 1.3 术语
| 术语 | 定义 |
|---|---|
| 库（database） | 单个 KDBX4 加密数据库文件（.kdbx） |
| 主密码 | 用户记忆的第一因子 |
| 密钥文件（keyfile） | 用户持有独立渠道保管的第二因子文件 |
| 引用 URI | `sp://<库>/<条目>/<字段>` 形式的密钥引用，值不落盘 |
| 解锁 | 用主密码+密钥文件完成 KDF 派生并打开库 |
| 会话 | 单次进程内的解锁状态（v0.1 不跨进程持久化） |

### 1.4 参考文档
KDBX4 格式规范（keepass.info/help/kb/kdbx_4.html，2026-10-25 核实）；RFC 6238（TOTP）；RFC 8439 / draft-irtf-cfrg-xchacha（KAT 来源）；Argon2 参考向量；DR-001~005。

## 2. 总体描述

### 2.1 用户与场景
单人开发者/个人用户；在 Windows（Git Bash/PowerShell/cmd）终端管理密码库；脚本与 CI 中以临时环境变量方式使用机密（F8）。

### 2.2 运行环境
- 开发/运行：Windows 10+ x64（一等公民）；代码保持 POSIX 兼容（步骤 2 WSL/Linux 验证）。
- 互操作对端：KeePassXC ≥2.7（keepassxc-cli，测试与用户侧双向迁移）。
- 依赖策略：全部成熟 crate（keepass_ng、RustCrypto、clap、arboard）；不自造算法。

### 2.3 v0.1 边界（非目标）

> V1.1 变更：浏览器自动填充（F1）已升级为正式需求（§3.7，SP-F-7xx），自本节非目标移除。
> V1.2 变更：Android（UniFFI 内核复用 + M1 系统 Autofill）已升级为正式需求（§3.8，SP-F-8xx），自本节非目标移除（DR-009；spike 事实见 design_docs/13）。iOS 仍为非目标。
> V1.3 变更：桌面 Windows 托盘壳（DR-013 批次 1）已升级为正式需求（§3.9，SP-F-9xx）。macOS 桌面仍为非目标（延后批次）。

私有部署服务器（F6，后置）、iOS、多库（B19）、1pux 解析、跨进程会话缓存（注：native messaging host 进程内会话缓存不属"跨进程持久化"，见 DR-006）、Shell Plugins（B8）、Watchtower（B10）、passkey 管理（B11）、共享（B15）、密码健康检查（B22）、生物识别解锁（B13，Android Keystore/生物识别同属此 backlog）、应急包 PDF（B18）。以上仅做接口预留（§8）。

## 3. 功能需求

> 字段说明：优先级 P0=本期必须；验收标准=测试用例的直接依据；来源=基线条目+决策记录；状态 confirmed=用户明确/基线既有。

### 3.1 库与条目（SP-F-1xx）

**SP-F-101 库初始化（P0，来源 B1/B2/B3+DR-003，confirmed）**
- 描述：`sp init` 在指定路径创建 KDBX4 库；Argon2id 参数可配（**下限 m=64MiB**；默认参数由 init 交互基准校准至派生约 700ms，SP-N-302）；主密码+密钥文件双因子。
- 输入：`--db <路径>`（或默认库路径）、交互主密码、`--keyfile <路径>`（可选生成新 keyfile）。
- 处理：KDF 参数校验（m≥64MiB）→ 创建库 → 写入库级元数据（名称、创建时间、版本）。
- 输出：.kdbx 文件；默认库路径写入 `%USERPROFILE%\.config\sp\config.toml`。
- 验收：创建的库可被 KeePassXC 2.7+ 正常打开（Argon2id 参数完整保留）；重复 init 拒绝（除非 `--force`）。

**SP-F-102 条目 CRUD（P0，B1，confirmed）**
- 描述：`sp item create/edit/delete/list`；类型化条目（login/secure-note/ssh-key/自定义字段）；**全字段加密**（标题/URL/备注等元数据纳入，红线 #4）。
- 字段：title、username、password、url、notes、自定义字段（名/值/受保护位）、history。
- 验收：CRUD 后 round-trip（KeePassXC 读回）字段无损；删除进入回收站语义；单库条目数 1000 时 list ≤500ms（SP-N-301）。

**SP-F-103 修改历史（P0，B1，confirmed）**
- 描述：每次 edit 保存旧版本为 history 条目（逐字段时间戳）。
- 验收：history 在 KeePassXC 中可见可回溯；`sp item get --history` 列出。

**SP-F-104 附件（P1→本期 P0，B21，confirmed；命令族补全 2026-10-29）**
- 描述：`sp attachment add/list/get`——add 支持对已有条目增补/覆盖附件（同名覆盖），list 列举附件名，get 取回为文件（ACL 限当前用户）或原始字节 stdout（可管道、不落盘）；`sp item create --attach` 保留为建条目时附加；附件存入 KDBX4 binaries（格式层加密）。
- 验收：1MB 附件 add→list→get 双通道（文件/stdout）往返无损；同名覆盖生效；错误路径（条目/附件缺失 → 退出码 3，错主密码 → 退出码 2）；KeePassXC 可见。

**SP-F-105 解锁（P0，B3+冲刺复审修正 1，confirmed）**
- 描述：每次命令交互输入主密码 + `--keyfile`（或默认库绑定的 keyfile 路径记录于 config）；派生密钥仅存内存，进程退出即清（zeroize）。
- 验收：错误密码/缺失 keyfile → 明确错误与退出码 2（SP-F-205）；内存清零可由代码审查与 debug 断言验证（SP-N-201）。

**SP-F-106 单库与默认库配置（P0，冲刺复审修正 4，confirmed）**
- 描述：`--db` 参数覆盖 > config.toml 默认库路径；`sp init` 首次引导写入配置。
- 验收：无 `--db` 且无配置 → 引导式提示；配置文件损坏 → 明确报错不崩溃。

**SP-F-107 库兼容与损坏处理（P0，derived——格式互操作承诺的必然要求）**
- 描述：打开非 KDBX4 / 更高格式版本 / 损坏库文件时，给出明确错误（退出码 1）与可行动提示；不静默失败、不尝试修复写回。
- 验收：损坏文件（截断/坏魔数/未知版本）三类样例的错误路径测试。

**SP-F-108 标签分类（P1，S1b-2，confirmed）**
- 描述：条目标签基于 KDBX4 原生 Tags 字符串列表（keepass_ng get_tags/get_tags_mut，KeePassXC 互认），全栈打通：EntryModel.tags（校验：trim 后非空）→ CLI（`sp item create --tag` 可重复、`sp item list --tag` 精确过滤大小写不敏感、`sp item get` 显示 Tags 行）→ sp-kffi（FfiEntry/FfiEntrySummary/FfiEntryDraft 携带 tags，UniFFI 面变更须同步重生 Android Kotlin 绑定）→ 桌面 GUI（创建/编辑表单逗号分隔输入、列表标签徽标点击过滤、详情标签行、搜索匹配标签）。
- 验收：kdbx/core 校验单测；kffi e2e 标签 roundtrip（含编辑覆写）；desktop session 测试（创建/编辑/摘要可见/空白标签拒绝）；CLI 手册同步；KeePassXC 互操作（Tags 为 KDBX4 原生字段，roundtrip 既有对拍覆盖格式层）。

### 3.2 CLI（SP-F-2xx）

**SP-F-201 命令族（P0，B5，confirmed）**
- 描述：`sp init/item/read/clip`（v0.1）；`sp run/inject`（3xx）；`sp import/export`（4xx/5xx）。全局参数：`--db`、`--keyfile`、`--quiet`、`--no-color`。
- 验收：`sp --help`/`sp <cmd> --help` 完整；命令树与《design_docs/03》一致。

**SP-F-202 单值读取（P0，B5，confirmed）**
- 描述：`sp read <uri>`；`sp://<条目>/<字段>`（库可省略=默认库）；`--out-file <路径>`、`-n`（去尾换行）。
- 验收：stdout 输出值本体（无额外装饰）；`--out-file` 在 Windows 下 ACL 限定当前用户（等价 POSIX 0600，差异写入 help 文案）。

**SP-F-203 剪贴板（P0，B20，confirmed）**
- 描述：`sp clip <uri>` 复制值到剪贴板，默认 30s 后自动清除（`--timeout` 可配）。
- 验收：超时后剪贴板内容被清（置空串）；Windows 剪贴板历史（Win+V）不受控的场景在 help 中明示（平台限制，非缺陷）。

**SP-F-204 退出码与错误模型（P0，冲刺复审修正 5，confirmed）**
- 描述：错误统一输出 stderr；退出码：0 成功 / 1 一般错误 / 2 未解锁或凭据错误 / 3 条目未找到 / 4 参数错误；`--quiet` 抑制非错误输出。
- 验收：每个错误分支的退出码有集成测试断言。

**SP-F-205 版本信息（P0，冲刺复审修正 6，confirmed）**
- 描述：`sp --version` 输出 SemVer + git describe（build.rs 注入）。
- 验收：从 tag 构建的二进制版本号与 tag 一致。

### 3.3 F8 临时环境变量注入（SP-F-3xx）

**SP-F-301 sp run（P0，B6+基线 §3.2，confirmed）**
- 描述：`sp run --env-file <路径> -- <command>`：解析 .env 中的 `sp://` 引用 → 解锁并内存解密 → 以环境变量注入子进程 → 子进程退出即销毁；父 shell 永不获得值。
- 处理约束（十项验收核心子集，全部 P0）：进程级注入；系统关键变量（HOME/PATH/USER/TERM/SHELL/XDG_*/LC_*）永不覆盖；.env 语法对齐 op 规则（UTF-8/#注释/引号/$VAR 展开/\$ 转义/多行值）；输出遮蔽默认开启（子进程 stdout/stderr 中出现的密钥值替换为 `<redacted>`，`--no-masking` 显式关闭）；非 POSIX 变量名（连字符等）设为 `_` 前缀化处理。
- 验收：Git Bash/PowerShell 下 `sp run -- printenv <KEY>`（cmd: `echo %KEY%`）输出真实值；进程结束后父 shell `echo $KEY` 为空；stdout 中出现的值被遮蔽；集成测试覆盖以上全部。

**SP-F-302 sp inject（P0，B6，confirmed）**
- 描述：`sp inject -i <模板> -o <产物>`：模板中 `{{ sp://... }}` 替换为真实值；产物默认 0600/ACL 限当前用户；`--stdout` 流式输出不落盘。
- 验收：拼接示例（`postgres://{{ sp://db/user }}:{{ sp://db/pw }}@host/db`）正确渲染；产物权限符合平台语义。

**SP-F-303 威胁边界文档（P0，基线 §3.2#8，confirmed）**
- 描述：README 明示"同 OS 用户进程可读取子进程环境变量"的固有边界；不夸大安全承诺。
- 验收：文档评审项（可执行性审核检查）。

### 3.4 批量导入（SP-F-4xx）

**SP-F-401 解析器（P0，B16+冲刺复审，confirmed）**
- 描述：四种 P0 来源：① .env/dotenv 语法文件与目录递归；② 通用 CSV（列映射向导，默认映射 KeePassXC 10 列表头）；③ Bitwarden JSON（核心字段：login/uri/notes/custom fields/TOTP；附件与 passkey 后置）；④ 当前进程环境变量快照——**内存枚举，不产生临时文件**（红线 #4）。
- 发现引擎：gitleaks 规则 TOML 静态内嵌（MIT，235 规则基线），输出统一 finding 模型 `{source, file, line, ruleId, secret, fingerprint}`。
- 验收：每个解析器 10+ 样例单测 + proptest 属性测试（合法输入不崩溃、字段往返一致）。

**SP-F-402 三级去重（P0，B16+基线 §4.3，confirmed）**
- 描述：① 值哈希精确匹配；② URL+用户名合并；③ 指纹/前缀匹配（私钥指纹、key 前缀）。冲突不清除：全部保留并打 `duplicate/conflict` 标记人工裁决。
- 验收：构造三级冲突样例，导入报告与标记断言。

**SP-F-403 分类与导入（P0，B16，confirmed）**
- 描述：按 `来源类型/项目路径/规则名` 聚类入分组；`sp import --dry-run` 预览；导入完成输出"需人工抽查"清单（未逐字段声明的映射）。
- 验收：dry-run 无副作用；导入报告含计数/分组/抽查项。

### 3.5 批量导出（SP-F-5xx）

**SP-F-501 加密归档导出（默认，P0，B17+基线 §5，confirmed）**
- 描述：`sp export --out <文件>` 默认输出**加密归档**（zip 容器 + age 口令加密；口令交互输入确认两次）；含全部条目+附件+TOTP+history。
- 验收：归档可用 age 解密还原；字段完整性清单输出（哪些类型/字段包含）。

**SP-F-502 明文导出（可选，P0，B17，confirmed）**
- 描述：`sp export --plaintext`（JSON）：显式 flag + 交互重输主密码 + 警告文案 + `--stdout` 流式（避免临时文件）。
- 验收：无 `--stdout` 时拒绝静默落盘（必须二次确认）；流式输出到管道的端到端测试。

**SP-F-503 字段损失明示（P0，基线 §5#5，confirmed）**
- 描述：每次导出输出摘要：包含范围 + 不在该格式中的字段清单。
- 验收：摘要与实际导出内容一致性断言。

### 3.6 TOTP（SP-F-6xx）

**SP-F-601 TOTP 生成（P0，B14，confirmed）**
- 描述：条目 `otp` 自定义属性（KeePass 约定：seed + period/digits 编码）；`sp item get <条目> --totp` 本地 RFC 6238 生成（6 位/30s 默认，尊重属性配置）。
- 验收：与 RFC 6238 官方测试向量一致；与 KeePassXC 显示的当前码一致（round-trip 对拍）。

### 3.7 浏览器扩展（SP-F-7xx，V1.1 新增，来源 B4 + DR-006）

> 设计见 design_docs/10/11/12；F1 从 §2.3 非目标升级为正式需求（步骤 3）。范围：Chrome（MV3）单页登录表单填充；iframe/SPA/Firefox 列 backlog。**自动保存已升级为正式需求 SP-F-708（step8，V1.4）**。

**SP-F-701 native messaging host（P0，B4+DR-006，confirmed）**
- 描述：`sp-nmhost` 长驻 host 进程（Rust，复用 sp-core/sp-kdbx）；Windows 注册表（HKCU）注册/注销脚本；manifest `allowed_origins` 绑定扩展 ID（无通配），host 启动时校验调用方 origin 双重防线。
- 验收：origin 不符时 host 拒绝服务退出；注册/注销脚本在 Chrome 与 Edge 均生效（HKCU 键与 manifest 文件落位正确）。

**SP-F-702 协议（P0，B4+DR-006，confirmed）**
- 描述：stdin/stdout 长度前缀 JSON（32 位本机字节序长度 + UTF-8 JSON）；消息信封含 `v`/`codec`/`payload`，codec 为 P1（明文）/P2（TweetNaCl box）适配器切换点（MVP 实现 P1）；动作集：sp.ping/status/unlock/lock/get-logins/get-secret + sp.event；host 不提供任何"导出全库/全字段枚举"能力。
- 验收：帧编解码 round-trip 与边界（超长断开/非法输入不 panic，fuzz 覆盖）；未知 action 与未知版本拒绝；主密码仅 unlock 消息传输一次。

**SP-F-703 会话状态机（P0，B4+DR-006，confirmed）**
- 描述：host 解锁一次缓存凭据因子（zeroize），每请求重开库解密（每请求含 KDF ~700ms，如实声明）；空闲超时（默认 300s，可配）自动锁库并通知扩展；MV3 service worker 生命周期以保活策略消化（Chrome ≥120 语义，断线即会话失效）。
- 验收：空闲超时锁库（可注入时钟测试）；锁后机密操作返回 `locked`；进程退出凭据清零（代码审查 + zeroize 语义）。

**SP-F-704 表单探测与填充（P0，B4，confirmed）**
- 描述：content script 单页登录表单探测（密码字段 + 用户名启发式 + autocomplete 信号）；填充经原生 setter + input/change 事件（受控组件兼容）；TOTP 可选填充（`autocomplete=one-time-code`）。
- 验收：本地 demo 页真实登录填充成功（Chrome 实测清单签字，B4 出口）；探测规则 vitest 全分支覆盖。

**SP-F-705 popup 列表与搜索（P0，B4，confirmed）**
- 描述：popup 显示当前页匹配条目（title/username），本地搜索过滤；解锁/锁定状态条。
- 验收：解锁→列表→填充主链路手工清单通过；无匹配页显示空列表不猜测。

**SP-F-706 URL 匹配引擎（P0，B4，confirmed）**
- 描述：host 侧纯函数：scheme 白名单（http/https）、host 归一（含端口）、精确 host 匹配 > base domain 匹配（末两段对齐，防子串误配）、评分降序输出。
- 验收：Rust 单测覆盖全部正例与反例（含 `evilexample.com` 类子串误配拒绝）。

**SP-F-707 扩展端机密边界（P0，红线延伸，confirmed）**
- 描述：扩展端机密只进内存即用即弃；不进 chrome.storage/localStorage/临时文件/日志；content script 填充后引用即清。
- 验收：代码审查（无 storage API 写机密路径）+ 构建期 lint 规则约束。

**SP-F-708 登录自动保存（P1，step8，confirmed）**
- 描述：content script 在登录表单**提交时点**捕获用户名/密码（document 级 capture 监听，复用 SP-F-704 表单判定；stopPropagation/preventDefault/SPA 场景可命中），经内存消息直送 SW **纯内存暂存**（每 tab 一项、60s TTL、不写任何 storage，SP-F-707 同口径）→ 页内确认浮层（**密码不回显**；同页立即弹出 + 跨导航新页查询两条路径；忽略=本会话同 host 不再询问）→ 确认后经 `sp.save-login`（design_docs/11 §3）写库：host(含端口)+username 精确去重——命中更新（旧密码进 history）、未命中新增（标题=host，撞题消歧）；host 侧仅解析 URL（http/https 白名单），**不发起任何网络请求**；拒绝路径零请求零写入。旧 host（不支持该动作）回 badrequest → 浮层明示版本过旧。桌面端更新路径 notes/otp 保真、自定义字段清空（FFI 读回面限制，与 Android M2 同口径，design_docs/15 §9）。
- 验收：模拟握手测试（新增/更新/锁定拒绝/旧 host 容忍，PR #35）+ 扩展单测（暂存队列 tab 隔离/TTL/忽略维度）+ Chrome 手工清单（真实导航页与 preventDefault 页双场景、锁定重试、忽略静默——docs/reviews/）。

### 3.8 Android（SP-F-8xx，V1.2 新增，来源 DR-002 + DR-008/DR-009）

> 设计见 design_docs/13（内核绑定与线程模型，spike 转正）；M1 Autofill 设计见 design_docs/14。Android 从 §2.3 非目标升级为正式需求；Keystore/生物识别解锁不在本组（backlog，独立 DR）。

**SP-F-801 Android 内核绑定（P0，DR-002/DR-009，confirmed）**
- 描述：`crates/sp-kffi` UniFFI 绑定复用 sp-core/sp-kdbx 内核（不复制加密代码）；线程模型=线程封闭 actor（keepass_ng Database !Send，每 Vault 一工作线程，命令 channel 派发）；双因子（主密码+keyfile）完整透传；摘要面不含密码（对齐 SP-F-706 语义）。
- 验收：模拟器（x86_64）e2e 断言全过（开库/全字段读回/摘要/TOTP/NotFound/错密码 Credentials）；主机侧 cargo test 覆盖同一断言面；aarch64 release 交叉编译通过。

**SP-F-802 Android 解锁会话（P0，红线#5#7，confirmed）**
- 描述：解锁 Activity 收集主密码+keyfile；解锁后库句柄驻留内存（actor），空闲超时自动锁库并 drop（凭据随 drop 零化）；凭据不落 SharedPreferences/文件/日志。
- 验收：空闲超时锁库（可注入时钟）；锁后机密操作拒绝；代码审查无机密持久化路径。

**SP-F-803 系统 Autofill 对接（P0，M1，confirmed）**
- 描述：`AutofillService`（android.service.autofill）响应系统填充请求：按 web 域名/包名匹配库内条目（URL 匹配语义对齐 SP-F-706），返回用户名+密码数据集；填充数据仅经 FFI 单条取回。
- 验收：模拟器主链路（Chrome/演示 App 表单）填充成功；不匹配场景返回空集不猜测；真机精度实测列入验收前置。

**SP-F-804 Android 机密边界（P0，红线延伸，confirmed）**
- 描述：机密在 Android 侧只进内存即用即弃；不进 SharedPreferences/日志/剪贴板历史；JNI 边界的 String 不可零化限制（R13-1）如实写入用户文档（威胁边界声明，对齐 design_docs/09§4 诚实边界原则）。
- 验收：代码审查（无持久化写机密路径）+ 文档评审。

**SP-F-805 Android Autofill 保存场景（P0，M2，DR-012 A1，confirmed）**
- 描述：`AutofillService.onSaveRequest` 解析表单凭据值 → 保存确认页（显示 host/包名+用户名，密码不回显）→ 归属（webDomain/包名，同填充口径）→ 去重（同 host+用户名已存在则更新密码进 history，否则新增）→ `add_entry` + `save_session`（凭据因子随 open_db 留驻 sp-kffi actor 工作线程，锁库随 actor drop 零化；主密码内存驻留窗口=会话期，与 SP-F-804 审查口径一致并如实声明）。填充响应须携带 SaveInfo（用户名+密码双必填）声明保存意图。拒绝路径（系统"No thanks"/确认页取消）零写入。
- 验收：模拟器演示链路（输入新凭据 → 系统保存建议 → 确认入库 → 重开库可读）；不保存路径行为正确（零写入）；e2e 断言（锁库后 save_session 拒绝；保存闭环重开读回）进 TestActivity；主机侧 cargo test 覆盖 save_session 语义。

### 3.9 桌面 Windows（SP-F-9xx，V1.3 新增，来源 DR-013 批次 1）

**SP-F-901 桌面托盘壳（P0，DR-013，confirmed；DR-014 修订前端形态）**
- 描述：Tauri v2 托盘常驻（独立 workspace，仅 MSVC）；关窗=隐藏（进程与会话保留）；托盘菜单=显示解锁窗/立即锁定/设置/退出；前端 Vue 3 + Vite 构建（DR-014，v0.3.0 静态零 Node 页为过渡形态；运行期零 Node，产物静态打包，机密不进前端存储）。
- 验收：`cargo tauri build` 产物托盘常驻、菜单四项行为正确；linux CI 不受影响（desktop 独立 workspace + 独立 job）。

**SP-F-902 桌面解锁会话（P0，DR-013，红线#5，confirmed）**
- 描述：**每次进程启动首次主密码**（解锁窗完成）；解锁 → 凭据因子随 open_db 留驻 sp-kffi actor（DR-012 A1 同源），锁库随 actor drop 清零；无生物、无落盘（不触碰红线 #5 定向豁免——该豁免仅对 Android 生物路径生效）。
- 验收：解锁真实 kdbx；错误密码拒绝且会话保持锁定；锁库后因子清零（actor drop 语义，复用 sp-kffi 测试）。

**SP-F-903 桌面双超时（P0，DR-013，confirmed）**
- 描述：idle（机密操作 touch 刷新）与 absolute（自解锁时刻）双超时独立可配（默认 300s/1800s，对齐 DR-006 与用户 30min 示例口径）；任一到即锁库，再次使用须重新主密码；解锁态翻转即向后端/扩展通知。
- 验收：可注入超时单测（idle=0 解锁后首次访问即锁）；NM 侧 `sp.event locked` 通知（watcher）；重新解锁重置双计时器。

**SP-F-904 桌面设置持久化（P1，DR-013，confirmed）**
- 描述：%APPDATA%/SuperPasswdKey/settings.json 仅存非机密（idle_ms/absolute_ms/autostart）；serde_json 读写；损坏/缺失回落默认值不崩；超时配置重启生效；自启开关即时生效。任何凭据不入设置文件。
- 验收：roundtrip/损坏回落/BOM 兼容单测；改设置后 `settings.json` 落盘正确。

**SP-F-905 桌面开机自启（P1，DR-013，confirmed）**
- 描述：HKCU\Software\Microsoft\Windows\CurrentVersion\Run 用户级注册表值（免管理员，手写 registry）；值名 SuperPasswdKey=带引号 exe 全路径；settings.json autostart 字段为偏好单一来源，启动与保存时对账（手删注册表值重启即恢复，关偏好即清残留）。
- 验收：独立值名注册表 roundtrip 单测；`reg query` 可验证；关闭自启后注册表值清零。

**SP-F-906 NM host 单进程嵌入（P0，DR-013，confirmed）**
- 描述：sp-desktop.exe 同时是托盘 GUI 进程与 native messaging host（注册表 manifest 指向它）——浏览器拉起的实例在专用线程跑 NM 请求循环，与会话管理器共享进程内同一会话（会话进程内闭环，免跨进程会话共享/IPC）；协议编解码/动作分发/请求循环/origin 校验复用 sp-nmhost 库目标（零复制）；锁定态机密请求 → 回 locked 语义 + 弹本实例解锁窗；stdin EOF（端口关闭）→ 进程退出。旧 sp-nmhost 注册双轨保持可用。
- 验收：模拟 NM 握手测试（serve 循环 + 适配器全链路：ping→解锁→get-logins→get-secret→lock→locked）；`cargo tauri build` 后 register.ps1 -Target desktop 注册 + 手工 Chrome 填充验证；旧 sp-nmhost 回归（既有测试/注册不变）。

**SP-F-907 桌面浏览器填充链路（P0，DR-013，confirmed）**
- 描述：sp.get-logins 按 URL 匹配条目摘要（复用 match_url 语义，SP-F-706 单源）+ sp.get-secret 单条显式取回（password/totp 白名单）；动作纪律与 SP-F-702 单源（无全库导出、锁定拒绝、机密不进错误链、成功请求刷新空闲计时）；NM 侧解锁目标=桌面单库（%APPDATA%/SuperPasswdKey/vault.kdbx），keyfile 仅经 nm-host-config.json（不经管道）。
- 验收：匹配命中与反例（子串误配不命中）；get-logins 响应不含密码字段；锁定态 get-logins/get-secret 拒绝。

**SP-F-908 桌面 GUI 条目浏览（P1，DR-014，S1a MVP，confirmed）**
- 描述：解锁后主窗提供条目列表（标题/用户名/URL/TOTP 标记，经 sp-kffi list_entry_summaries）+ 即时搜索（客户端过滤）+ 详情只读视图（find_entry 含密码瞬态展示、TOTP 动态码 30s 轮询、复制按钮经 Rust clip_copy=arboard+30s 超时清空，对齐 CLI clip 语义）；所有机密操作经 Session 惰性超时检查（过期即锁库，SP-F-903 口径）；机密不进前端存储（SP-F-707 口径）；MVP 不含写路径（条目 CRUD/附件/TOTP 编辑在 S1b）。
- 验收：真实库解锁→列表→搜索→详情→TOTP 复制全链路；锁定态请求返回 locked 语义；组件测试（vitest）+ session 测试回归全绿；CI 七 job 全绿。

**SP-F-909 桌面 GUI 条目写路径（P1，S1b-1，confirmed；S1b-2b 扩展自定义字段编辑 UI）**
- 描述：GUI 内条目全生命周期——创建（"新的项目"表单）、编辑（详情页编辑表单，标题可改；旧快照自动进 history，SP-F-103 语义经 sp-kdbx edit_entry 单源）、删除（确认对话框）；表单校验对齐 CLI 语义（标题必填；otp 必须 otpauth:// 开头；同名允许，靠列表/搜索区分；S1b-2b 起自定义字段名称非空且非 KDBX 保留名，与 sp-core 同源）；详情交互增强对标 1Password：密码行下拉（显示=掩码↔明文切换、放大显示=全屏大字号模态、复制）；自定义字段完整读回并保真回传（sp-kffi Rust-only 面 find_entry_full，不经 UniFFI 导出、Kotlin 绑定零漂移）；S1b-2b 起表单提供自定义字段增删改行（v0.1 存储不支持 protected 标记持久化——DR-005，保护标记恒为 false 如实披露）；写入后经 save_session 零凭据保存（DR-012 A1 同源）；机密不进前端存储（SP-F-707 口径）。
- 验收：GUI 完成一条真实条目的创建→编辑（含改名/改密）→显示/放大显示→删除全生命周期；custom_fields 编辑后不丢（自动化断言）；双超时/锁定语义不回归；组件测试 + Rust session 测试全绿；CI 全绿。

**SP-F-910 桌面 GUI 附件管理（P1，S1b-2b，confirmed）**
- 描述：详情页附件区——列表（sp-kffi Rust-only 面 list_attachments，语义=sp_kdbx::list_attachments 单源）、添加（文件选择器读字节经 Rust add_attachment 同名覆盖，对齐 CLI sp attachment add 语义，save_session 零凭据保存）、下载（Rust 侧系统保存对话框 tauri-plugin-dialog → 落盘 + 限权收敛复用 sp-core::fsperms 单源——与 CLI attachment_get 同语义 icacls/0600）；锁定态拒绝；机密不进前端存储（附件字节经 invoke 瞬态传递，不落前端持久层）。
- 验收：kffi e2e（list/add/get roundtrip + 同名覆盖）+ desktop session 测试（roundtrip/覆盖/锁定拒绝）+ vitest（附件渲染/下载调用）；下载文件 `icacls` 核对仅当前用户可读（手工项）。

## 4. 非功能需求（SP-N，全部绑定测量方法）

| 编号 | 需求 | 量化指标 | 测量方法 |
|---|---|---|---|
| SP-N-301 | 读写性能 | 1000 条目 list ≤500ms；1000 条目+10×1MB 附件建库+全量读 ≤2s | `cargo test --release` 内计时断言（**仅本地必跑并记录结果；CI 不作阻塞断言**——runner 性能波动会假失败，CI 中以 `#[ignore]` 标注供手动触发） |
| SP-N-302 | KDF 派生 | 默认参数经 init 交互基准校准至约 **700ms**（可调；硬下限 m=64MiB） | init 校准流程 + 测试计时 |
| SP-N-303 | 解锁路径（除 KDF） | ≤50ms | 计时断言（不含 KDF 段） |
| SP-N-304 | 零网络 | 运行期无任何对外网络请求（同步由用户外部完成） | 代码审查（禁用网络 crate）+ 依赖审计（deny.toml bans） |
| SP-N-305 | 内存卫生 | 主密码/派生密钥 zeroize；Debug 断言不可恢复路径 | 代码审查 + secrecy 类型强制 + debug_assertions 测试 |
| SP-N-306 | 遮蔽完整性 | 注入子进程输出中的密钥值 100% 替换 | 集成测试：子进程 echo 全部注入值，断言无明文 |
| SP-N-307 | 测试矩阵 | 加密路径单测全覆盖；解析器 proptest；round-trip 全套；clippy `-D warnings`；deny/audit 零高危 | CI 门禁（SP-C-102） |
| SP-N-308 | 可移植性 | 同一代码库零修改编译于 Windows MSVC 与 Linux GNU；平台差异仅限文件权限/剪贴板后端抽象 | 步骤 2 WSL 构建验证（本期预留接口，抽象层设计见《02》） |
| SP-N-309 | 输出语言 | v0.1 CLI 错误与帮助信息为英文（脚本化友好、编码安全），README 与文档为中文 | assumption（国际化后置） |
| SP-N-310 | 进程加固 | SecVec/zeroize + VirtualLock 密钥页锁（失败降级警告）+ 进程缓解策略（ExtensionPointDisable/DynamicCode）+ 反调试告警；keepass_ng DOM 残留如实文档化 | design_docs/09§4；代码审查 + 行为测试（限制已如实声明） |
| SP-N-311 | 文件完整性基线 | config 记录库 SHA256 基线；打开成功静默更新；打开失败且哈希不符→外部修改提示；`sp doctor` 手动核查 | design_docs/09§5；行为测试 |
| SP-N-312 | 自动备份轮换 | 每次保存后复制到 `.sp-backup/`，滚动保留 10 份，只读属性 | design_docs/09§5；行为测试（11 次保存→10 份） |

## 5. 外部接口

| 接口 | 说明 |
|---|---|
| KDBX4（keepass_ng） | 库格式唯一载体；格式级加密即产品加密（**crate 选型经 H1 spike 终核后由 DR-005 定稿**；首选 keepass_ng，回退 keepass-rs） |
| keepassxc-cli | 测试对拍与用户迁移对端；`SP_KEEPASSXC_CLI` 环境变量覆盖路径，默认探测 `C:\Program Files\KeePassXC\`；密码经 stdin；测试 fixture 密码 `sp-test-only`（仅测试用途，可入仓） |
| OS 剪贴板 | arboard（Windows clipboard API）；Win+V 历史不受控已在 help 明示 |
| OS 文件权限 | Windows ACL 限定当前用户（等价 0600）；POSIX 平台 0600 |
| 终端 | rpassword 交互输入；UTF-8；Git Bash/PowerShell/cmd |
| 浏览器 native messaging（V1.1） | Chrome/Edge MV3 `connectNative`；帧=32 位本机字节序长度+UTF-8 JSON（上限 host→浏览器 1MB / 浏览器→host 64MiB）；host manifest 经 HKCU 注册表发现 |

## 6. 数据需求

- 库：KDBX4（Argon2id + AES-256/ChaCha20 分块 Encrypt-then-MAC，格式规范 2026-10-25 核实）。
- 条目 → KDBX 映射表：设计文档《02_sp-kdbx_KDBX4适配层.md》维护。
- 配置：`%USERPROFILE%\.config\sp\config.toml`（默认库路径、keyfile 路径、clip 超时）。
- 测试：fixture 库与样例文件入 `crates/*/tests/fixtures/`；生成脚本随仓。

## 7. 约束（SP-C）

| 编号 | 约束 | 来源 |
|---|---|---|
| SP-C-101 | 十条红线全项（基线 §6）：签名校验/可复现构建路径/分发纪律/零遥测+元数据加密/主密码不出进程/Argon2id+AEAD/双因子/零知识/量子余量/供应链卫生 | 基线 §6，confirmed |
| SP-C-102 | 质量门禁：clippy `-D warnings`、cargo-deny/audit 零高危、proptest 解析器、round-trip 全套进 CI | 冲刺复审修正 5，confirmed |
| SP-C-103 | 许可证 AGPL-3.0-only（DR-004）；依赖许可证兼容（keepass_ng 终核后记入） | confirmed |
| SP-C-104 | 分发：官方二进制优先；crates.io 仅源码渠道；不做竞价广告 | 红线 #3，confirmed |
| SP-C-105 | 换行：仓库强制 LF（.gitattributes），保障步骤 2 WSL | 冲刺复审修正 4，confirmed |
| SP-C-106 | 测试向量来源：RFC 8439 / draft-irtf-cfrg-xchacha / Argon2 官方向量 / crate 自带测试；禁止自造向量 | 冲刺复审修正 6，confirmed |

## 8. 扩展性与接口预留

- `sp://` URI：语法版本化（v0.1 = `<条目>/<字段>`，库段预留）；未来字段属性查询（`?attribute=otp`）。
- 条目模型：自定义字段天然扩展位（passkey/证书等未来类型）。
- 导入器：解析器注册表设计（新增来源即插）。
- 服务器轨（F6）：客户端侧 secrets 网关接口位（`sp read` 的后端抽象，v0.1 仅本地实现）。
- 后续路线：步骤 2 WSL/Linux（✅ v0.1.2-linux）→ 步骤 3 浏览器扩展（native messaging + 本填充引擎，V1.1 已升级为正式需求 §3.7）→ 步骤 4 Android（UniFFI 复用内核，V1.2 已升级为正式需求 §3.8，DR-009 转正）→ 步骤 5 macOS/iOS（远期）。

## 9. 待确认事项

（无——全部 pending 已于规划问答确认：瀑布/GitHub 私有仓/约 3 天/Markdown 入仓；解锁流降级、proptest 转正等经两轮冲刺复审 confirmed。）

## 10. 后续版本路线（v0.2+）

v0.2：1pux 解析器、指纹去重第三级增强、协议层 P2（TweetNaCl box 加密，DR-006 适配器切换点）、host 密钥复用开库（省每请求 KDF）；会话缓存决策已部分由 DR-006 收敛（host 进程内缓存+超时锁库，跨进程持久化缓存仍为 backlog）；v0.3：WSL/Linux 一等支持（步骤 2，✅ v0.1.2-linux）；v0.4：浏览器扩展（步骤 3，V1.1 提前实现）；服务器轨与移动端见基线 §9 里程碑。
