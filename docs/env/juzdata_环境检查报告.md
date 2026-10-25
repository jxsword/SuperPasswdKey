# 云服务器 juzdata 环境检查报告（SuperPasswdKey 项目）

- 检查日期：2026-10-25（SSH 只读实测，未做任何修改）
- 配套脚本：`docs/env/juzdata_setup.sh`（幂等分段；用户手动执行，见 §5）
- 服务器画像：与 VM ubuntu26 / WSL 是**不同基线**——Ubuntu 24.04（非 26.04）、用户 `sf`、2 核 3.8G 内存、磁盘紧张

## 1. 系统与资源

| 项 | 实测 | 评估 |
|---|---|---|
| OS | Ubuntu 24.04.5 LTS x86_64 | 与 26.04 机器 apt 候选版本不同（已按 24.04 适配脚本） |
| CPU / 内存 | 2 核 / 3.8Gi（可用 2.7Gi） | 满足 ≥4G 基线（勉强）；源码编译慢，优先预编译安装 |
| 磁盘 / | 20G 总量，**可用仅 6.5G**（67% 已用） | ⚠️ 系统盘紧张 |
| 磁盘 /data | **40G 数据盘，可用 33G**（vdb1） | ✅ Rust 工具链重定向至此（见 §3a） |
| docker data-root | **已在 /data/docker**（daemon.json 已配置 + 镜像加速），容器/镜像不占系统盘 | ✅ 无需改动 |

**磁盘分流结论（2026-10-25 与用户确认 /data 后更新）**：
- **装到 /data**（占大头）：Rust 工具链 `RUSTUP_HOME=/data/rust/rustup`、`CARGO_HOME=/data/rust/cargo`（stable minimal 约 700M + cargo 工具/注册表缓存）；docker 已在数据盘。
- **留系统盘**（小、apt 强制路径）：keepassxc/age/minisign 合计几十 MB、nodejs 约 30MB。
- 系统盘净增预计 <100M，数据盘净增预计 1–1.5G（不含 cargo_tools_full 段）。
| /var/lib/docker | 基本为空 | 无可清理空间 |
| systemd journal | 26.2M | 无需清理 |

## 2. 已满足（无需安装）

| 项目 | 实测 |
|---|---|
| docker engine | 29.8.2（比 26.04 VM 的 29.1.3 还新），服务运行中 |
| docker compose | v5.6.0 |
| docker 组 | **sf 已在 docker 组**（sudo/users/admin/docker） |
| git / gcc / make / pkg-config / curl | 2.43.0 / 13.3.0 / 4.3 / 1.8.1 / 8.5.0，全部达标 |

## 3. 缺失清单（按脚本段映射）

| 段 | 内容 | 24.04 候选版本 | 基线判定 |
|---|---|---|---|
| rust | rustup + stable（minimal profile 省盘） | 官方 dist 最新 stable | ✅ 装后达标 |
| keepassxc | keepassxc-cli（round-trip 测试对端） | 2.7.6 | ✅ ≥2.7 |
| age_minisign | age / minisign | 1.1.1 / 0.11 | ✅（rage 不在 24.04 源内，age 即满足基线） |
| cargo_tools_light | binstall / audit / deny | crates.io 预编译 | ✅ 轻量 |
| cargo_tools_full | nightly / cargo-fuzz / sccache | — | ⏸ **默认不装**：约 1.5G+，磁盘紧张；fuzz 是开发机活动，2 核生产机不适合，按需单独执行 |
| optional | nodejs / rclone | nodejs **18.19（低于基线 ≥20）** | ⚠️ node 达标需 NodeSource 源（另行决策）；rclone 正常 |
| docker_check | 仅验证 | 已装 | — |

## 4. 定位建议

该服务器适合作为 **M6 部署目标**（docker 就绪、组权限就绪、资源勉强够 Bitwarden Lite 200MB 级服务）而非开发机：2 核编译慢、磁盘紧、内存小。开发/round-trip 测试/fuzz 放 Windows + VM；服务器只跑构建产物与容器。

## 5. 手动执行方式

```bash
# 1) 从本机传输（Windows 侧 Git Bash）：
scp "E:/proj/SuperPasswdKey/docs/env/juzdata_setup.sh" juzdata:~/juzdata_setup.sh

# 2) SSH 登录后执行（sudo 需输密码）：
sudo bash ~/juzdata_setup.sh all        # 全部默认段（不含 cargo_tools_full）
sudo bash ~/juzdata_setup.sh cargo_tools_full   # 仅当确需 nightly/fuzz/sccache

# 直连 GitHub 慢时加代理（服务器侧可达的代理地址）：
sudo PROXY=http://<代理地址>:<端口> bash ~/juzdata_setup.sh all
```

- 脚本特性：幂等（已装自动 SKIP）；**磁盘余量双检查**（系统盘 <2G、/data <3G 拒续）；Rust 工具链装到 `/data/rust/`（环境变量幂等写入 ~/.profile，PATH 自动生效）；**权限设计**：/data 本身保持 root 所有不动，仅 rust 子树 chown 给 sf 及其主组（目录 0755，安装后 chown -R 统一校正）——其他 sudo 组用户可读可执行（可直接使用该工具链），写入/安装仅 sf 可做，且安装后以 sf 身份做读写功能验证；cargo 操作以 sf 身份执行不污染 /root；rustup-init 下载带 sha256 校验；PROXY 以环境变量传入（argv/环境方式，不拼接命令行参数）；docker data-root 已在 /data/docker，脚本不改动 docker 配置。
- 脚本已通过 Mimosa 安全扫描（初稿的"curl 管道 shell"字样与动态代理参数拼接已按要求改写）与 `bash -n` 语法校验。
- 执行完成后叫我做远程终验。

---

## 6. 执行记录（2026-10-25 补齐完成，终验通过）

- **过程排障（3 轮）**：① apt 锁被僵尸 `apt-get update` 占用（挂 18 天）→ kill；② `unattended-upgrade` 反复抢占 dpkg 锁且忽略 SIGTERM → `systemctl stop` 三个触发器后 `kill -9`，脚本完成后恢复服务；③ rustup-init 执行报 Permission denied → 脚本 bug（mktemp 临时目录 0700 root 属主未授权给 sf），已修复（目录随文件一并 chown）。
- **用户侧同步处理**：apt 源已切 aliyun（根治官方源间歇性僵死）；期间 unattended-upgrades 顺带升级了内核（重启后生效，不阻塞）。
- **终验结果（全部通过）**：
  - 权限：`/data/rust` 及子树 `sf:sf` 0755（其他 sudo 组用户可读可执行），`/data` 本身保持 root 未动；
  - Rust：rustc/cargo **1.99.1**（与 Windows/VM 三端对齐），装在 `/data/rust/`，`~/.profile` 已含 RUSTUP_HOME/CARGO_HOME/PATH；
  - 工具：cargo-binstall 1.26.1 / cargo-audit 0.22.2 / cargo-deny 0.20.2 / keepassxc-cli 2.7.6 / age 1.1.1 / minisign 0.11 / node 18.19.1 / rclone 1.60.1；
  - Docker：免 sudo 可用，data-root `/data/docker`（数据盘）。
- **未装（默认设计）**：cargo_tools_full（nightly/fuzz/sccache，磁盘敏感，按需 `sudo bash ~/juzdata_setup.sh cargo_tools_full`）。
- **注意事项**：node 18 低于基线 ≥20（24.04 源上限，需 NodeSource 源另行决策）；服务器已 80 包待升级（含内核），建议近期找窗口重启；apt 定时器已在脚本完成后由用户恢复。
