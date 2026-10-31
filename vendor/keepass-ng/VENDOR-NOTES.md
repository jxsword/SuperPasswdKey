# VENDOR-NOTES — keepass-ng 0.11.12 vendored 补丁

- 来源：crates.io 发布包 `keepass-ng-0.11.12.crate`（checksum 与根 Cargo.lock
  一致：`79cd2e22ea1c4e641659c6324cc7cb06f56000eb433d364618a4aaaaf5840830`），
  经 tar 解包入库。上游仓库 github.com/ssrlive/keepass-ng，MIT 许可（LICENSE
  随包保留）。
- 接线：根 `Cargo.toml` `[patch.crates-io]` → `keepass-ng = { path = "vendor/keepass-ng" }`。
- 升级上游时：以新版本发布包重新解包，rebase 下列补丁并更新本文件。

## 改动清单（S1d-1，DR-017）

仅 `src/db/types/entry.rs`，新增两个公开方法（现有 API 零改动、零破坏）：

1. `set_additional_attribute_value(key, Option<Value<String>>)` —— 自定义属性
   的受保护写入口。上游仅暴露未保护变体 `set_additional_attribute`（内部硬走
   `set_unprotected_field_pair`），受保护写入函数 `set_protected_field_pair`
   为 `pub(crate)`；而 KDBX4 格式原生支持任意 string 字段的 ProtectedValue
   （KeePassXC"受保护自定义属性"即此语义）。本方法按传入 `Value::Protected/
   Unprotected` 直写 fields map，序列化层（writer/reader）零改动。
2. `additional_attributes_typed() -> Vec<(String, Value<String>)>` —— 读侧保留
   保护位。上游 `additional_attributes()` 返回 `(String, String)` 并透明解密，
   丢弃 `is_protected()` 信息，读回面无法还原受保护标记。

## 与本仓库的关系

- 消费方：`crates/sp-kdbx`（apply_model/model_from_entry，DR-005 v0.1 限制
  "自定义字段 protected 恒 false"在本批闭环）。
- 互操作护栏：`crates/sp-kdbx/tests/roundtrip.rs` 受保护自定义字段对拍用例
  （KeePassXC 掩码语义 + 双向 roundtrip）。
- 上游若后续版本公开等价 API，可删除本 patch 回归 crates.io 版本。
