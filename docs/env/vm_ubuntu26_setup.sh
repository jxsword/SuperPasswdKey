#!/usr/bin/env bash
#
# SuperPasswdKey — VM ubuntu26 (Ubuntu 26.04 LTS, 192.168.33.130, 用户 ssy) 环境补齐脚本
# 由 SuperPasswdKey 环境检查生成于 2026-10-25，手动执行：sudo bash vm_ubuntu26_setup.sh [段名|all]
#
# 段名:
#   all          依序执行全部段
#   base_deps    git / build-essential / pkg-config / openssl / curl（2026-10-25 检查已全满足，将全部 SKIP）
#   rust         rustup + stable 工具链（检查时已装 rustc 1.99.1，将 SKIP；仅在其他机器未装时生效）
#   keepassxc    keepassxc >= 2.7（apt 源内候选 2.7.10），提供 keepassxc-cli 供 KDBX 互操作测试
#   age_minisign age + minisign（apt 源内候选 1.2.1 / 0.12）。段名别名: age+minisign
#                注: rage 不在 Ubuntu 26.04 apt 源内，基线由 age 满足；如需 rage 请以 ssy 身份执行
#                `cargo install rage`（不走本脚本）
#   cargo_tools  cargo-binstall / cargo-audit / cargo-deny / sccache / cargo-fuzz（fuzz 需先装 nightly 工具链）
#   docker       docker.io + docker-compose-v2（apt 源内候选 29.1.3 / 2.40.3）+ ssy 加入 docker 组（需重新登录生效）
#   optional     M4/M5 可选项: nodejs(源内 22.22.1) + npm + pnpm + rclone(1.60.1) + syncthing(1.29.5)
#
# 特性: 幂等（已安装自动跳过）；cargo 相关操作以真实用户（SUDO_USER，缺省 ssy）身份执行，绝不装入 /root；
#       本脚本不含任何凭据。
#
set -euo pipefail

REAL_USER="${SUDO_USER:-ssy}"
case "$REAL_USER" in
    ''|*[!a-zA-Z0-9_-]*) echo "错误: 非法用户名 '$REAL_USER'（来自 SUDO_USER）" >&2; exit 1 ;;
esac
getent passwd "$REAL_USER" >/dev/null 2>&1 || { echo "错误: 用户 $REAL_USER 不存在" >&2; exit 1; }
REAL_HOME="$(getent passwd "$REAL_USER" | cut -d: -f6)"
CARGO_BIN="$REAL_HOME/.cargo/bin"
NEED_APT_UPDATE=1

