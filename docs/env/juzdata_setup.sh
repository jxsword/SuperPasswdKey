#!/usr/bin/env bash
#
# SuperPasswdKey — 云服务器 juzdata（Ubuntu 24.04.5 LTS x86_64，用户 sf）环境补齐脚本
# 由 SuperPasswdKey 环境检查生成于 2026-10-25。手动执行：sudo bash juzdata_setup.sh [段名|all]
#
# 段名:
#   all             依序执行：base_deps rust keepassxc age_minisign cargo_tools_light docker_check optional
#   base_deps       git/build-essential/pkg-config/openssl/curl（检查时已满足，将 SKIP）
#   rust            rustup + stable（--profile minimal 省盘；下载 rustup-init 二进制 + sha256 校验后执行，
#                   不通过 shell 管道执行任何远程脚本）
#   keepassxc       keepassxc 2.7.6（24.04 源内候选，满足 ≥2.7），提供 keepassxc-cli 供 KDBX 互操作测试
#   age_minisign    age 1.1.1 + minisign 0.11（24.04 源内候选；rage 不在 24.04 源内，基线由 age 满足）
#   cargo_tools_light  cargo-binstall + cargo-audit + cargo-deny（预编译下载，轻量）
#   cargo_tools_full   nightly + cargo-fuzz + sccache（约 1.5G+，2 核编译慢；默认不装、按需执行）
#   docker_check    docker 已装 29.8.2 + compose v5.6.0，sf 已在 docker 组 —— 仅验证，不安装
#   optional        nodejs 18.19（24.04 源内最高，低于基线 ≥20，仅 M4/M5 前端构建验证用）+ rclone
#
# 磁盘注意：系统盘检查时仅剩 6.5G。Rust 工具链（占大头，约 1–1.5G）默认安装到 /data/rust
#           （DATA_ROOT 可改）；apt 小包（keepassxc/age/minisign/node 共约 50M）留在系统盘。
#           脚本对两个盘都做余量检查：系统盘 <2G、数据盘 <3G 拒续。
#           Docker data-root 已指向 /data/docker（daemon.json 已有配置），本脚本不改动 docker。
#
# 代理：本机不在 192.168.33.x 内网，VM 侧代理不可用。若 GitHub 直连慢，执行时传入环境变量：
#       sudo PROXY=http://<你的服务器侧代理>:<端口> bash juzdata_setup.sh all
#       （PROXY 会以环境变量方式传给 curl 与 cargo 工具，留空=直连）
#
# 特性: 幂等（已装自动跳过）；cargo 操作以真实用户（SUDO_USER，缺省 sf）身份执行，绝不装入 /root；
#       本脚本不含任何凭据。
#
set -euo pipefail

REAL_USER="${SUDO_USER:-sf}"
case "$REAL_USER" in
    ''|*[!a-zA-Z0-9_-]*) echo "错误: 非法用户名 '$REAL_USER'（来自 SUDO_USER）" >&2; exit 1 ;;
esac
getent passwd "$REAL_USER" >/dev/null 2>&1 || { echo "错误: 用户 $REAL_USER 不存在" >&2; exit 1; }
REAL_HOME="$(getent passwd "$REAL_USER" | cut -d: -f6)"
# Rust 工具链重定向到数据盘（/data 挂载点不存在时回退到家目录，同样不占系统盘大头）
DATA_ROOT="/data"
if mountpoint -q /data 2>/dev/null && [ -w /data ]; then
    RUST_HOME="$DATA_ROOT/rust"
else
    log "警告: /data 不可用，Rust 工具链回退安装到 $REAL_HOME/.rust-cargo（系统盘）"
    RUST_HOME="$REAL_HOME/.rust-cargo"
fi
RUSTUP_HOME_DIR="$RUST_HOME/rustup"
CARGO_HOME_DIR="$RUST_HOME/cargo"
CARGO_BIN="$CARGO_HOME_DIR/bin"
PROXY="${PROXY:-}"
if [ -n "$PROXY" ]; then
    # 以环境变量方式提供代理，curl / cargo 等工具自动识别；不向命令行拼接任何动态参数
    export http_proxy="$PROXY" https_proxy="$PROXY" all_proxy="$PROXY"
fi
NEED_APT_UPDATE=1

