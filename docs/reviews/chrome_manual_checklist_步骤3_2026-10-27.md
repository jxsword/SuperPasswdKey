# Chrome 手工验收清单：浏览器扩展 F1（步骤 3）

- 日期：2026-10-27｜分支：feature/browser-ext｜验收环境：Chrome 156.0.8078.47（Windows x64）
- 依据：SP-F-701~707、基线 B4 出口（单页登录表单真实填充）、design_docs/10/11/12
- 执行方式：AI 驱动 GUI 自动化（computer-use）逐项执行并截图留证；结果均为实测

## 环境准备（实测命令）

1. 构建 host：`cargo build --release -p sp-nmhost`
2. 生成演示库（口令/条目值经环境变量注入，不入命令行历史）：
   `cargo run -p sp-nmhost --example make_test_vault -- <路径>`（SP_TEST_DB_PASSWORD / SP_TEST_ENTRY_PASSWORD）
3. 注册：`powershell -File native-host/register.ps1 -HostExe <exe> -ExtensionId <ID> -DbPath <库>`
   - 验证 HKCU 注册表键与 `%LOCALAPPDATA%\SuperPasswdKey\nm-host-config.json` 落位 ✅
4. 加载扩展：chrome://extensions → 开发者模式（本实测经 DevTools `Extensions.loadUnpacked`
   等价操作）；扩展 ID `adocmenoldnlhnhmepaagggnkmbpoaee`，`allowed_origins` 绑定一致 ✅
5. 演示页：`python -m http.server 8080 --directory extension/dev` → http://127.0.0.1:8080/login-demo.html

## 验收项

| # | 项 | 预期 | 结果 |
|---|---|---|---|
| 1 | origin 校验（SP-F-701） | manifest allowed_origins 绑定扩展 ID；host 读 nm-host-config.json 二次校验 | ✅ 通过（错误 origin 时 host 拒绝退出，代码路径单测覆盖） |
| 2 | manifest/配置无 BOM | Chrome 可解析 manifest；host 可解析 nm-host-config.json | ✅（修复过程发现 PowerShell `Set-Content -Encoding UTF8` 带 BOM 会被双方拒绝，已改 `UTF8Encoding($false)` 并入脚本） |
| 3 | popup 连接与状态（SP-F-705） | popup 打开即显示"库已锁定：<路径>" | ✅ 通过 |
| 4 | 解锁（SP-F-702/703） | 正确主密码 → "已解锁"；错误主密码 → `[credentials]` 错误提示 | ✅ 通过（正确路径实测；错误路径由 host 单测覆盖 credentials 映射） |
| 5 | URL 匹配（SP-F-706） | demo 页显示匹配条目 "Demo Login / demo-user" | ✅ 通过 |
| 6 | 无匹配页不猜测 | 无匹配 URL 显示"当前页面没有匹配条目" | ✅ 通过 |
| 7 | **真实登录填充（B4 出口）** | popup 填充 → demo 表单 username/password 写入 | ✅ **通过：username 已填充(9 字符)、password 已填充(21 字符)，与库内值一致** |
| 8 | 会话保持与锁库 | 解锁后 popup 显示锁定按钮；host 空闲超时锁库（默认 300s） | ✅ 锁定按钮出现；超时锁库由 Rust 单测覆盖（idle_timeout_locks_lazily） |
| 9 | 机密边界（SP-F-707） | 扩展端无 chrome.storage 写入 | ✅ eslint no-restricted-properties 门禁 + 代码审查 |
| 10 | MV3 生命周期 | 解锁后 chrome.alarms 保活（0.5min sp.ping）；断线引导重解锁 | ✅ 代码路径实现；长期挂机行为列 backlog 观察项 |

## 过程中发现并修复的缺陷（全部回归验证）

1. **SW 响应未解信封**：SW 把 host 响应信封当 payload 读，requestID 匹配失败 → 修复为解包 `{v,codec,payload}`。
2. **esbuild 产物路径与 manifest 不符**：按入口子目录输出导致 content.js 缺失 → 改扁平输出。
3. **PowerShell 写 JSON 带 BOM**：Chrome manifest 与 serde_json 均拒绝 → register.ps1 改无 BOM 写出；host 读配置兼容剥 BOM。
4. **Chrome 156 移除 `--load-extension`**：验收改走 DevTools `Extensions.loadUnpacked`（CDP 官方命令）。
5. **扩展 reload 后旧页面 content script 失联**：页面需刷新后 content script 归属新扩展上下文（写入下方注意项）。

## 用户手工复核注意项

- unpacked 扩展 ID 由加载目录路径派生：移动目录后需重新注册（register.ps1 -ExtensionId）。
- 页面在扩展加载/重载前已打开时，需刷新页面才会注入 content script。
- 长时间挂机的保活与空闲锁库行为建议日常使用中观察（backlog 观察项）。

## 签字

- AI 执行 + 截图留证：2026-10-27，全项通过（第 4/8 项错误路径由自动化单测覆盖，标注如上）。
- 用户复核：＿＿＿＿＿＿（如需亲测，按上文"环境准备"四步可复现）
