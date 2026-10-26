# fuzz 短跑记录：.env 解析器（env_parser）

- 日期：2026-10-27（feature/linux-port，v0.1.2-linux 步骤 2）
- 依据：design_docs/07 §1 解析器属性家族 / §3（fuzz 不进 CI，本地短跑，结果入 docs/reviews/）
- 对象：`sp_cli::f8::parse_env_str`（F8 .env 解析器，UTF-8/# 注释/引号/$VAR 展开/转义/多行值全路径）
- 目标：`fuzz/fuzz_targets/env_parser.rs`（非 UTF-8 字节序列按 design_docs/04 §3 规范直接跳过，不入解析路径）

## 执行

```bash
cd fuzz
cargo +nightly fuzz run env_parser --target x86_64-unknown-linux-gnu -- -max_total_time=60
```

- 工具链：cargo-fuzz 0.13.2 / nightly-x86_64-unknown-linux-gnu / ASan + sancov
- 注：cargo-fuzz 0.13 默认 musl 目标与 ASan 静态 libc 冲突，需显式 `--target x86_64-unknown-linux-gnu`

## 结果

- 执行次数：**1,307,974 runs / 61s，0 崩溃，0 挂起，0 OOM**
- 覆盖：词法分支（注释/引号/展开/转义/POSIX 名前缀化）均进入（sancov 特征库已收敛，推荐字典生成正常退出）
- 处置：无需修复；语料（fuzz/corpus、fuzz/artifacts）不入仓（.gitignore 已含）

## 结论

解析器在百万级随机输入下无 panic/UB 迹象，与既有 proptest 属性（env_parse_never_panics、masker_never_leaks_across_chunks）互为印证。
