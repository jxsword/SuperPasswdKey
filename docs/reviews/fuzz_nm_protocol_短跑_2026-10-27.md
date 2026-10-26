# fuzz 短跑记录：native messaging 协议（nm_protocol）

- 日期：2026-10-27（feature/browser-ext，步骤 3）
- 依据：design_docs/11 §8 / SP-F-702（fuzz 不进 CI，本地短跑，结果入 docs/reviews/）
- 对象：`sp_nmhost::protocol::decode_frame`（帧边界/非法 UTF-8/未知版本与 codec）+
  `sp_nmhost::actions::handle`（锁定态会话动作分发，无库 IO）
- 目标：`fuzz/fuzz_targets/nm_protocol.rs`

## 执行

```bash
cd fuzz
cargo +nightly fuzz run nm_protocol --target x86_64-unknown-linux-gnu -- -max_total_time=60
```

- 工具链：cargo-fuzz 0.13.2 / nightly-x86_64-unknown-linux-gnu / ASan + sancov（WSL ubuntu2604）
- 注：与 env_parser 同款约束——cargo-fuzz 默认 musl 目标与 ASan 静态 libc 冲突，需显式 gnu target；
  仓库 rust-toolchain.toml 锁 stable，需 `+nightly` 覆盖

## 结果

- 执行次数：**264,421 runs / 61s，0 崩溃，0 挂起，0 OOM**
- 推荐字典生成正常退出（sancov 特征收敛）；语料不入仓（fuzz/.gitignore 已含）

## 结论

协议解码与动作分发在 26 万级随机输入下无 panic/UB 迹象；与单测
（protocol.rs 边界族、actions.rs 错误码族）互为印证。
