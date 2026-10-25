# VM ubuntu26 环境检查报告（SuperPasswdKey Linux 侧）

- 检查日期：2026-10-25
- 检查方式：SSH 只读检查（`ssh vm_ubuntu26_desktop '<命令>'`，主机别名对应 192.168.33.130，用户 ssy，免密登录正常，一次连通成功）
- 检查代理：环境检查代理 V
- 配套安装脚本：`docs/env/vm_ubuntu26_setup.sh`（未执行，仅落盘，手动 sudo 执行）

## 0. 系统概览（实测输出摘要）

| 项 | 实测值 |
|---|---|
| OS | Ubuntu 26.04 LTS（resolute，PRETTY_NAME="Ubuntu 26.04 LTS"） |
| 架构 | x86_64 |
| CPU | 8 核（nproc=8） |
| 内存 | 总 15Gi（MemTotal 15798848 kB），检查时可用 9.5Gi，swap 11Gi |
| 磁盘（/ 与 /home 同一分区 /dev/sda2） | 总 79G，已用 35G，**可用 40G**（已用 47%） |
| 运行时长 | up 5:00（检查时已开机 5 小时，3 个用户会话） |
| snap | snap/snapd 2.77.1 可用（本次脚本不依赖 snap） |

资源基线：磁盘可用 ≥20G → **满足**（40G）；内存 ≥4G → **满足**（15Gi/9.5Gi 可用）。

## 1. 重要发现：Rust 实际已安装（PATH 陷阱）

初检 `rustc --version` 在非交互 SSH 下报"未找到命令"，但进一步检查发现：

- `~/.cargo/bin/` 内有完整工具链（rustc/cargo/rustup/clippy/rustfmt/rust-analyzer 等）
- `~/.rustup/toolchains/` 内有 `stable-x86_64-unknown-linux-gnu`（active, default）
- `~/.profile:29` 与 `~/.bashrc:138-139` 已正确配置 `. "$HOME/.cargo/env"` 与 PATH

原因：非交互 SSH shell 不加载 `~/.profile`/`~/.bashrc`。`source ~/.cargo/env` 后实测版本：

```
rustc 1.99.1 (58cf8f9c3 2026-10-13)
cargo 1.99.1 (5f94df478 2026-08-27)
rustup 1.29.2 (d8c92c676 2026-10-21)
stable-x86_64-unknown-linux-gnu (active, default)
```

**结论：rustc 1.99.1 ≥ 1.85，与 Windows 侧 1.99.1 完全对齐，满足 M1–M3 要求。**
交互式登录 shell（正常 ssh 登录）下 PATH 正常，无需修复。

## 2. 检查结果矩阵

### 2.1 M1–M3 核心要求

| 项目 | 要求 | 实际状态 | 满足 |
|---|---|---|---|
| rustc（rustup 管理） | stable ≥1.85 | 1.99.1，rustup 1.29.2，stable 默认 | 是 |
| cargo | 与 rustc 配套 | 1.99.1 | 是 |
| git | ≥2.40 | 2.53.0 | 是 |
| gcc / g++ | build-essential 提供 | gcc/g++ 15.2.0（build-essential 12.12ubuntu2.26.04.2 状态 ii 已装） | 是 |
| make | build-essential 提供 | GNU Make 4.4.1 | 是 |
| pkg-config | 已安装 | 2.5.1 | 是 |
| openssl | 已安装 | OpenSSL 3.5.5（2026-01-27） | 是 |
| curl | 已安装 | 8.18.0 | 是 |
| keepassxc-cli | ≥2.7（KDBX round-trip 互操作测试） | **未安装**（命令不存在，dpkg 无记录；apt 源内候选 2.7.10） | **否** |
| age / rage | 至少其一 | **均未安装**（apt 源内 age 候选 1.2.1；**rage 不在 26.04 apt 源内**） | **否** |
| minisign | 已安装 | **未安装**（apt 源内候选 0.12） | **否** |

### 2.2 工具链卫生（M1–M3 伴随）

| 项目 | 要求 | 实际状态 | 满足 |
|---|---|---|---|
| cargo-audit | 已安装 | **未安装**（`~/.cargo/bin` 内无） | **否** |
| cargo-deny | 已安装 | **未安装** | **否** |
| cargo-fuzz | 已安装（需 nightly 工具链） | **未安装**，且 rustup 仅有 stable、**无 nightly** | **否** |
| sccache | 已安装 | **未安装** | **否** |
| cargo-binstall | 已安装 | **未安装** | **否** |

### 2.3 M6 服务器轨

| 项目 | 要求 | 实际状态 | 满足 |
|---|---|---|---|
| docker engine | 已安装运行 | **未安装**（`docker` 命令不存在；`systemctl status docker` 报 `Unit docker.service could not be found`；dpkg 无 docker-ce/docker.io/containerd） | **否** |
| compose v2 | 已安装 | **未安装**（apt 源内 docker-compose-v2 候选 2.40.3） | **否** |
| ssy 在 docker 组 | 是 | **不在**（`id -nG` 无 docker 组；docker 本身未装） | **否** |

### 2.4 M4/M5 可选项

| 项目 | 要求 | 实际状态 | 满足 |
|---|---|---|---|
| node ≥20 | 可选 | **未安装**（apt 源内 nodejs 候选 22.22.1，装后即 ≥20） | 否（可选） |
| pnpm | 可选 | **未安装** | 否（可选） |
| rclone | 可选 | **未安装**（apt 源内候选 1.60.1） | 否（可选） |
| syncthing | 可选 | **未安装**（apt 源内候选 1.29.5） | 否（可选） |

