# sp CLI 完整手册（SuperPasswdKey 命令行）

> - 适用版本：**sp 0.1.0**（2026-10-29 构建核对）
> - 核对基准：本手册每个命令的参数表均逐项对照 `sp --help` 与各子命令 `--help` 的真实输出（附录 A 附覆盖核对表）；若程序输出与手册不一致，**以程序输出为准**。
> - Windows 下可执行文件名为 `sp.exe`，手册中统一写作 `sp`（用法完全一致）。

---

## 目录

- [1. 快速开始](#1-快速开始)
- [2. 全局参数与配置文件](#2-全局参数与配置文件)
- [3. `sp init` —— 初始化新库](#3-sp-init--初始化新库)
- [4. `sp item` —— 条目操作族](#4-sp-item--条目操作族)
  - [4.1 `sp item create` —— 创建条目](#41-sp-item-create--创建条目)
  - [4.2 `sp item get` —— 查看条目](#42-sp-item-get--查看条目)
  - [4.3 `sp item list` —— 列出标题](#43-sp-item-list--列出标题)
  - [4.4 `sp item delete` —— 删除条目](#44-sp-item-delete--删除条目)
- [5. `sp attachment` —— 附件操作（add / list / get）](#5-sp-attachment--附件操作add--list--get)
  - [5.1 `sp attachment add` —— 添加/替换附件](#51-sp-attachment-add--添加替换附件)
  - [5.2 `sp attachment list` —— 列出附件](#52-sp-attachment-list--列出附件)
  - [5.3 `sp attachment get` —— 取回附件](#53-sp-attachment-get--取回附件)
- [6. `sp read` —— 按 URI 读单值](#6-sp-read--按-uri-读单值)
- [7. `sp clip` —— 复制到剪贴板（超时清除）](#7-sp-clip--复制到剪贴板超时清除)
- [8. `sp run` —— 注入环境变量运行子进程](#8-sp-run--注入环境变量运行子进程)
- [9. `sp inject` —— 模板渲染](#9-sp-inject--模板渲染)
- [10. `sp import` —— 批量导入](#10-sp-import--批量导入)
  - [10.1 导入总览与去重报告](#101-导入总览与去重报告)
  - [10.2 `import dotenv`](#102-import-dotenv)
  - [10.3 `import csv`](#103-import-csv)
  - [10.4 `import bw-json`](#104-import-bw-json)
  - [10.5 `import env`](#105-import-env)
  - [10.6 `import files`](#106-import-files)
- [11. `sp export` —— 批量导出](#11-sp-export--批量导出)
- [12. 交互式提示点汇总](#12-交互式提示点汇总)
- [13. 退出码模型](#13-退出码模型)
- [14. 已知边界与安全提示](#14-已知边界与安全提示)
- [附录 A：全命令全参数覆盖核对表](#附录-a全命令全参数覆盖核对表)
- [附录 B：相关文档](#附录-b相关文档)

---

## 1. 快速开始

### 先理解三个概念

| 概念 | 说明 |
|---|---|
| **库（vault）** | 单个 `.kdbx` 文件（KDBX 4.1），全部条目、密码、附件都在其中，整库加密 |
| **主密码** | 解锁库的密码，**只在你输入时存在于内存**，任何文件里都不存 |
| **keyfile** | 第二因子文件（`sp init` 自动生成 `库名.keyfile`）。**双因子缺一不可；keyfile 丢失 = 库永久不可解**，必须离线备份 |

> ⚠️ **同名程序撞车提示（sp.exe）**：本 CLI 的可执行文件名为 `sp.exe`，其他第三方程序也可能使用同一名称（已知案例：**win-pyvideotrans v4.17 项目的 GUI 应用**）。PATH 按目录顺序"先到先得"，敲 `sp` 时实际命中哪个取决于 PATH 顺序。调用前建议核对：
>
> ```powershell
> where.exe sp          # cmd / PowerShell 均可，列出 PATH 中全部同名命中（按优先级排序）
> Get-Command sp        # PowerShell：显示当前实际命中的完整路径
> sp --version          # 本 CLI 会输出版本号；子命令面为 item/clip/run/import/export 等
> ```
>
> 若命中了错误的程序：把本 CLI 所在目录在 PATH 中**移到该第三方条目之前**（推荐用 `scripts/add-path.ps1` 调整，避免手改注册表），或始终以完整路径调用（如 `C:\path\to\sp.exe item list`）。桌面 GUI（sp-desktop.exe）无此撞名问题。

### 三步上手

```bash
# ① 初始化新库（会交互式要求设定主密码两次，并自动生成 keyfile）
sp init --db C:\vault\main.kdbx

# ② 创建第一条条目（解锁后交互式输入条目密码）
sp item create github -u alice --url https://github.com

# ③ 取用：复制密码到剪贴板（30 秒后自动清除）
sp clip github/Password
```

- **用途**：完成"建库 → 存一条 → 取用"的最小闭环。
- **注意事项**：`sp init` 必须显式给 `--db`（见 §3 注意事项）；初始化成功后路径会写入配置文件，后续命令可省略 `--db`；keyfile 生成后**立即备份到独立介质**。
- **关联命令**：`sp item list` 核对 → `sp export` 建立备份 → 浏览器扩展填充（条目含 `--url` 时）。

### 命令行约定

- 所有写入型命令执行前都会**先解锁库**（除 `init`）：交互式提示 `Enter password to unlock <库路径>: `。
- 交互提示与进度信息写入 **stderr**；stdout 是纯净数据通道（`sp read` 管道取值、附件原始字节输出均不受提示文本污染）。
- 管道/非交互环境下，密码从标准输入读一行（脚本化用法见 §12，注意安全提示）。
- 所有命令支持 `-h/--help`；`sp help <命令>` 与 `sp <命令> --help` 等价。

---

## 2. 全局参数与配置文件

### 全局参数（可放在任意位置）

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `--db <路径>` | — | 否* | 配置文件 `default_db` | 指定库文件（覆盖配置默认值） |
| `--keyfile <路径>` | — | 否* | 配置文件 `keyfile` | 指定 keyfile（第二因子） |
| `--help` | `-h` | — | — | 打印帮助 |
| `--version` | `-V` | — | — | 打印版本 |

\* 未给且配置文件也没有 `default_db` 时报错：`no database configured; run sp init first or pass --db <path>`（退出码 1）。keyfile 缺失时按其库是不是双因子的：本工具 `sp init` 建的库一律需要 keyfile。

全局参数放在子命令前后均可，等价：

```bash
sp --db C:\vault\main.kdbx item list
sp item list --db C:\vault\main.kdbx
```

- **用途**：多库切换（工作库/个人库）或临时指向备份库。
- **注意事项**：`--db` 每次都要给时，建议直接写进配置文件（下文）一劳永逸。
- **关联命令**：`sp init`（初始化后会回写配置文件）。

### 配置文件 config.toml

位置：`%USERPROFILE%\.config\sp\config.toml`（Windows 一般为 `C:\Users\<你>\.config\sp\config.toml`）。

| 键 | 类型 | 说明 |
|---|---|---|
| `default_db` | 字符串（路径） | 默认库路径；`sp init` 成功后自动写入 |
| `keyfile` | 字符串（路径） | 默认 keyfile；`sp init` 成功后自动写入 |
| `clip_timeout_secs` | 整数 | `sp clip` 默认超时秒数，默认 `30` |
| `baseline_sha256` | 字符串 | 上次成功打开后库的 SHA-256 基线（完整性检测用，自动维护，无需手工编辑） |

示例：

```toml
default_db = 'C:\vault\main.kdbx'
keyfile = 'D:\usb\main.keyfile'
clip_timeout_secs = 30
```

- **用途**：把常用库设为默认，后续所有命令免 `--db/--keyfile`。
- **注意事项**：手改后立即生效（无缓存）；**keyfile 建议放独立介质**（上例放 U 盘），不要与库文件同目录同备份。
- **关联命令**：`sp init`（自动写）、`sp clip`（读取 `clip_timeout_secs`）。

---

## 3. `sp init` —— 初始化新库

**用法**：`sp init [OPTIONS]`

**用途**：创建一个新的 KDBX4 库：设定主密码（交互输入两次）→ 生成 keyfile（若未指定则生成在 `<库路径同名>.keyfile`）→ 按目标耗时校准 Argon2id 参数 → 写库 → 把库路径/keyfile 路径写入配置文件。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `--db <DB>` | — | 否（**实际必填**，见注意事项） | 配置文件默认值 | 新库输出路径（覆盖配置默认值） |
| `--kdf-target-ms <毫秒>` | — | 否 | `700` | KDF 校准目标：Argon2id 派生耗时校准到约该毫秒数 |
| `--keyfile <KEYFILE>` | — | 否 | 自动生成 `<库>.keyfile` | 指定 keyfile 路径（不存在则生成） |
| `--help` | `-h` | — | — | 打印帮助 |

**示例 1：建默认库**

```bash
sp init --db C:\vault\main.kdbx
```

- **用途**：一步完成建库 + keyfile 生成 + 配置回写，是全部后续命令的起点。
- **注意事项**：① 会交互要求 `Choose a master password:` 与 `Confirm master password:` 两次输入；② 自动生成 `C:\vault\main.keyfile` 后打印 `REMINDER: back up the keyfile to a separate medium`——**请立即离线备份 keyfile**；③ 校准过程打印 `KDF: Argon2id m=… t=… p=…`（你的库参数，可记录备查）；④ **目标库已存在时报错**（退出码 4）；⑤ **`init` 不会使用配置文件的 `default_db`**（行为与帮助文案略有差异）——未给 `--db` 时会报 `no database configured`（退出码 1），务必带上 `--db`。
- **关联命令**：`sp item create`（建条目）、`sp import`（批量导入）、`sp export`（建备份基线）。

**示例 2：定制 KDF 与 keyfile 位置**

```bash
sp init --db D:\keys\work.kdbx --keyfile E:\separate-medium\work.keyfile --kdf-target-ms 1000
```

- **用途**：keyfile 直接落到独立介质（推荐做法）；把解锁耗时调高（更强抗暴破，代价是每次解锁多等）。
- **注意事项**：`--kdf-target-ms` 越大越安全也越慢；目标机性能差异会导致实际耗时浮动 ±30% 属正常（校准内建复核）。
- **关联命令**：改参不需要重建库——KDF 参数随库头存储，`init` 时既定；如需调整请新建库并 `sp export`/`sp import` 迁移。

---

## 4. `sp item` —— 条目操作族

**用法**：`sp item <create | get | list | edit | delete>`

**用途**：条目（一个登录/一条密钥记录）的创建、查看、列举、**修改**、删除。子命令见下。

---

### 4.1 `sp item create` —— 创建条目

**用法**：`sp item create [OPTIONS] <TITLE>`

**用途**：新建一条条目，可带用户名、站点地址、备注、自定义字段、TOTP 与附件；浏览器扩展的自动填充按条目的 URL 匹配。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<TITLE>` | — | ✅ | — | 条目标题（位置参数；也是后续 get/clip/delete 的引用键） |
| `--username <USERNAME>` | `-u` | 否 | 无 | 用户名 |
| `--url <URL>` | — | 否 | 无 | 站点地址（**浏览器填充按此匹配**，建议写完整网址） |
| `--notes <NOTES>` | — | 否 | 无 | 备注（自由文本） |
| `--field <FIELD>` | — | 否 | 无 | 自定义字段 `键=值`，**可重复**多次；9 个保留名禁止：`Password/Title/URL/UserName/Notes/otp/BinaryData/Additional/BinaryDesc`（用了报参数错误，退出码 4） |
| `--field-secret <FIELD>` | — | 否 | 无 | **受保护**自定义字段 `键=值`，可重复（S1d-1）：值以字段级 ChaCha20 二次加密存储（与 Password/TOTP 同机制），KeePassXC 显示掩码互认；保留名/空名同禁 |
| `--totp <TOTP>` | — | 否 | 无 | TOTP 链接，必须是 `otpauth://` 开头（否则退出码 4） |
| `--tag <TAG>` | — | 否 | 无 | 标签分类，**可重复**多次（如 `--tag work --tag infra`）；存入 KDBX4 原生 Tags 字段，KeePassXC 互认；自动 trim，空标签被剔除（SP-F-108） |
| `--attach <ATTACH>` | — | 否 | 无 | 附加一个文件（任意类型二进制；**仅取文件名**，同名会覆盖；已有条目补加/读回见 §5） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例 1：最小可用条目**

```bash
sp item create github -u alice --url https://github.com
```

- **用途**：建一条可用于浏览器填充的 github 登录条目。
- **注意事项**：① 命令先提示 `Enter password to unlock <库>: `（主密码，写操作需解锁）；② 随后提示 `Entry password (empty to skip): `——**条目密码只能交互输入**（没有 `-p` 参数），留空则该条目不存密码；③ **同名标题不会报错**：会生成重复条目，查询/删除/修改用 `--index` 指定实例（§4.5 编辑、§4.2/§4.4；寻址语义详见 §4.5 开头）；④ **参数校验在解锁之前**：标题空/`--totp` 非 otpauth:// 开头/自定义字段保留名等坏参数**不提示主密码**直接报参数错误退出 4（2026-10-31 起）。
- **关联命令**：`sp item list`（查重）→ `sp item create` → `sp item get github -a Password` / `sp clip github/Password`（取用）→ `sp item edit`（改字段）→ `sp item delete github`（删除）。

**示例 2：全字段条目（自定义字段 + TOTP）**

```bash
sp item create work-vpn -u alice --url https://vpn.corp.example \
  --notes "办公 VPN，仅内网" \
  --field env=prod --field tier=gold \
  --totp "otpauth://totp/VPN:alice?secret=<BASE32占位符>&issuer=VPN"
```

- **用途**：一条里同时保存登录信息、自定义标签（env/tier）和 TOTP（页面有一站式验证框时浏览器可一并填充）。
- **注意事项**：① `--totp` 的值必须是完整的 `otpauth://` 链接（从二维码/密码管理器导出获取）；示例中 `<BASE32占位符>` 请替换为你的真实值（**真实密钥不要粘贴到聊天/文档/仓库**）；② `--field` 的值**与用户名/URL 同为"库内明文级"保护**（整库加密之内，无字段级加密）——不宜放高度敏感材料，敏感文件请用附件。
- **关联命令**：`sp item get work-vpn -t`（出 6 位动态码）。

**示例 3：附加文件（如 SSH 公钥、证书）**

```bash
sp item create server-key --notes "prod jump host" --attach C:\keys\id_ed25519
```

- **用途**：把重要文件作为附件随库加密保存。
- **注意事项**：① 附件名**只取文件名**（`id_ed25519`），同名附件互相覆盖；② **已有条目**的附件补加/列举/取回用 `sp attachment` 命令族（§5）。
- **关联命令**：`sp attachment get`（读回）、`sp export`（附件随归档导出）。

---

### 4.2 `sp item get` —— 查看条目

**用法**：`sp item get [OPTIONS] <TITLE>`

**用途**：查看条目字段；`-a` 单字段、`-t` 输出当前 TOTP 动态码、都不给则打印全部字段（含自定义字段；**不含附件**）。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<TITLE>` | — | ✅ | — | 条目标题 |
| `--field <FIELD>` | `-a` | 否 | 无 | 只显示单字段；**解析顺序：先查自定义字段，再查保留名**（`Title/UserName/Password/URL/Notes`，大小写敏感） |
| `--totp` | `-t` | 否 | — | 输出当前 TOTP 六位码；**优先级高于 `-a`** |
| `--index <N>` | — | 否 | 1 | **同名条目实例序号**（1-based，按库内文档序；DR-016）——同名时区分取哪一条，越界报未找到（退出 3，附同名总数） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例 1：取密码**

```bash
sp item get github -a Password
```

- **用途**：终端里查看密码明文（适合核对/一次性使用）。
- **注意事项**：密码会**打印到终端回显与滚动历史**；日常取用建议改用 `sp clip`（超时清除）或 `sp read --out-file`；只读命令同样需要解锁主密码。
- **关联命令**：`sp clip github/Password`（更安全的取用）→ `sp read`（脚本化取单值）。

**示例 2：一眼看全 + 动态码**

```bash
sp item get github
sp item get work-vpn -t
```

- **用途**：`get` 无参时逐行列出 Title/UserName/Password/URL/Tags（条目带标签时）/Notes 与全部自定义字段；`-t` 直接拿 TOTP 码去登录。
- **注意事项**：无参输出把**明文密码整行打印**（脚本请勿 `grep` 后再 `echo` 进日志）；**受保护自定义字段（--field-secret）在全量输出中显示掩码 `●●●●●●●●`**，需值时用 `-a <名称>` 显式取值；`-t` 输出每 30 秒变化的 6 位码，请勿保存/转发。
- **关联命令**：`sp read github/totp`（脚本里取码的等价写法）。

---

### 4.3 `sp item list` —— 列出标题

**用法**：`sp item list [OPTIONS]`

**用途**：打印库内全部条目**标题**（排序后），是查重与全库概览的最快途径。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `--tag <TAG>` | — | 否 | 无 | 只列携带该标签的条目（精确匹配、大小写不敏感；SP-F-108） |
| `--enumerate` | — | 否 | — | 每行加**同名组内序号前缀** `#n<TAB>`（1-based 文档序，与 `--index` 同口径）；默认纯标题排序输出（脚本兼容） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例**：

```bash
sp item list
sp item list --tag work
sp item list --enumerate
```

- **用途**：建条目前查重；确认导入结果；确认插件/桌面用库的条目清单；按标签筛选同类条目。
- **注意事项**：只列**标题**，不显示用户名等字段；库需要解锁（会提示主密码）；`--enumerate` 输出**文档序**（非排序），序号即 `item get/edit/delete --index` 用的实例号。
- **关联命令**：`sp item get <标题>`（看某条详情）→ `sp item edit`（改字段）→ `sp item delete <标题>`（清理误建条目）。

---

### 4.4 `sp item delete` —— 删除条目

**用法**：`sp item delete [OPTIONS] <TITLE>`

**用途**：删除条目——**回收站语义**：条目移入库的回收站对象（KeePassXC 中可见并恢复），并非彻底抹除。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<TITLE>` | — | ✅ | — | 要删除的条目标题 |
| `--index <N>` | — | 否 | 1 | **同名条目实例序号**（1-based 文档序）——同名时指定删哪一条；删除后其余同名的序号**前移** |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例**：

```bash
sp item list
sp item delete old-test
sp item list
```

- **用途**：删前 `list` 确认标题、删后再 `list` 复核。
- **注意事项**：① CLI **没有恢复命令**——误删恢复请用 KeePassXC 打开同一库，从回收站恢复；② 同名条目用 `sp item list --enumerate` 看序号后以 `--index` 指定（默认只删第一个命中）。
- **关联命令**：`sp item list`（前后核对）。

---

### 4.5 `sp item edit` —— 修改条目（2026-10-31 新增）

**用法**：`sp item edit [OPTIONS] <TITLE>`

**用途**：就地修改既有条目的字段（**patch 语义**：只给要改的，未给的字段一律保持不变）。旧版本自动压入条目 history（SP-F-103，KeePassXC 可见可回溯）。

**同名寻址口径（DR-016，2026-10-31 起）**：同名条目**允许存在**（创建/导入不查重）；`item get/edit/delete` 与 `read/clip` 均支持 `--index <N>`——N 为该标题下实例的**组内序号**（1-based，按库内文档序），`sp item list --enumerate` 可查看每个标题的序号。**序号在删除同名条目后会前移**，脚本引用请以"删前重查"为惯例。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<TITLE>` | — | ✅ | — | 要修改的条目标题（定位键；改名用 `--rename`） |
| `--index <N>` | — | 否 | 1 | 同名条目实例序号（1-based 文档序；越界退出 3 并附同名总数） |
| `--username <VAL>` | `-u` | 否 | 不变 | 新用户名；**传空串 `""` 清除**该字段 |
| `--url <VAL>` | — | 否 | 不变 | 新 URL；空串清除 |
| `--notes <VAL>` | — | 否 | 不变 | 新备注；空串清除 |
| `--password` | `-p` | 否 | 不变 | **交互式**提示输入新密码（`New entry password (empty to keep current): `）；输入空行=保持现密码 |
| `--totp <VAL>` | — | 否 | 不变 | 新 TOTP 链接（须 `otpauth://` 开头）；空串清除；非 otpauth 值退出 4 |
| `--tag <TAG>` | — | 否 | 不变 | **整体替换**标签集（可重复；自动 trim、空标签剔除；全空串=清空全部标签） |
| `--field <K=V>` | — | 否 | 不变 | 自定义字段 upsert（可重复）：同名键覆盖值，新键追加；保留名/空键名退出 4 |
| `--field-secret <K=V>` | — | 否 | 不变 | **受保护**自定义字段 upsert（可重复，S1d-1）：同名键覆盖 `--field` 的值与保护位；既有受保护字段经 `--field` 改值时保护位不变 |
| `--remove-field <K>` | — | 否 | 不变 | 按名删除自定义字段（可重复；名字不存在无副作用） |
| `--rename <NEW>` | — | 否 | 不变 | 改标题（可与其它字段同批）；空串退出 4；改名后允许与其它条目同名（DR-016） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例 1：改密码与 URL**

```bash
sp item edit github --url https://github.com/login
sp item edit github -p
# New entry password (empty to keep current): ********
```

- **用途**：换站点入口、轮换密码。
- **注意事项**：① 全部校验前置——`--index 0`/`--rename ""`/`--totp` 非 otpauth/`--field` 保留名/**一个修改参数都不给**（`nothing to edit`）都在**提示主密码之前**报参数错误退出 4；② 摘要输出只列**改了哪些字段类别**（如 `Updated entry 'github': password, url`），**不回显任何值**。
- **关联命令**：`sp item get github`（复核改动）。

**示例 2：自定义字段增删与标签替换**

```bash
sp item edit work-vpn --field tier=platinum --remove-field env --tag work --tag infra
```

- **用途**：维护条目的结构化元数据。
- **注意事项**：`--tag` 是**整体替换**（想追加请先把现有标签一并写全，可用 `sp item get work-vpn` 先看现状）；`--field env=` 会把 env 设为空值而不是删除，删除请用 `--remove-field env`。
- **关联命令**：`sp item get work-vpn`（看现值）。

**示例 3：同名条目的定向修改**

```bash
sp item list --enumerate
# #1	svc
# #2	svc
sp item edit svc --index 2 --rename svc-work
```

- **用途**：把同名条目中的一条改名以便后续区分（改名后两条即可用不同标题直接引用）。
- **注意事项**：`--index` 缺省=1（文档序第一条）；改名在合并后生效，**定位仍按原标题**。

---

## 5. `sp attachment` —— 附件操作（add / list / get）

**用法**：`sp attachment <add | list | get>`

**用途**：给已有条目**添加/替换附件**（SSH 私钥、证书、文档等任意二进制文件，随库加密），**列出**条目附件名，并把附件**取回**为文件或原始字节流。`sp item create --attach` 只能"建条目时带一个"；本命令族补齐"已有条目增补与读回"。

---

### 5.1 `sp attachment add` —— 添加/替换附件

**用法**：`sp attachment add [OPTIONS] <TITLE> <FILE>`

**用途**：把文件附加到已有条目；同名附件直接替换（无确认提示）。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<TITLE>` | — | ✅ | — | 目标条目标题（不存在报 notfound，退出码 3） |
| `<FILE>` | — | ✅ | — | 要附加的文件（任意二进制；只读内容，不保留源路径） |
| `--name <NAME>` | — | 否 | 源文件名 | 附件名（**同名覆盖**） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例**：

```bash
sp attachment add server-key C:\keys\id_ed25519
sp attachment add server-key C:\keys\id_ed25519 --name id_ed25519.prod
```

- **用途**：把 SSH 私钥、证书等挂到指定条目，随库加密；用 `--name` 区分不同环境的同名文件。
- **注意事项**：① **同名附件直接覆盖**（HashMap 键语义），替换敏感附件前注意；② 命令完成后会滚动备份库（`.sp-backup/` 保留 10 份，自动）；③ 大文件增加每次保存的耗时与内存占用（附件与库均为全量读写内存）。
- **关联命令**：`sp attachment list`（核验）→ `sp attachment get`（取回）→ `sp export`（随归档备份）。

### 5.2 `sp attachment list` —— 列出附件

**用法**：`sp attachment list [OPTIONS] <TITLE>`

**用途**：逐行打印条目全部附件名（排序后；无附件时无输出）。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<TITLE>` | — | ✅ | — | 条目标题（不存在报 notfound，退出码 3） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例**：

```bash
sp attachment list server-key
```

- **用途**：确认条目挂了哪些附件、附件名是什么（取回时要用）。
- **注意事项**：只输出**名字**，不输出大小/时间（KDBX 不存附加元数据）。
- **关联命令**：`sp attachment get <标题> <名字>`。

### 5.3 `sp attachment get` —— 取回附件

**用法**：`sp attachment get [OPTIONS] <TITLE> <NAME>`

**用途**：把附件写回文件（`--out`，ACL 限当前用户）或输出**原始字节**到 stdout（不给 `--out` 时）——stdout 模式可安全管道，例如 `sp attachment get t id_ed25519 > key` 或 `| ssh-add -`。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<TITLE>` | — | ✅ | — | 条目标题 |
| `<NAME>` | — | ✅ | — | 附件名（见 `list`；不存在报 notfound，退出码 3） |
| `--out <OUT>` | — | 否 | 无（stdout 原始字节） | 写入文件（ACL 限当前用户）；不给则原始字节写 stdout |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例 1：取回为文件**

```bash
sp attachment get server-key id_ed25519 --out C:\tmp\id_ed25519
```

- **用途**：把附件落盘到指定位置（权限限当前用户）。
- **注意事项**：① 落盘的是**明文附件**（如 SSH 私钥）——用后请立即删除或转移；② 同名目标文件会被**覆盖**；③ 进度/提示信息走 stderr，可 `--out` 与提示共存。
- **关联命令**：`ssh-add C:\tmp\id_ed25519`（使用后即删）。

**示例 2：直接管道（不落盘）**

```bash
sp attachment get server-key id_ed25519 | ssh-add -
```

- **用途**：私钥不落盘直接喂给 `ssh-add` 等消费者（bash/管道友好）。
- **注意事项**：stdout 输出是**原始字节**（无换行、无编码转换）——**PowerShell 的重定向（`>`）会按文本处理二进制**，PowerShell 用户请改用 `--out`；Git Bash/cmd/管道场景可直接使用。
- **关联命令**：`sp attachment get --out`（PowerShell 场景）。

---

## 6. `sp read` —— 按 URI 读单值

**用法**：`sp read [OPTIONS] <URI>`

**用途**：按 `<标题>/<字段>` 读取单个值并输出到 stdout——脚本化取值的标准入口（相对 `item get` 的输出更适合管道）。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<URI>` | — | ✅ | — | 形如 `github/Password`；字段解析顺序：**先保留名**（`Title/UserName/Password/URL/Notes`，大小写敏感），再自定义字段；`totp`/`TOTP` 特殊（直接出当前动态码） |
| `--no-newline` | `-n` | 否 | — | 输出不带结尾换行（便于拼进命令替换） |
| `--out-file <FILE>` | — | 否 | — | 写入文件（**Windows ACL 限当前用户**；不再输出到终端） |
| `--index <N>` | — | 否 | 1 | 同名条目实例序号（1-based 文档序；DR-016） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例 1：管道取值**

```bash
TOKEN=$(sp read api-service/Password -n)
curl -H "Authorization: Bearer $TOKEN" https://api.example.com/ping
```

- **用途**：把库里的密钥喂给命令行工具，全程不落盘。
- **注意事项**：① 解锁提示写 stderr，命令替换只捕获值本身（stdout 纯净）；② 值进入 shell 变量（内存）与进程参数时，在有审计/快照的环境仍可能被观察到——比明文落盘安全，但不是零暴露；③ 注意 `-n` 去掉换行，否则 `Bearer` 头会多带 `\r`；④ 字段名大小写敏感（`password` 与 `Password` 是不同字段）。
- **关联命令**：`sp run`（更安全的注入方式）→ `sp inject`（模板文件渲染）。

**示例 2：写进受限文件**

```bash
sp read api-service/Password --out-file C:\tmp\token.txt
```

- **用途**：给只认文件路径的工具使用；文件权限限当前用户。
- **注意事项**：这是**明文落盘**（虽然是临时文件）——用后立即删除；优先考虑 `sp run`/`sp inject` 这类不落盘的方式。
- **关联命令**：`sp clip`（不落盘但要人操作）、`sp run`（推荐替代）。

---

## 7. `sp clip` —— 复制到剪贴板（超时清除）

**用法**：`sp clip [OPTIONS] <URI>`

**用途**：把单值复制到系统剪贴板，等待超时后自动清空。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<URI>` | — | ✅ | — | 同 `sp read` 的 URI 语法 |
| `--timeout <秒>` | — | 否 | 配置文件 `clip_timeout_secs`（默认 30） | 清除倒计时秒数 |
| `--index <N>` | — | 否 | 1 | 同名条目实例序号（1-based 文档序；DR-016） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例**：

```bash
sp clip github/Password
sp clip github/Password --timeout 10
```

- **用途**：粘贴到浏览器/远程桌面等不适合命令注入的场合。
- **注意事项**：① **命令会前台阻塞**直到超时清空（提示 `Copied to clipboard; clearing in Ns (Win+V history is NOT controlled by sp).`）——请不要关掉窗口，否则提前终止=不清除；② **Windows 剪贴板历史（Win+V）不受 sp 控制**，敏感值建议粘贴完立刻 `Win+V` 删历史或用短超时；③ 剪贴板被其它程序覆写属正常，sp 只在超时点清空一次。
- **关联命令**：`sp read --out-file`（不阻塞的替代）。

---

## 8. `sp run` —— 注入环境变量运行子进程

**用法**：`sp run [OPTIONS] -- <CMD>...`

**用途**：把库里的秘密作为环境变量（或 stdin 记录）注入，运行一条子命令；**秘密不落盘**，且子进程输出中的秘密值默认被打码（`<redacted>`）。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `--env-file <FILE>` | — | 否 | 无 | `.env` 格式文件，值写 `sp://标题/字段` 引用（见示例） |
| `--stdin` | — | 否 | 否 | 秘密改由**子进程 stdin 的 NUL 分隔记录**传入而非环境变量（避免 `/proc/<pid>/environ` 暴露；POSIX 场景，design_docs/04 §4.1） |
| `--no-masking` | — | 否 | 否 | **关闭输出打码（仅调试）**——默认开打码，关闭后子进程输出里的秘密会原样透出，慎用 |
| `<CMD>...` | — | ✅ | — | `--` 之后的命令与其参数 |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例 1：env 注入**

```bash
# 先写引用文件（值不落盘，只落"引用"）
# app.env:
#   GH_TOKEN=sp://github/Password
#   API_KEY=sp://api-service/Password
sp run --env-file app.env -- my-tool --flag
```

- **用途**：给工具喂多份密钥，一次解锁、零落盘。
- **注意事项**：① 子进程 stdout/stderr 里的秘密值会被打码为 `<redacted>`——**打码面 = env 文件全部值**（含明文非 `sp://` 引用的值，保守设计防漏保护；需查看真实值时用 `--no-masking`）；② **Windows 下环境变量可被同用户进程读取**——威胁模型内可接受，更高要求请关注 `--stdin`；③ 子进程**退出码原样透传**（脚本可直接判断成败）；④ 运行期间解锁提示交互一次；⑤ `sp://` 引用的**格式**校验在解锁之前（坏引用不提示主密码直接退 4），**条目存在性**在解锁之后（需查库，退 3）。
- **关联命令**：`sp inject`（需要文件/配置模板时）；`sp read`（单值场景）。

**示例 2：stdin 记录模式（POSIX）**

```bash
sp run --stdin --env-file app.env -- bash -c 'while IFS= read -r -d "" r; do echo "got record len: ${#r}"; done'
```

- **用途**：让子进程从 stdin 读 NUL 分隔记录，完全避开环境变量暴露面。
- **注意事项**：需要子进程配合读取；Windows 下该开关的帮助文案标注为 POSIX 场景，请以目标平台行为为准。
- **关联命令**：`sp run --env-file`（常规模式）。

---

## 9. `sp inject` —— 模板渲染

**用法**：`sp inject [OPTIONS] --input <INPUT>`

**用途**：把模板文件中的 `{{ sp://标题/字段 }}` 占位符替换为库中真实值，输出到文件或 stdout——用于生成 `.env`、kubeconfig、部署配置等。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `--input <INPUT>` | `-i` | ✅ | — | 模板文件路径（内含 `{{ sp://title/field }}` 占位符） |
| `--output <OUTPUT>` | `-o` | 否* | — | 输出文件路径（**含明文秘密**；Windows ACL 限当前用户） |
| `--stdout` | — | 否* | — | 渲染结果输出到 stdout（两者同时给出时 **stdout 优先**） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

\* `-o` 与 `--stdout` **必须给出其一**，都未给报参数错误（退出码 4）。

**示例 1：生成配置文件**

```bash
# config.tpl:  token = "{{ sp://api-service/Password }}"
sp inject -i config.tpl -o config.prod.json
```

- **用途**：从模板一键生成带真实密钥的配置。
- **注意事项**：① 输出文件**是明文秘密**——提示 `contains SECRETS in cleartext; delete after use.`，用后即删；② 占位符格式必须精确（`{{ sp://标题/字段 }}`，字段规则同 `sp read`）。
- **关联命令**：`sp run`（不需要文件的场景更安全）。

**示例 2：管道直出**

```bash
sp inject -i config.tpl --stdout | some-consumer
```

- **用途**：不落盘，直接喂给下游程序。
- **注意事项**：下游若会把内容写日志/落盘，等同于明文落盘。
- **关联命令**：`sp run --env-file`（等价思路的另一种形态）。

---

## 10. `sp import` —— 批量导入

### 10.1 导入总览与去重报告

**用法**：`sp import [OPTIONS] <dotenv | csv | bw-json | env | files>`

**用途**：从五种来源扫描"疑似秘密值"，去重后批量入库为新条目。

**参数表（所有子命令共有）**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `<子命令> <PATH>` | — | ✅（`env` 无 PATH） | — | 来源与输入路径（见各小节） |
| `--dry-run` | — | 否 | 否 | **只扫描并打印报告，不开库、不写入** |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

每次运行先打印去重报告：

```
Scan: <发现数> findings -> <候选数> unique candidates (L1 dup x / L2 merged y / L3 conflicts z)
REVIEW [L3 conflict] ...   ← 需人工复查的行
```

**⚠️ 当前版本导入条目的形态（三个来源一致，务必知晓）**：

- 导入条目**只携带"值"**（密码/TOTP），**标题、用户名、URL 均不来自源文件**；
- 条目标题是生成式：`[来源/来源目录/规则] 文件名#序号`；备注记录 `source: 文件:行号 rule: 规则`；另有自定义字段 `fingerprint`（值哈希前 8 字节）；
- 因此导入后**需要人工补全用户名/URL**（当前 CLI 无 `sp item edit`，可用 KeePassXC 打开同一库编辑，或后续版本提供）；
- 重复运行导入会再次入库（不与库内查重）——先 `--dry-run` 看报告，再正式导。

- **用途**：把散落在 `.env`/CSV/Bitwarden 导出/环境变量/代码目录中的密钥，先集中收进保险库。
- **注意事项**：先 `--dry-run` 永远是第一步；导入是对**已解锁库的写操作**，会触发自动备份轮换（保留 10 份）。
- **关联命令**：`--dry-run` → 正式导入 → `sp item list` 核对 → `sp export` 备份。

### 10.2 `import dotenv`

**用法**：`sp import [--dry-run] dotenv <PATH>`

**用途**：解析 `.env` 语法文件；**PATH 给目录时递归扫描目录内文件名以 `.env` 开头的文件**。

**参数表**：`<PATH>`（✅，文件或目录）；其余同上表。

```bash
sp import dotenv .\secrets.env --dry-run
sp import dotenv D:\projects\app1
```

- **用途**：收拢手头的 env 密钥文件。
- **注意事项**：目录模式只收 `.env*` 命名的文件；单文件模式不做命名限制。
- **关联命令**：`sp run --env-file`（值在库中后，反向用于运行时注入）。

### 10.3 `import csv`

**用法**：`sp import [--dry-run] csv <PATH>`

**用途**：解析通用 CSV；列名按别名表识别。

**参数表**：`<PATH>`（✅，CSV 文件）；其余同上表。

**列名别名**：`title|name|account`、`username|user|login|login_username`、`password|passwd|pwd`、`url|uri|website|login_uri`、`notes|note|comment`、`totp|otp`。

```bash
sp import csv C:\export\keeppassxc.csv --dry-run
sp import csv C:\export\keeppassxc.csv
```

- **用途**：从其它密码管理器导出的 CSV 里回收密码/TOTP 值。
- **注意事项**：**即使 CSV 带 title/username/url 列，当前版本也不写入条目**（见 §10.1 形态说明，L2 去重暂不启用）；文件编码建议 UTF-8。
- **关联命令**：导入后按 §10.1 说明用 KeePassXC 补全字段。

### 10.4 `import bw-json`

**用法**：`sp import [--dry-run] bw-json <PATH>`

**用途**：解析 Bitwarden JSON 导出（核心字段）。

**参数表**：`<PATH>`（✅，JSON 文件）；其余同上表。

```bash
sp import bw-json C:\export\bitwarden_export.json --dry-run
```

- **用途**：从 Bitwarden 迁移值。
- **注意事项**：同 §10.1 形态说明（用户名/URL 不随条目写入）；Bitwarden 导出文件本身是明文，导入完成并核对后请**安全销毁**源导出文件。
- **关联命令**：`sp export`（迁移完成后做一次完整加密备份）。

### 10.5 `import env`

**用法**：`sp import [--dry-run] env`

**用途**：扫描**当前进程环境变量**（内存内，不落盘）。

**参数表**：无位置参数；其余同上表。

```bash
sp import env --dry-run
```

- **用途**：清理当前 shell/会话里大量 `*_TOKEN/*_KEY` 环境变量。
- **注意事项**：只扫描当前进程可见的环境变量；建议在**专门开的干净窗口**里跑，通过 `set`/`$env:` 临时注入你要收拢的几个值——避免把系统级无关变量扫进来。
- **关联命令**：`sp run`（收拢后再注入给子进程）。

### 10.6 `import files`

**用法**：`sp import [--dry-run] files <PATH>`

**用途**：递归扫描目录，用内置 gitleaks 规则找疑似密钥；**自动跳过 >2MB 或非 UTF-8 文件**。

**参数表**：`<PATH>`（✅，目录或文件）；其余同上表。

```bash
sp import files D:\projects --dry-run
sp import files D:\projects
```

- **用途**：项目大扫除——找散落的 `id_rsa` 注释、硬编码 token 等。
- **注意事项**：这是"疑似"扫描（规则命中即报），正式导入前请把 `REVIEW` 行与报告核对一遍；跳过的大文件/二进制需另行人工检查。
- **关联命令**：`import csv/bw-json`（有结构的来源）。

---

## 11. `sp export` —— 批量导出

**用法**：`sp export [OPTIONS]`

**用途**：导出全库。**默认加密归档**（zip + age 口令加密，官方 age 工具可解）；`--plaintext` 输出明文 JSON（三重防护）。

**参数表**：

| 参数 | 简写 | 必填 | 默认 | 说明 |
|---|---|---|---|---|
| `--out <OUT>` | — | 否（加密归档模式**实际必填**） | — | 加密归档输出路径（age + zip）；未给且非明文模式时报参数错误（退出码 4） |
| `--plaintext` | — | 否 | 否 | 输出明文 JSON 到 stdout（**需要重输主密码确认**） |
| `--stdout` | — | 否 | 否 | 仅影响是否打印一句确认提示（明文内容恒走 stdout） |
| `--db / --keyfile / -h` | — | — | — | 见 §2 |

**示例 1：加密归档（推荐的备份方式）**

```bash
sp export --out C:\backup\vault-2026-10-29.zip.age
```

- **用途**：产生独立于库文件的全量备份（条目全字段 + TOTP + 自定义字段 + **附件**，含 `entries.json` 与 `attachments/` 目录）。
- **注意事项**：① 交互两次输入**归档口令**（`Archive passphrase:` / `Confirm passphrase:`）——**该口令与主密码独立**，请一并妥善保管；② 归档用 age 加密（XChaCha20-Poly1305 + scrypt），官方 `age` 工具可独立解密校验；③ 摘要行明示**不含 passkey/回收站对象**。
- **关联命令**：定期执行形成异机备份链；`sp init`（灾备恢复的起点，见示例 2 注意事项）。

**示例 2：明文 JSON（危险操作）**

```bash
sp export --plaintext > C:\backup\vault-plain.json
```

- **用途**：给需要原始数据的迁移/脚本使用（含附件，附件在 JSON 中为字节数组）。
- **注意事项**：① **三重防护**：必须显式 `--plaintext` + 重输主密码（`Re-enter master password to confirm plaintext export:`）+ 警告文案；② 明文恒写 stdout，**不落临时文件**（由你决定重定向到哪——重定向即落盘）；③ **灾难恢复路径如实说明**：当前版本没有"从归档直接回导"的命令——恢复方式为：用 age 解密并解包归档 → `entries.json` 与 `attachments/` 可用脚本/KeePassXC 等导入或人工处理；或用明文 JSON 自行迁移。请在导出后**实测一次恢复流程**再依赖它。
- **关联命令**：`sp import`（值级回迁）、`sp init`（新建恢复目标库）。

---

## 12. 交互式提示点汇总

| 提示原文 | 出现命令 | 说明 |
|---|---|---|
| `Enter password to unlock <库>: ` | 除 `init` 外全部命令 | 主密码解锁；**管道/非 TTY 时从 stdin 读一行**；提示写 **stderr**，stdout 保持纯净 |
| `Choose a master password:` / `Confirm master password:` | `sp init` | 新库主密码两次输入 |
| `Entry password (empty to skip): ` | `sp item create` | 条目密码，留空则不存 |
| `New entry password (empty to keep current): ` | `sp item edit -p` | 新条目密码，留空=保持现密码 |
| `Archive passphrase:` / `Confirm passphrase: ` | `sp export`（默认模式） | 归档加密口令（与主密码独立） |
| `Re-enter master password to confirm plaintext export: ` | `sp export --plaintext` | 明文导出二次确认 |

**脚本化用法与安全提示**：非交互环境（管道/重定向）下，主密码从 stdin 读取一行，例如：

```bash
printf '%s\n' "$MASTER_PW" | sp item list
```

- **注意事项**：把主密码放进 shell 变量/环境再喂管道，等同把机密暴露给 shell 历史与进程环境——仅建议在受控的自动化环境使用，且**不要把密码写进脚本文件或命令行参数**。
- **关联命令**：无（全局机制）。

---

## 13. 退出码模型

| 退出码 | 含义 | 典型触发 |
|---|---|---|
| `0` | 成功 | — |
| `1` | 一般错误 | 库格式/IO/配置错误（如未配置默认库）、剪贴板不可用 |
| `2` | **凭据错误** | 主密码错、keyfile 错或缺失（**不细分为防侧信道**） |
| `3` | 未找到 | 条目/字段/URI/附件不存在 |
| `4` | 参数错误 | 业务校验失败（如 `--totp` 非 otpauth、`--field` 用了保留名、`init` 目标库已存在、`inject` 未给输出、`export` 未给 `--out`） |
| 子进程退出码 | `sp run` 专用 | 子进程的退出码**原样透传** |

- **注意事项**：命令行**用法层面**的错误（缺位置参数、`--field` 格式不对等）由参数解析器直接以**退出码 2** 退出——与"凭据错误"同码，脚本判错时请结合 stderr 文案区分。
- **关联命令**：`sp run`（透传规则见 §8）。

---

## 14. 已知边界与安全提示

**功能边界（当前版本如实声明）**：

1. **附件命令族完备度**：`sp attachment add/list/get` 已补齐"已有条目增补与读回"；仍**没有附件删除/重命名命令**（需要时用 KeePassXC 打开同一库操作）；同名附件直接覆盖。
2. **导入条目只含"值"**：标题/用户名/URL 不随导入写入（§10.1），需人工补全后可用 `sp item edit` 补齐（2026-10-31 起有编辑入口）。
3. **无归档一键回导**：`sp export` 的归档恢复需手动解包处理（§11 示例 2）。
4. **没有 `--force/--quiet/--verbose`**：SRS/早期设计提及但未实现；`item create -p` 参数不存在（帮助文案以 `sp --help` 为准）。
5. **字段保护级别**：密码、TOTP 与**受保护自定义字段**（`--field-secret`/GUI"保护"勾选，S1d-1）在库内还有一层字段级保护（ChaCha20 ProtectedValue）；**用户名、URL、备注、普通自定义字段只有整库加密保护**（解密后 XML 内明文）——高敏材料请放密码字段、受保护字段或附件。注意：`item get` 全量输出对受保护字段显示掩码，`-a <名称>` 显式取值仍输出明文；KeePassXC 的 `show -a` 点名也会输出明文（掩码仅在其默认/`--all` 列表生效）。
6. **同名条目**（DR-016，2026-10-31 起）：创建/导入**不查重**（允许同名）；查询/删除/修改用 `--index` 指定实例（`item get/edit/delete/read/clip`；`item list --enumerate` 看序号），缺省=首个命中；**附件命令族与 `sp run/inject` 的 sp:// 引用暂不支持 `--index`**（维持首个命中）；删除同名条目后其余实例序号前移。
7. **每次保存都会重加密整库**（随机盐/随机种子轮换），大附件会增加保存耗时与内存占用（附件与库均为全量读入内存）。

**安全提示**：

- **keyfile 必须离线备份**（与库文件分渠道存放）；丢失=库永久不可解。
- `sp clip` 无法控制 Windows 剪贴板历史（Win+V），用后请清理。
- 明文出处：`item get` 无参输出、`read`（终端/`--out-file`）、`attachment get`（文件/管道）、`inject -o`、`export --plaintext`——这些会产生明文，按需使用、用后清理。
- 本手册中的示例值均为**占位符**；请勿把真实密钥粘进文档、聊天或仓库。

---

## 15. .env 工作流与注入启动（SP-F-301；2026-10-31 起 GUI 同款能力）

`.env` + `sp run` 是"引用不落值"的注入工作流：`.env` 文件里写的不是机密本身，
而是 `sp://` 引用，机密只在子进程运行期存在。

**三步工作流**：

```bash
# ① 写引用文件（值不落盘，只落"引用"）
#    app.env:
#      GH_TOKEN=sp://github/Password
#      API_KEY=sp://api-service/Password

# ② 以注入方式启动
sp run --env-file app.env -- your-app --serve

# ③ 子进程输出中的机密默认被遮蔽（<redacted>，跨 chunk 100% 替换）
```

**要点**：

- `--env-file` 解析对齐 1Password op 规则：`$VAR`/`${VAR}` 引用展开、单引号字面量、多行值；`sp://` 引用格式在解锁前校验（坏引用退出 4，不提示主密码）。
- 受保护变量（`PATH`/`HOME`/`USER`/`TERM`/`SHELL`/`LC_*`/`XDG_*`）拒绝覆盖（SP-F-301）。
- 两种机密通道：默认注入**子进程环境变量**；`--stdin`（POSIX）改写 NUL 分隔记录到 stdin，规避 `/proc/<pid>/environ` 读取面。
- `sp inject`（SP-F-302）为模板渲染变体：`{{ sp://... }}` 写进任意模板，`-o` 落盘文件自动限权（含明文，用后即删）。
- **GUI 等价能力（2026-10-31 起）**：桌面 GUI"注入启动"对话框——选条目、勾选注入字段（UserName/Password/URL/TOTP/自定义字段）、填程序路径即以 `sp run` 同语义启动（argv 直 exec 无 shell；受保护变量同样拒绝；子进程输出不遮蔽，对齐 `--no-masking` 口径）。机密仅存在于子进程环境，不落盘。
- **GUI 导入/导出向导**同批落地：导入（.env/CSV/Bitwarden JSON/目录扫描/环境变量，两阶段扫描→确认，与 CLI `sp import` 同管道）、导出（age 加密归档，与 CLI `sp export` 同实现；明文导出仍仅限 CLI `--plaintext` 三重防护）。

---

## 附录 A：全命令全参数覆盖核对表

> 核对基准：`sp --help` 与全部子命令 `--help` 输出（2026-10-31，随 `sp item edit`/`--field-secret` 更新），逐项比对打勾。

| 命令 | 子命令 / 参数（全部） | 覆盖 |
|---|---|---|
| `sp` | `init` `item` `attachment` `read` `run` `inject` `clip` `import` `export` `help` | ☑ |
| 全局 | `--db` `--keyfile` `-h/--help` `-V/--version` | ☑ |
| `sp init` | `--db` `--kdf-target-ms`（默认 700） `--keyfile` `-h` | ☑ |
| `sp item` | `create` `get` `list` `edit` `delete` `help` | ☑ |
| `sp item create` | `<TITLE>` `-u/--username` `--url` `--notes` `--field`(重复) `--field-secret`(重复) `--totp` `--tag`(重复) `--attach` `--db` `--keyfile` `-h` | ☑ |
| `sp item get` | `<TITLE>` `-a/--field` `-t/--totp` `--index` `--db` `--keyfile` `-h` | ☑ |
| `sp item list` | `--tag` `--enumerate` `--db` `--keyfile` `-h` | ☑ |
| `sp item edit` | `<TITLE>` `--index` `-u/--username` `--url` `--notes` `-p/--password` `--totp` `--tag`(重复) `--field`(重复) `--field-secret`(重复) `--remove-field`(重复) `--rename` `--db` `--keyfile` `-h` | ☑ |
| `sp item delete` | `<TITLE>` `--index` `--db` `--keyfile` `-h` | ☑ |
| `sp attachment` | `add` `list` `get` `help` | ☑ |
| `sp attachment add` | `<TITLE>` `<FILE>` `--name` `--db` `--keyfile` `-h` | ☑ |
| `sp attachment list` | `<TITLE>` `--db` `--keyfile` `-h` | ☑ |
| `sp attachment get` | `<TITLE>` `<NAME>` `--out` `--db` `--keyfile` `-h` | ☑ |
| `sp read` | `<URI>` `-n/--no-newline` `--out-file` `--index` `--db` `--keyfile` `-h` | ☑ |
| `sp clip` | `<URI>` `--timeout` `--index` `--db` `--keyfile` `-h` | ☑ |
| `sp run` | `--env-file` `--no-masking` `--stdin` `-- <CMD>...`(必填) `--db` `--keyfile` `-h` | ☑ |
| `sp inject` | `-i/--input`(必填) `-o/--output` `--stdout` `--db` `--keyfile` `-h` | ☑ |
| `sp import` | `--dry-run` `dotenv <PATH>` `csv <PATH>` `bw-json <PATH>` `env` `files <PATH>` `--db` `--keyfile` `-h` | ☑ |
| `sp export` | `--out` `--plaintext` `--stdout` `--db` `--keyfile` `-h` | ☑ |
| 配置文件 | `default_db` `keyfile` `clip_timeout_secs` `baseline_sha256` | ☑ |
| 交互提示 | 解锁 / 主密码×2 / 条目密码 / 归档口令×2 / 明文重确认 | ☑ |
| 退出码 | 0 / 1 / 2 / 3 / 4 / 子进程透传 | ☑ |

---

## 附录 B：相关文档

| 文档 | 内容 |
|---|---|
| `README.md` | 项目总览与快速上手 |
| `docs/SRS.md` | 软件需求规格（需求编号与验收口径） |
| `design_docs/03_sp-cli_命令族与错误模型.md` | CLI 设计文档 |
| `design_docs/09_安全实现方案.md` | 加密与存储格式细节（KDBX4/Argon2id/keyfile） |
| `native-host/README.md` | 浏览器扩展 native messaging 注册与部署 |
| `desktop/`（design_docs/15） | 桌面壳（托盘 + 会话 + 内置 NM host） |

---

*手册维护约定：任何 CLI 行为变更必须同步更新本手册与附录 A；`sp --help` 是唯一事实来源。*
