# 06 F10 导出与 TOTP

- 状态：v1.0｜追溯：SP-F-501/502/503、SP-F-601、基线§5｜图：mermaid

## 1. 导出双模式

```mermaid
flowchart TD
    START["sp export"] --> COLLECT["收集全量: 条目+自定义字段+TOTP+history+附件"]
    COLLECT --> DEFAULT{"默认路径"}
    DEFAULT --> ENC["加密归档: zip 容器 → age 口令加密\n(口令交互输入,二次确认一致)"]
    ENC --> OUT1["--out <文件> 写入\n+ SHA256 校验和输出"]
    START -->|"--plaintext"| WARN["警告文案 → 交互重输主密码确认"]
    WARN -->|确认| STDOUT["默认 --stdout 流式输出 JSON\n(不落盘,SP-F-502)"]
    WARN -->|拒绝| ABORT["中止(退出码 0,无副作用)"]
    STDOUT --> SUMMARY["字段损失摘要\n包含: 全字段/附件/TOTP/history\n不含: passkey(v0.1 未存)"]
    ENC --> SUMMARY
```

红线落地：默认加密归档（行业空白点）；明文三重防护（flag+重输+警告）；stdout 流式一等（避免临时文件）；残留治理采用"不落盘"而非 shred（gopass ADR 结论：SSD 上不可靠）。

## 2. 加密归档结构

```
archive.age  ← age (scrypt passphrase 钳) 加密
  └─ archive.zip
       ├─ manifest.json      # 版本/时间/计数/字段摘要
       ├─ entries.json       # 全量条目（含 history）
       ├─ attachments/       # 二进制附件（原名+条目引用）
       └─ checksums.txt      # 内容 SHA256
```

## 3. TOTP（SP-F-601）

```mermaid
flowchart LR
    A["条目 custom attr 'otp'\nKeePass 约定: seed?period=&digits="] --> B["解析参数\n(6位/30s 默认,尊重属性)"]
    B --> C["RFC 6238 HOTP->TOTP\nHMAC-SHA1(按约定)"]
    C --> D["sp item get --totp 输出当前码"]
    D --> E["验收: RFC 6238 官方向量\n+ KeePassXC 显示值对拍"]
```

## 4. 测试挂钩

age 归档端到端解密断言；明文导出拒绝路径（无确认→退出码 0 无副作用）；stdout 流式端到端；字段摘要与实际内容一致性；TOTP 向量（SP-C-106 来源锁定）+ keepassxc 对拍。
