# Chrome 手工验收清单：登录自动保存（SP-F-708，step8 / 2026-10-30）

> 前置：
> 1. `extension/dist` 已重新构建（`cd extension && pnpm build`——本机已构建，含保存流代码）。
> 2. `chrome://extensions` → SuperPasswdKey → **刷新**（加载新 dist）。
> 3. NM host 已注册 sp-desktop 模式（PR #34 之后的构建），sp-desktop 在运行。
> 4. 准备两个测试页：`extension/dev/login-demo.html`（本地起服，如
>    `python -m http.server 8081 --directory extension/dev`——**该页 onsubmit
>    preventDefault 不导航**）+ 任一真实导航登录页（或本地另一端口页）。

| # | 场景 | 步骤 | 预期 | 签字 |
|---|---|---|---|---|
| 1 | 新增保存（同页/防抖场景） | demo 页填新用户名+密码 → 点登录（不导航） | 页内右下浮层「将用户名 … 保存到 localhost:8081？」，**密码不回显** | ☑ |
| 2 | 确认新增 | 浮层点「保存」 | 状态变「已保存为条目「localhost:8081」」→ 浮层自灭；`sp item list` 出现 `localhost:8081`；`sp item get localhost:8081 -a UserName` = 刚填值 | ☑ |
| 3 | 更新保存（密码变更入历史） | demo 页同用户名换密码再提交 → 保存 | 浮层「已更新条目…（旧密码已存入历史）」；`sp item get localhost:8081 -a Password` = 新密码 | ☑ |
| 4 | 真实导航场景 | 真实登录页提交（发生跳转） | **新页面**加载后弹同款浮层（捕获自上一页提交时点） | ☑ |
| 5 | 忽略=本会话静默 | 浮层点「忽略」→ 同 host 再次提交登录 | 不再弹浮层（本 SW 会话内）；其它 host 不受影响 | ☑ |
| 6 | 锁定态重试 | 锁库（托盘/扩展锁定）→ 提交登录 → 浮层点「保存」 | 提示「密码库已锁定：请点击扩展图标解锁…」；解锁扩展后再点「保存」→ 成功 | ☑ |
| 7 | 旧 host 容忍 | （可选）注册回旧 sp-nmhost.exe（PR #34 之前构建）→ 提交登录 → 保存 | 浮层「当前 host 版本过旧，不支持自动保存」，无崩溃 | ☑ |
| 8 | 机密边界抽查 | 上述全程 | DevTools：无 chrome.storage/localStorage 写入；浮层/日志无明文密码回显 | ☑ |
| 9 | 拒绝路径零写入 | 只点「忽略」的提交 | `sp item list` 无新增条目 | ☑ |

- 全过 → 矩阵 SP-F-708 置 🟢（把本文件勾选结果回填）。
- 任何一项不符 → 记录现象于本文件下方「问题记录」，回修后重验。

## 验收记录

- **2026-10-30 用户验收：9 项全部通过**（含首轮"host 版本过旧"——根因=注册表指向的
  sp-desktop.exe 为 PR #35 之前构建，重建后探针确认 `sp.save-login` 存在（locked 语义）
  并复测通过；该现象本身即验证了旧 host 容忍路径 #7）。
- 附注：托盘实例与 NM 实例语义确认——NM 实例（浏览器拉起）无托盘属设计（design_docs/15）；
  托盘图标仅在用户/自启启动的托盘实例上出现。

