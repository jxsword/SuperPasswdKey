# SuperPasswdKey 软件需求规格说明书（SRS）

- 版本：**V1.2（2026-10-27，V1.0 定稿三轮审核见 docs/reviews/srs_review.md；V1.1 变更=F1 扩展升级为 SP-F-7xx/DR-006；V1.2 变更=Android 从非目标升级为正式需求 SP-F-8xx/DR-009）**
- 开发模型：瀑布（三轮审核：完整性→一致性→可执行性，记录见 `docs/reviews/srs_review.md`）
- 需求来源：《docs/research/自研需求基线_SuperPasswdKey.md》（B1–B22）、决策记录 DR-001~005、2 天冲刺计划两轮复审修正
- 编号体系：`SP-F-1xx` 库与条目｜`SP-F-2xx` CLI｜`SP-F-3xx` F8 注入｜`SP-F-4xx` 导入器｜`SP-F-5xx` 导出｜`SP-F-6xx` TOTP｜`SP-F-7xx` 浏览器扩展（V1.1 新增）｜`SP-F-8xx` Android（V1.2 新增）｜`SP-N-xxx` 非功能｜`SP-C-xxx` 约束
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

**SP-F-104 附件（P1→本期 P0 简化版，B21，confirmed）**
- 描述：`sp attachment add/get`；附件存入 KDBX4 binaries（格式层加密）。
- 验收：1MB 附件 add/get 往返无损；KeePassXC 可见。

**SP-F-105 解锁（P0，B3+冲刺复审修正 1，confirmed）**
- 描述：每次命令交互输入主密码 + `--keyfile`（或默认库绑定的 keyfile 路径记录于 config）；派生密钥仅存内存，进程退出即清（zeroize）。
- 验收：错误密码/缺失 keyfile → 明确错误与退出码 2（SP-F-205）；内存清零可由代码审查与 debug 断言验证（SP-N-201）。

**SP-F-106 单库与默认库配置（P0，冲刺复审修正 4，confirmed）**
- 描述：`--db` 参数覆盖 > config.toml 默认库路径；`sp init` 首次引导写入配置。
- 验收：无 `--db` 且无配置 → 引导式提示；配置文件损坏 → 明确报错不崩溃。

**SP-F-107 库兼容与损坏处理（P0，derived——格式互操作承诺的必然要求）**
- 描述：打开非 KDBX4 / 更高格式版本 / 损坏库文件时，给出明确错误（退出码 1）与可行动提示；不静默失败、不尝试修复写回。
- 验收：损坏文件（截断/坏魔数/未知版本）三类样例的错误路径测试。

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

> 设计见 design_docs/10/11/12；F1 从 §2.3 非目标升级为正式需求（步骤 3）。范围：Chrome（MV3）单页登录表单填充；iframe/SPA/自动保存/Firefox 列 backlog。

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
