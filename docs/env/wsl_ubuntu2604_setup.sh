#!/usr/bin/env bash
# wsl_ubuntu2604_setup.sh — SuperPasswdKey Linux 侧环境安装脚本（WSL ubuntu2604）
# 由 SuperPasswdKey 环境检查生成于 2026-10-25，依据 docs/env/WSL_ubuntu2604_环境检查报告.md
# 手动执行：sudo bash wsl_ubuntu2604_setup.sh [all|keepassxc|age_minisign|cargo_tools|align_rust|docker|optional]
# 特性：幂等（已装自动跳过）；cargo/rustup 操作以 ssy 用户执行，不污染 /root；不含任何凭据。
# 语法校验：bash -n 通过（2026-10-25）

set -euo pipefail

TARGET_USER="ssy"
WSL_USER_HOME="/home/${TARGET_USER}"

as_user() { sudo -u "${TARGET_USER}" -H bash -lc "$*"; }

have() { command -v "$1" >/dev/null 2>&1; }

seg_keepassxc() {
  echo "==> [keepassxc] KDBX round-trip 互操作测试对端（apt 候选 2.7.10，满足 ≥2.7）"
  if have keepassxc-cli; then
    echo "    已安装：$(keepassxc-cli --version 2>&1 | head -1)，跳过"
  else
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y keepassxc
    echo "    完成：$(keepassxc-cli --version 2>&1 | head -1)"
  fi
}

seg_age_minisign() {
  echo "==> [age_minisign] F10 加密归档与签名工具（apt：age 1.2.1 / minisign 0.12）"
  local need=()
  have age      || need+=(age)
  have minisign || need+=(minisign)
  if [ ${#need[@]} -eq 0 ]; then
    echo "    age 与 minisign 均已安装，跳过"
  else
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y "${need[@]}"
  fi
  echo "    注意：apt 的 age 包提供 age/age-keygen 命令（不含 rage）；与 Windows 侧 rage 系语法兼容但命令名不同。"
}

seg_cargo_tools() {
  echo "==> [cargo_tools] cargo-deny + nightly + cargo-fuzz（以 ${TARGET_USER} 执行，约 3-8 分钟）"
  if have cargo-deny; then
    echo "    cargo-deny 已安装：$(as_user 'cargo deny --version' 2>/dev/null || cargo-deny --version)，跳过"
  else
    as_user 'cargo binstall -y cargo-deny || cargo install cargo-deny --locked'
  fi
  echo "    -- nightly 工具链（cargo-fuzz 依赖 -Zsanitizer）--"
  if as_user 'rustup toolchain list' | grep -q nightly; then
    echo "    nightly 已存在，跳过"
  else
    as_user 'rustup toolchain install nightly --profile minimal'
  fi
  if have cargo-fuzz; then
    echo "    cargo-fuzz 已安装，跳过"
  else
    as_user 'cargo binstall -y cargo-fuzz || cargo install cargo-fuzz --locked'
  fi
}

seg_align_rust() {
  echo "==> [align_rust] 可选对齐：1.99.0 -> 最新 stable（与 Windows 侧一致，不阻塞）"
  as_user 'rustup update'
}

seg_docker() {
  echo "==> [docker] M6 服务器轨：docker engine + compose v2（Ubuntu 源方案）"
  if have docker; then
    echo "    docker 已安装：$(docker --version)，跳过安装"
  else
    apt-get update -qq
    # docker.io = Ubuntu 源 moby 引擎；docker-compose-v2 提供 `docker compose` 子命令插件
    DEBIAN_FRONTEND=noninteractive apt-get install -y docker.io docker-compose-v2
  fi
  systemctl enable --now docker
  if id -nG "${TARGET_USER}" | tr ' ' '\n' | grep -qx docker; then
    echo "    ${TARGET_USER} 已在 docker 组"
  else
    usermod -aG docker "${TARGET_USER}"
    echo "    已将 ${TARGET_USER} 加入 docker 组——需要重新登录 WSL 会话后免 sudo 生效"
  fi
  echo "    验证：docker --version && docker compose version && docker run --rm hello-world"
}

seg_optional() {
  echo "==> [optional] M4/M5 可选项：nodejs(22.x)+pnpm / rclone / syncthing"
  local need=()
  have node    || need+=(nodejs)
  have rclone  || need+=(rclone)
  have syncthing || need+=(syncthing)
  if [ ${#need[@]} -eq 0 ]; then
    echo "    node/rclone/syncthing 均已安装，跳过"
  else
    apt-get update -qq
    DEBIAN_FRONTEND=noninteractive apt-get install -y "${need[@]}"
  fi
  if ! have pnpm; then
    echo "    -- pnpm（corepack 方式，以 ${TARGET_USER} 执行）--"
    as_user 'corepack enable pnpm 2>/dev/null || npm install -g pnpm' || echo "    pnpm 安装失败：请手动 npm install -g pnpm"
  fi
}

main() {
  local seg="${1:-all}"
  echo "SuperPasswdKey WSL 环境安装（段：${seg}）  用户：${TARGET_USER}  时间：$(date '+%F %T')"
  case "${seg}" in
    keepassxc)     seg_keepassxc ;;
    age_minisign)  seg_age_minisign ;;
    cargo_tools)   seg_cargo_tools ;;
    align_rust)    seg_align_rust ;;
    docker)        seg_docker ;;
    optional)      seg_optional ;;
    all)           seg_keepassxc; seg_age_minisign; seg_cargo_tools; seg_docker; seg_optional ;;
    *) echo "未知段名：${seg}（可用：all|keepassxc|age_minisign|cargo_tools|align_rust|docker|optional）"; exit 1 ;;
  esac
  echo "完成。"
}

main "$@"