## 3. 缺失清单（汇总）

**M1 必补（阻塞核心开发/测试）：**
1. keepassxc（2.7.10，apt）— keepassxc-cli 是 KDBX 互操作测试基线
2. age（1.2.1，apt）— rage 无 apt 包，如需 rage 走 `cargo install rage` 或 GitHub release（可选补充）
3. minisign（0.12，apt）

**M1 伴随（工具链卫生，建议同批补齐）：**
4. cargo-binstall（先装，用于加速其余工具安装）
5. cargo-audit、cargo-deny、sccache
6. cargo-fuzz — 需先 `rustup toolchain install nightly`

**M6 前补：**
7. docker.io（29.1.3，apt）+ docker-compose-v2（2.40.3，apt）
8. `usermod -aG docker ssy` + 重新登录生效
9. （决策项，非安装）虚拟机是否常开作部署目标

**可选（M4/M5）：**
10. nodejs 22.22.1 + pnpm
11. rclone 1.60.1、syncthing 1.29.5

## 4. 资源事实（M6 部署目标评估，仅列事实）

- CPU 8 核 x86_64；内存 15Gi（检查时可用 9.5Gi，另有 11Gi swap）；根分区可用 40G（共 79G）
- 单分区 /dev/sda2 同时挂载 `/` 与 `/home`
- 检查时已连续运行 5 小时、3 个用户会话，负载 2.14/2.49/1.93（8 核下中等偏低）
- 满足磁盘 ≥20G、内存 ≥4G 基线；**是否常开 / 是否作为 M6 生产或准生产部署目标，由用户决定**（常开与否影响 docker 常驻与服务可用性）

## 5. 结论（按里程碑分层）

| 里程碑 | 状态 | 需补项 |
|---|---|---|
| M1–M3 核心 | **大部分就绪**：Rust 1.99.1 对齐 Windows 侧、git/gcc/make/pkg-config/openssl/curl 全满足 | M1 必补 keepassxc、age、minisign（均一条 apt 命令级） |
| 工具链卫生 | 未就绪 | cargo-audit / cargo-deny / cargo-fuzz(+nightly) / sccache / cargo-binstall 全缺 |
| M6 服务器轨 | 未就绪 | docker.io + docker-compose-v2 + docker 组成员；资源本身充足 |
| M4/M5 可选 | 未就绪 | node22/pnpm/rclone/syncthing 按需 |

总体：该 VM 基础盘面好（Ubuntu 26.04、8C/15G/40G 可用、Rust 已对齐），缺的全部是 apt 包与 cargo 工具，无版本冲突风险（apt 源内 keepassxc 2.7.10 ≥2.7、nodejs 22 ≥20、compose v2 2.40 均达标）。唯一注意点：**rage 不在 26.04 apt 源内**，age 即可满足基线，rage 需要时另装。

## 6. 安装脚本使用说明

```bash
# 用户手动 ssh 到 VM 后以 sudo 执行（本报告未执行任何安装）：
scp docs/env/vm_ubuntu26_setup.sh vm_ubuntu26_desktop:/tmp/
ssh vm_ubuntu26_desktop
sudo bash /tmp/vm_ubuntu26_setup.sh all            # 全部段
sudo bash /tmp/vm_ubuntu26_setup.sh base_deps      # 单段：base_deps|rust|keepassxc|age_minisign|cargo_tools|docker|optional
```

- 脚本幂等：已满足项（git/build-essential/pkg-config/openssl/curl/rust 1.99.1）自动检测并 SKIP，只补缺失。
- docker 段执行后需**注销重新登录**（或 `newgrp docker`）使 docker 组生效。
- 脚本不含任何凭据。

---

## 7. 执行记录（2026-10-25 补齐完成）

- **执行方式**：SSH 分解执行。原计划的 zcode-run 整脚本执行被 VM sudoers 白名单拦截——规则只放行 `apt/apt-get/dpkg/systemctl 子集/tee /etc/hosts`，`sudo -n bash 脚本` 不在白名单内（属有意设计，未为其扩权）。apt 段走白名单免密执行；cargo/rustup 段以 ssy 身份直接 SSH 执行（无需 sudo）；GitHub 预编译下载走代理 `http://192.168.33.1:10808`（用户指定）。
- **过程偏差**：① `sudo DEBIAN_FRONTEND=... apt-get` 因 sudo 环境变量白名单被拒，去掉该变量重跑成功；② pnpm 改用 `npm install -g pnpm --prefix ~/.local` 用户级安装（npm 全局装需 root，不在白名单）。
- **结果（全量终验通过）**：keepassxc-cli 2.7.10 / age 1.2.1 / minisign 0.12 / docker 29.1.3 + compose 2.40.3（服务 active）/ node 22.22.1 + pnpm 12.15.0（用户级）/ rclone 1.60.1 / syncthing 1.29.5 / nightly 工具链 / cargo-audit 0.22.2 / cargo-deny 0.20.2 / cargo-fuzz 0.13.2 / sccache 0.19.0 / cargo-binstall 1.26.1 —— **M1–M6 全部就绪**。
- **遗留 1 项（需手动，白名单外）**：`usermod -aG docker ssy` 不在 sudoers 白名单。请在 VM 终端执行一次（需输密码），然后重新登录验证：
  ```bash
  sudo usermod -aG docker ssy   # 之后重新登录，docker ps 免 sudo 即通
  ```
- VM 临时脚本副本（~/vm_setup.sh）已清理；本地脚本保留于 `docs/env/` 供幂等重跑。
