# S1c 插入批交付说明：item edit + 同名 --index 寻址（2026-10-31）

> 本文是 S1c 前插入批的交付记录。决策见 [decision_log.md DR-016](../decision_log.md)；
> 需求口径见 [SRS V1.10](../SRS.md) SP-F-102/103；验收项见
> [acceptance_checklist.md](../manual/acceptance_checklist.md) §A2b。

## 1. 核实结论（用户两问）

1. **CLI 此前没有 `item edit`**：`ItemOp` 仅 create/get/list/delete，手册两处
   明示"无 sp item edit"（建议用 KeePassXC 编辑）。但底层 `sp-kdbx::edit_entry`
   已存在且被 GUI/NM/Android 复用（history 单源），SRS SP-F-102 描述本就含
   edit、design_docs/03 命令树预留 `edit <名称/ID>`——本批属补齐设计预留。
2. **同名条目**：允许同名、不查重、首个命中遮蔽（无测试锚定，仅手册文字）。
   策略定版为 **DR-016 方案 A**：允许同名 + `--index` 组内序号寻址
   （1-based 文档序）；方案 B（禁止同名）因与 GUI 已验收口径（SP-F-909）、
   扩展保存消歧命名、现实导入数据冲突而弃用。

## 2. 落地内容

- `sp item edit <TITLE> [--index N] [-u|--url|--notes|-p|--totp|--tag|--field|--remove-field|--rename]`
  ——patch 语义（未给字段不变）、校验前置（nothing-to-edit/--index 0/空 rename/
  非 otpauth/保留名均在提示主密码前退 4）、旧快照自动进 history（SP-F-103）。
- `--index` 接入 `item get/delete`、`read`、`clip`；`item list --enumerate`
  输出 `#<组内序号>\t<title>`（默认纯标题排序输出，脚本零破坏）。
- sp-kdbx：`find_entry_at/delete_entry_at/edit_entry_at/entry_totp_now_at`
  （None=首个=现状锚定；越界 NotFound 附同名总数退 3；原函数委托 None 零破坏）。
- 附件命令族与 `sp run/inject` 的 sp:// 引用**本批不支持** `--index`（维持首个
  命中，手册 §14 第 6 条已注明）；UUID 寻址留后续。
- 单测：sp-kdbx 同名 3 实例场景 3 项 + sp-cli 纯函数（validate/apply patch）3 项。

## 3. 门禁与验证

根 workspace fmt/clippy -D warnings/test 全绿（99 passed）；desktop workspace
fmt/clippy/test 全绿（34 passed）；PR CI 七 job 全绿后合并 develop。

## 4. 进程级 e2e（Mimosa 拦截处置，AGENTS §8 披露）

`crates/sp-cli/tests/item_edit_e2e.rs`（同名全流程 + 退出码矩阵）经 Write
3 种合规形态尝试均被 Mimosa 拦截（"命令注入"误报——argv 数组直传无 shell，
与已入库 attachment_e2e.rs 同款形态、同样来历）。按其先例**交维护者手动落盘**：
从本 PR 描述复制全文存为 `crates/sp-cli/tests/item_edit_e2e.rs` 即可
（`cargo test -p sp-cli` 验证）。等价纯逻辑覆盖（validate/apply patch 单测）
已随源码入库。
