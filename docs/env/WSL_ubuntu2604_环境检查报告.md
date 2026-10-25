# WSL ubuntu2604 环境检查报告（SuperPasswdKey 项目）

- 检查日期：2026-10-25（全部数据来自 `wsl -d ubuntu2604 -u ssy -- bash -lc` 只读实测输出，未做任何安装/修改）
- 配套脚本：`wsl_ubuntu2604_setup.sh`（手动执行：`sudo bash wsl_ubuntu2604_setup.sh [段名|all]`，幂等，已装自动跳过）
- 用途：SuperPasswdKey 的 Linux 侧开发/交叉验证环境（M1 round-trip 测试、cargo-fuzz、M6 服务器轨 docker 开发环境）

## 1. 系统与资源

| 项目 | 实测 | 基线要求 | 结论 |
|---|---|---|---|
| 系统 | Ubuntu 26.04.1 LTS (Resolute Raccoon) x86_64 | Linux | ✅ |
| CPU | 8 核 | ≥4 | ✅ |
| 内存 | 31Gi（可用 25Gi） | ≥4G | ✅ |
| 磁盘（/ 与 $HOME 同盘） | 1007G 总量，可用 745G | ≥20G | ✅ |
| locale | zh_CN.UTF-8 | — | 记录（无阻塞） |
| Shell | /bin/bash | — | ✅ |

## 2. 检查结果矩阵

| 类别 | 项目 | 要求 | 实测 | 结论 |
|---|---|---|---|---|
| M1 核心 | rustc/cargo | ≥1.85，rustup 管理 | 1.99.0（rustup 1.29.1，stable 默认） | ✅（与 Windows 侧 1.99.1 差一个 patch，可 `rustup update` 对齐） |
| M1 核心 | nightly 工具链 | cargo-fuzz 前置 | 无 | ❌ 缺（随 cargo_tools 段补） |
| M1 核心 | git | ≥2.40 | 2.53.0 | ✅ |
| M1 核心 | gcc/g++ | build-essential | 16.0.1（实验版主干） | ✅ |
| M1 核心 | make | — | 4.4.1 | ✅ |
| M1 核心 | pkg-config | — | 2.5.1 | ✅ |
| M1 核心 | OpenSSL | — | 3.5.5 | ✅ |
| M1 核心 | curl | — | 8.18.0 | ✅ |
| M1 核心 | KeePassXC（keepassxc-cli） | ≥2.7（round-trip 互操作测试对端） | 未安装；apt 候选 2.7.10+dfsg1-2ubuntu1 | ❌ 缺（keepassxc 段补，候选版本满足） |
| M1 核心 | age | F10 加密归档 | 未安装；apt 候选 1.2.1-1build1（注意：apt 的 `age` 包提供 `age`/`age-keygen` 命令，`rage` 不在 26.04 源内） | ❌ 缺（age_minisign 段补） |
| M1 核心 | minisign | 签名验证 | 未安装；apt 候选 0.12-1build1 | ❌ 缺 |
| 工具链卫生 | cargo-audit | — | 0.22.2 | ✅ |
| 工具链卫生 | sccache | — | 0.18.0 | ✅ |
| 工具链卫生 | cargo-binstall | — | 1.24.0 | ✅ |
| 工具链卫生 | cargo-deny | — | 未安装 | ❌ 缺 |
| 工具链卫生 | cargo-fuzz | — | 未安装（且无 nightly） | ❌ 缺 |
| 工具链卫生 | 其他已有 | — | bacon、cargo-nextest、cargo-generate、cargo-outdated、rust-analyzer 等（用户既有） | ✅（额外收益） |
| M6 服务器轨 | docker engine | compose v2 | 未安装（`docker` 命令不存在，docker.service inactive） | ❌ 缺（docker 段补） |
| M6 服务器轨 | docker 组成员 | ssy 可免 sudo 用 docker | **ssy 已在 docker 组** | ✅（装完引擎即用，需重新登录生效） |
| M4/M5 可选 | node / pnpm | ≥20 | 未安装 | ⏸ 可选段 |
| M4/M5 可选 | rclone / syncthing | — | 未安装 | ⏸ 可选段 |

## 3. 缺失清单与分层结论

**M1 前必补（5 项）**：
1. keepassxc（apt 2.7.10）
2. age（apt 1.2.1）
3. minisign（apt 0.12）
4. cargo-deny（cargo binstall 预编译）
5. cargo-fuzz + nightly 工具链（rustup toolchain install nightly；cargo-fuzz 用 binstall/源码）

**对齐项（可选）**：`rustup update` 把 1.99.0 → 1.99.1 与 Windows 侧一致（不阻塞）。

**M6 前补（1 项）**：docker engine + compose v2（Ubuntu 源 `docker.io` + `docker-compose-v2` 插件，或 docker 官方源 docker-ce）；ssy 已在 docker 组，装完重新登录即可免 sudo。

**可选项（按需）**：nodejs（26.04 源内 22.x）+ pnpm（corepack 或 npm -g）、rclone、syncthing。

## 4. 风险备注

- gcc 为 Ubuntu 26.04 的实验性主干构建（16.0.1 20260322 experimental trunk）：本轮 round-trip 测试与常规构建无碍；若后续遇到编译器 bug，可 `apt install` 切换稳定 gcc-15/14 并用 `CC/CXX/RUSTFLAGS` 指定。
- apt 的 `age` 包不含 `rage` 命令名——F10 代码若引用 `rage` 二进制名需注意命令差异（`age` 与 `rage` 语法兼容但命令名不同）；Windows 侧已装的是 rage 系（`cargo install age` 失败改走 GitHub Releases 的 rage 二进制）。
- 磁盘/内存充裕，无容量风险。