log()  { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*"; }
skip() { printf '[SKIP] %s\n' "$*"; }

pkg_installed() { dpkg-query -W -f='${Status}' "$1" 2>/dev/null | grep -q 'install ok installed'; }

ensure_apt_update() {
    if [ "$NEED_APT_UPDATE" -eq 1 ]; then
        log "apt-get update ..."
        apt-get update -qq
        NEED_APT_UPDATE=0
    fi
}

apt_install() {
    local p
    for p in "$@"; do
        if pkg_installed "$p"; then
            skip "已安装: $p"
        else
            ensure_apt_update
            log "安装: $p"
            DEBIAN_FRONTEND=noninteractive apt-get install -y "$p"
        fi
    done
}

# 以真实用户身份运行登录 shell（加载 ~/.profile -> ~/.cargo/env，PATH 含 ~/.cargo/bin）
as_user() { sudo -u "$REAL_USER" -H bash -lc "$*"; }

# ---------- 段: base_deps（2026-10-25 检查结果: 全部已满足，预期全部 SKIP） ----------
seg_base_deps() {
    log "== base_deps: git >=2.40 / build-essential / pkg-config / openssl / curl =="
    apt_install git build-essential pkg-config libssl-dev openssl curl
}

# ---------- 段: rust（检查结果: rustc 1.99.1 已装且 >=1.85，预期 SKIP） ----------
seg_rust() {
    log "== rust: rustup + stable (>=1.85) =="
    if [ -x "$CARGO_BIN/rustc" ]; then
        local v major minor
        v="$("$CARGO_BIN/rustc" --version | awk '{print $2}')"
        major="${v%%.*}"
        minor="$(echo "$v" | cut -d. -f2)"
        if [ "$major" -gt 1 ] || { [ "$major" -eq 1 ] && [ "$minor" -ge 85 ]; }; then
            skip "rustc $v (rustup 管理) 已满足 >=1.85"
            return 0
        fi
        log "rustc $v 低于 1.85，升级 stable ..."
        as_user 'rustup update stable && rustup default stable'
        return 0
    fi
    log "未检测到 rustup，为用户 $REAL_USER 安装（官方 rustup-init 二进制：下载 + sha256 校验后执行）..."
    apt_install curl ca-certificates
    local triple tmp
    triple="$(uname -m)-unknown-linux-gnu"
    tmp="$(mktemp -d)"
    log "下载 rustup-init（$triple）..."
    curl -fsSL -o "$tmp/rustup-init" "https://static.rust-lang.org/rustup/dist/$triple/rustup-init"
    curl -fsSL -o "$tmp/rustup-init.sha256" "https://static.rust-lang.org/rustup/dist/$triple/rustup-init.sha256"
    if ! ( cd "$tmp" && sha256sum -c rustup-init.sha256 ); then
        rm -rf "$tmp"
        log "错误: sha256 校验未通过，中止（可到 https://rustup.rs 手动安装）"
        return 1
    fi
    chown "$REAL_USER" "$tmp/rustup-init"
    chmod 0755 "$tmp/rustup-init"
    if ! sudo -u "$REAL_USER" -H "$tmp/rustup-init" -y --default-toolchain stable --profile default; then
        rm -rf "$tmp"
        log "错误: rustup-init 执行失败"
        return 1
    fi
    rm -rf "$tmp"
    log "rustup 安装完成（新终端中 PATH 自动经 ~/.profile 生效）"
}

# ---------- 段: keepassxc ----------
seg_keepassxc() {
    log "== keepassxc: >=2.7 (keepassxc-cli) =="
    apt_install keepassxc
    if command -v keepassxc-cli >/dev/null 2>&1; then
        log "keepassxc-cli 可用: $(keepassxc-cli --version 2>&1 | head -1)"
    fi
}

# ---------- 段: age + minisign（别名 age+minisign） ----------
seg_age_minisign() {
    log "== age + minisign =="
    apt_install age minisign
    log "age: $(age --version 2>&1); minisign: $(minisign -v 2>&1)"
}

# ---------- 段: cargo 工具（工具链卫生） ----------
cargo_tool() { # cargo_tool <可执行名> <crate名> [额外 cargo install 参数...]
    local bin="$1" crate="$2"
    shift 2
    if [ -x "$CARGO_BIN/$bin" ]; then
        skip "已安装: $bin"
        return 0
    fi
    log "安装: $crate"
    if [ -x "$CARGO_BIN/cargo-binstall" ]; then
        as_user "cargo binstall -y $crate $*" || as_user "cargo install --locked $crate $*"
    else
        as_user "cargo install --locked $crate $*"
    fi
}

seg_cargo_tools() {
    log "== cargo_tools: binstall / audit / deny / sccache / fuzz(+nightly) =="
    if [ ! -x "$CARGO_BIN/cargo" ]; then
        echo "错误: 未检测到 $CARGO_BIN/cargo，请先执行: sudo bash $0 rust" >&2
        exit 1
    fi
    # nightly（cargo-fuzz 依赖）
    if as_user 'rustup toolchain list' 2>/dev/null | grep -q nightly; then
        skip "nightly 工具链已安装"
    else
        log "安装 nightly 工具链（minimal profile，仅供 cargo-fuzz）..."
        as_user 'rustup toolchain install nightly --profile minimal'
    fi
    cargo_tool cargo-binstall cargo-binstall
    cargo_tool cargo-audit  cargo-audit
    cargo_tool cargo-deny   cargo-deny
    cargo_tool sccache      sccache
    cargo_tool cargo-fuzz   cargo-fuzz --locked  # 常规安装；nightly 已在上一步就绪
}

# ---------- 段: docker（M6 服务器轨） ----------
seg_docker() {
    log "== docker: engine + compose v2 + docker 组 =="
    apt_install docker.io docker-compose-v2
    log "启用 docker 服务..."
    systemctl enable --now docker
    command docker --version || true
    command docker compose version || true
    if id -nG "$REAL_USER" | tr ' ' '\n' | grep -qx docker; then
        skip "$REAL_USER 已在 docker 组"
    else
        log "将 $REAL_USER 加入 docker 组..."
        usermod -aG docker "$REAL_USER"
        cat <<EOF

[注意] 已将 $REAL_USER 加入 docker 组。
请注销并重新登录 VM（或临时执行 newgrp docker）后，用 `docker ps`（无需 sudo）验证生效。

EOF
    fi
}

# ---------- 段: 可选项（M4/M5） ----------
seg_optional() {
    log "== optional: nodejs + npm + pnpm / rclone / syncthing =="
    apt_install nodejs npm rclone syncthing
    if command -v pnpm >/dev/null 2>&1; then
        skip "已安装: pnpm ($(pnpm --version))"
    else
        log "安装 pnpm（npm 全局）..."
        npm install -g pnpm
    fi
    log "node: $(node --version 2>&1) / pnpm: $(pnpm --version 2>&1)"
}

main() {
    local seg="${1:-}"
    if [ "$(id -u)" -ne 0 ]; then
        echo "错误: 请以 sudo 执行（目标用户由 SUDO_USER 决定，当前缺省 $REAL_USER）" >&2
        exit 1
    fi
    if [ -z "$seg" ]; then
        echo "用法: sudo bash $0 [base_deps|rust|keepassxc|age_minisign|age+minisign|cargo_tools|docker|optional|all]" >&2
        exit 1
    fi
    case "$seg" in
        base_deps)    seg_base_deps ;;
        rust)         seg_rust ;;
        keepassxc)    seg_keepassxc ;;
        age_minisign|age+minisign) seg_age_minisign ;;
        cargo_tools)  seg_cargo_tools ;;
        docker)       seg_docker ;;
        optional)     seg_optional ;;
        all)
            seg_base_deps
            seg_rust
            seg_keepassxc
            seg_age_minisign
            seg_cargo_tools
            seg_docker
            seg_optional
            log "全部段完成。docker 组如本次新增，请重新登录后验证。"
            ;;
        *)
            echo "错误: 未知段名 '$seg'" >&2
            echo "用法: sudo bash $0 [base_deps|rust|keepassxc|age_minisign|age+minisign|cargo_tools|docker|optional|all]" >&2
            exit 1
            ;;
    esac
    log "done: $seg"
}

main "$@"