log()  { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*"; }
skip() { printf '[SKIP] %s\n' "$*"; }

pkg_installed() { dpkg-query -W -f='${Status}' "$1" 2>/dev/null | grep -q 'install ok installed'; }

ensure_apt_update() {
    if [ "$NEED_APT_UPDATE" -eq 1 ]; then
        log "apt-get update ..."
        # Lock::Timeout：等待 apt 锁最多 10 分钟（防僵尸 apt 进程卡死；apt ≥2.x 支持）
        apt-get -o DPkg::Lock::Timeout=600 update -qq
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
            apt-get -o DPkg::Lock::Timeout=600 install -y \
                -o Dpkg::Options::=--force-confdef -o Dpkg::Options::=--force-confold "$p"
        fi
    done
}

# 以真实用户身份运行登录 shell（加载 ~/.profile：PATH 与 Rust 环境变量）；
# 代理与 Rust 路径经 env 命令以 argv 形式传入子进程，不拼接 shell 字符串。
as_user() {
    if [ -n "$PROXY" ]; then
        sudo -u "$REAL_USER" -H env RUSTUP_HOME="$RUSTUP_HOME_DIR" CARGO_HOME="$CARGO_HOME_DIR" \
            http_proxy="$PROXY" https_proxy="$PROXY" all_proxy="$PROXY" bash -lc "$*"
    else
        sudo -u "$REAL_USER" -H env RUSTUP_HOME="$RUSTUP_HOME_DIR" CARGO_HOME="$CARGO_HOME_DIR" bash -lc "$*"
    fi
}

# 磁盘余量检查：check_disk <挂载点> <最低可用MB>
check_disk() {
    local mnt min_mb avail_kb
    mnt="$1"; min_mb="$2"
    avail_kb="$(df -Pk "$mnt" | awk 'NR==2{print $4}')"
    if [ "$avail_kb" -lt $((min_mb * 1024)) ]; then
        log "错误: $mnt 可用空间不足 ${min_mb}M（当前 $(awk "BEGIN{printf \"%.1f\", $avail_kb/1048576}")G），先清理再执行"
        exit 1
    fi
    log "$mnt 余量 $(awk "BEGIN{printf \"%.1f\", $avail_kb/1048576}")G，继续"
}

# ---------- 段: base_deps（检查时全部已满足，预期 SKIP） ----------
seg_base_deps() {
    log "== base_deps: git >=2.40 / build-essential / pkg-config / openssl / curl =="
    apt_install git build-essential pkg-config libssl-dev openssl curl ca-certificates
}

# ---------- 段: rust（检查时无 rustup；安装到数据盘 RUST_HOME；minimal profile 省盘） ----------
# 权限设计（针对 /data 数据盘）：
#   - /data 本身保持 root:root 所有，本脚本不改动其权限（最小影响面）；
#   - 仅 $RUST_HOME 子树授予真实用户：属主/属组改为 $REAL_USER 及其主组，目录 0755、
#     安装产物沿用用户 umask（Ubuntu 默认 022 → 其他 sudo 组用户可读可执行=可直接使用
#     该工具链，但写入/安装仅属主可做）；
#   - 安装完成后做功能验证：以 $REAL_USER 在 CARGO_HOME 内建/删测试文件 + rustc --version。
seg_rust() {
    log "== rust: rustup + stable（minimal profile，安装到 $RUST_HOME）=="
    check_disk "$(dirname "$RUST_HOME")" 3072
    if [ -x "$CARGO_BIN/rustc" ]; then
        skip "rustc $("$CARGO_BIN/rustc" --version | awk '{print $2}') 已安装"
        return 0
    fi
    local primary_group
    primary_group="$(id -gn "$REAL_USER")"
    mkdir -p "$RUST_HOME"
    chown "$REAL_USER:$primary_group" "$RUST_HOME"
    chmod 0755 "$RUST_HOME"
    log "权限: $RUST_HOME -> $REAL_USER:$primary_group (0755)；/data 本身保持 root 所有不动"
    log "下载官方 rustup-init 二进制（带 sha256 校验后执行）..."
    local triple tmp
    triple="$(uname -m)-unknown-linux-gnu"
    tmp="$(mktemp -d)"
    # mktemp -d 默认 root:root 0700 —— 必须把目录属主一并交给真实用户，
    # 否则 sudo -u sf 无法穿越该目录执行 rustup-init（Permission denied）
    chown "$REAL_USER:$primary_group" "$tmp"
    chmod 0700 "$tmp"
    curl -fsSL -o "$tmp/rustup-init" "https://static.rust-lang.org/rustup/dist/$triple/rustup-init"
    curl -fsSL -o "$tmp/rustup-init.sha256" "https://static.rust-lang.org/rustup/dist/$triple/rustup-init.sha256"
    if ! ( cd "$tmp" && sha256sum -c rustup-init.sha256 ); then
        rm -rf "$tmp"
        log "错误: sha256 校验未通过，中止（可到 https://rustup.rs 手动安装）"
        return 1
    fi
    chown "$REAL_USER" "$tmp/rustup-init"
    chmod 0755 "$tmp/rustup-init"
    if ! sudo -u "$REAL_USER" -H env RUSTUP_HOME="$RUSTUP_HOME_DIR" CARGO_HOME="$CARGO_HOME_DIR" \
            "$tmp/rustup-init" -y --default-toolchain stable --profile minimal; then
        rm -rf "$tmp"
        log "错误: rustup-init 执行失败"
        return 1
    fi
    rm -rf "$tmp"
    # 持久化环境变量到用户 profile（幂等：已有则跳过）
    local prof="$REAL_HOME/.profile"
    if ! grep -q 'RUSTUP_HOME=' "$prof" 2>/dev/null; then
        cat >> "$prof" << PROFILE_EOF

# Rust toolchain on data disk (added by juzdata_setup.sh)
export RUSTUP_HOME="$RUSTUP_HOME_DIR"
export CARGO_HOME="$CARGO_HOME_DIR"
export PATH="$CARGO_BIN:\$PATH"
PROFILE_EOF
        chown "$REAL_USER" "$prof"
        log "已写入 $prof（RUSTUP_HOME/CARGO_HOME/PATH）"
    fi
    # 归属权收尾：rustup-init 以 $REAL_USER 运行，产物本就属其所有；此处统一校正一次
    # （覆盖任何以 root 意外创建的文件），并确保其他 sudo 组用户可读可执行。
    chown -R "$REAL_USER:$primary_group" "$RUST_HOME"
    find "$RUST_HOME" -type d -exec chmod 0755 {} +
    # 功能验证：属主可在 CARGO_HOME 内读写；rustc 经 profile 环境变量可用
    if ! as_user "touch '$CARGO_HOME_DIR/.perm_test' && rm '$CARGO_HOME_DIR/.perm_test' && rustc --version"; then
        log "错误: 权限验证失败（$REAL_USER 无法读写 $CARGO_HOME_DIR 或 rustc 不可用）"
        return 1
    fi
    log "权限验证通过：$REAL_USER 可读写，其他用户可读可执行（0755 目录）"
    log "rustup 安装完成（$(as_user 'rustc --version')，位于 $RUST_HOME）"
}

# ---------- 段: keepassxc（apt 小包，留系统盘） ----------
seg_keepassxc() {
    log "== keepassxc: >=2.7（24.04 候选 2.7.6，apt 小包留系统盘）=="
    check_disk / 2048
    apt_install keepassxc
    log "keepassxc-cli: $(keepassxc-cli --version 2>&1 | head -1)"
}

# ---------- 段: age + minisign ----------
seg_age_minisign() {
    log "== age + minisign（24.04 候选 1.1.1 / 0.11）=="
    apt_install age minisign
    log "age: $(age --version 2>&1); minisign: $(minisign -v 2>&1)"
}

# ---------- 段: cargo 工具（轻量：binstall/audit/deny） ----------
cargo_tool() { # cargo_tool <可执行名> <crate名>
    local bin="$1" crate="$2"
    if [ -x "$CARGO_BIN/$bin" ]; then
        skip "已安装: $bin"
        return 0
    fi
    log "安装: $crate"
    if [ -x "$CARGO_BIN/cargo-binstall" ]; then
        as_user "cargo binstall -y $crate" || as_user "cargo install --locked $crate"
    else
        as_user "cargo install --locked $crate"
    fi
}

seg_cargo_tools_light() {
    log "== cargo_tools_light: binstall / audit / deny（预编译，装在 $CARGO_HOME_DIR）=="
    [ -x "$CARGO_BIN/cargo" ] || { echo "错误: 未检测到 $CARGO_BIN/cargo，请先执行: sudo bash $0 rust" >&2; exit 1; }
    check_disk "$RUST_HOME" 1024
    # binstall 自身无预编译时从源码装一次（此后其余工具走预编译）
    if [ ! -x "$CARGO_BIN/cargo-binstall" ]; then
        log "源码安装 cargo-binstall（一次性，2 核约 8-15 分钟）..."
        as_user "cargo install --locked cargo-binstall"
    else
        skip "已安装: cargo-binstall"
    fi
    cargo_tool cargo-audit cargo-audit
    cargo_tool cargo-deny  cargo-deny
}

# ---------- 段: cargo 工具（重量，默认不装） ----------
seg_cargo_tools_full() {
    log "== cargo_tools_full: nightly + cargo-fuzz + sccache（约 1.5G+，装在 $RUST_HOME）=="
    check_disk "$RUST_HOME" 2048
    if as_user 'rustup toolchain list' 2>/dev/null | grep -q nightly; then
        skip "nightly 工具链已安装"
    else
        as_user "rustup toolchain install nightly --profile minimal"
    fi
    cargo_tool cargo-fuzz cargo-fuzz
    cargo_tool sccache    sccache
}

# ---------- 段: docker 验证（已装 29.8.2 + compose v5.6.0，data-root 已在 /data/docker，仅验证） ----------
seg_docker_check() {
    log "== docker_check: 已装，仅验证 =="
    skip "docker $(docker --version 2>/dev/null | awk '{print $3}' | tr -d ,) 已安装"
    skip "$(docker compose version 2>/dev/null | head -1) 已安装"
    skip "data-root: $(docker info 2>/dev/null | awk '/Docker Root Dir/{print $NF}')（已在数据盘，无需改动）"
    if systemctl is-active --quiet docker; then
        skip "docker 服务 active"
    else
        log "docker 服务未运行，启用..."
        systemctl enable --now docker
    fi
    if id -nG "$REAL_USER" | tr ' ' '\n' | grep -qx docker; then
        skip "$REAL_USER 已在 docker 组"
    else
        log "将 $REAL_USER 加入 docker 组（重新登录后生效）..."
        usermod -aG docker "$REAL_USER"
    fi
}

# ---------- 段: 可选项（apt 小包，留系统盘） ----------
seg_optional() {
    log "== optional: nodejs(18.19，低于基线 ≥20，仅前端构建验证用) / rclone（apt 小包留系统盘）=="
    check_disk / 1024
    apt_install nodejs npm rclone
    log "node: $(node --version 2>&1)（注意：24.04 源内最高 18，若需 ≥20 需 NodeSource 源，另行决策）"
}

main() {
    local seg="${1:-}"
    if [ "$(id -u)" -ne 0 ]; then
        echo "错误: 请以 sudo 执行（目标用户由 SUDO_USER 决定，当前缺省 $REAL_USER）" >&2
        exit 1
    fi
    if [ -z "$seg" ]; then
        echo "用法: sudo bash $0 [base_deps|rust|keepassxc|age_minisign|cargo_tools_light|cargo_tools_full|docker_check|optional|all]" >&2
        exit 1
    fi
    case "$seg" in
        base_deps)         seg_base_deps ;;
        rust)              seg_rust ;;
        keepassxc)         seg_keepassxc ;;
        age_minisign)      seg_age_minisign ;;
        cargo_tools_light) seg_cargo_tools_light ;;
        cargo_tools_full)  seg_cargo_tools_full ;;
        docker_check)      seg_docker_check ;;
        optional)          seg_optional ;;
        all)
            seg_base_deps
            seg_rust
            seg_keepassxc
            seg_age_minisign
            seg_cargo_tools_light
            seg_docker_check
            seg_optional
            log "全部默认段完成。cargo_tools_full（nightly/fuzz/sccache）默认未装，按需单独执行。"
            ;;
        *)
            echo "错误: 未知段名 '$seg'" >&2
            echo "用法: sudo bash $0 [base_deps|rust|keepassxc|age_minisign|cargo_tools_light|cargo_tools_full|docker_check|optional|all]" >&2
            exit 1
            ;;
    esac
    log "done: $seg"
}

main "$@"
