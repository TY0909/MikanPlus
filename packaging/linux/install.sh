#!/usr/bin/env bash
# MikanPlus Linux 安装器。
#
# 默认使用用户级 AppImage 安装，不需要 root:
#   curl -fsSL https://github.com/TY0909/MikanPlus/releases/latest/download/MikanPlus-linux-install.sh | bash
#
# 系统级安装:
#   curl -fsSL https://github.com/TY0909/MikanPlus/releases/latest/download/MikanPlus-linux-install.sh \
#     | sudo bash -s -- --system
#
# 卸载:
#   mikanplus --uninstall
#   mikanplus --uninstall --purge
set -euo pipefail

readonly APP_NAME="MikanPlus"
readonly COMMAND_NAME="mikanplus"
readonly REPO="TY0909/MikanPlus"

MODE="user"
VERSION="${MIKANPLUS_VERSION:-}"
INSTALL_DIR=""
BIN_DIR=""
UNINSTALL=0
PURGE=0
REMOVE_DOWNLOADS=0
YES=0
TMP_DIR=""

trap 'if [[ -n "${TMP_DIR:-}" && -d "$TMP_DIR" ]]; then rm -rf -- "$TMP_DIR"; fi' EXIT

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    C_RESET=$'\033[0m'
    C_INFO=$'\033[36m'
    C_WARN=$'\033[33m'
    C_ERR=$'\033[31m'
    C_OK=$'\033[32m'
else
    C_RESET=""
    C_INFO=""
    C_WARN=""
    C_ERR=""
    C_OK=""
fi

info() { printf '%s==>%s %s\n' "$C_INFO" "$C_RESET" "$*"; }
ok() { printf '%s==>%s %s\n' "$C_OK" "$C_RESET" "$*"; }
warn() { printf '%swarn:%s %s\n' "$C_WARN" "$C_RESET" "$*" >&2; }
die() { printf '%serror:%s %s\n' "$C_ERR" "$C_RESET" "$*" >&2; exit 1; }

usage() {
    cat <<'EOF'
MikanPlus Linux 安装器

用法:
  bash install.sh [选项]

默认安装到当前用户的 ~/.local/opt/mikanplus，不需要 root。

选项:
  --system                    系统级安装到 /opt/mikanplus，需要 root
  --user                      用户级安装(默认)
  --version <x.y.z>           安装指定版本(默认取最新 Release)
  --dir <path>                自定义 AppImage 安装目录
  --bin-dir <path>            自定义 mikanplus 命令目录
  --uninstall                 卸载已安装的程序
  --purge                     卸载时同时删除配置和缓存
  --remove-downloads          卸载时删除指定的下载目录
  --download-dir <path>       配合 --remove-downloads 明确指定下载目录
  --yes                       跳过卸载确认
  -h, --help                  显示帮助

环境变量:
  MIKANPLUS_VERSION           同 --version
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --system) MODE="system"; shift ;;
        --user) MODE="user"; shift ;;
        --version)
            [[ $# -ge 2 ]] || die "--version 需要版本号"
            VERSION="$2"
            shift 2
            ;;
        --dir)
            [[ $# -ge 2 ]] || die "--dir 需要路径"
            INSTALL_DIR="$2"
            shift 2
            ;;
        --bin-dir)
            [[ $# -ge 2 ]] || die "--bin-dir 需要路径"
            BIN_DIR="$2"
            shift 2
            ;;
        --uninstall) UNINSTALL=1; shift ;;
        --purge) PURGE=1; shift ;;
        --remove-downloads) REMOVE_DOWNLOADS=1; shift ;;
        --download-dir)
            [[ $# -ge 2 ]] || die "--download-dir 需要路径"
            DOWNLOAD_DIR="$2"
            shift 2
            ;;
        --yes) YES=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) die "未知参数: $1 (使用 --help 查看帮助)" ;;
    esac
done

DOWNLOAD_DIR="${DOWNLOAD_DIR:-}"

if [[ "$MODE" == "system" ]]; then
    [[ "$(id -u)" -eq 0 ]] || die "系统级安装需要 root，请使用 sudo bash -s -- --system"
    INSTALL_DIR="${INSTALL_DIR:-/opt/mikanplus}"
    BIN_DIR="${BIN_DIR:-/usr/local/bin}"
    DATA_HOME=""
    CACHE_HOME=""
    DESKTOP_DIR="/usr/share/applications"
    ICON_ROOT="/usr/share/icons/hicolor"
else
    [[ "$(id -u)" -ne 0 ]] || die "用户级安装不应使用 root；如需系统级安装请加 --system"
    INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/opt/mikanplus}"
    BIN_DIR="${BIN_DIR:-${XDG_BIN_HOME:-$HOME/.local/bin}}"
    DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
    CACHE_HOME="${XDG_CACHE_HOME:-$HOME/.cache}"
    DESKTOP_DIR="$DATA_HOME/applications"
    ICON_ROOT="$DATA_HOME/icons/hicolor"
fi

DESKTOP_FILE="$DESKTOP_DIR/mikanplus.desktop"
ICON_DIR="$ICON_ROOT/256x256/apps"
ICON_FILE="$ICON_DIR/mikanplus.png"
BIN_LINK="$BIN_DIR/$COMMAND_NAME"
APPIMAGE="$INSTALL_DIR/$APP_NAME.AppImage"
UNINSTALLER="$INSTALL_DIR/uninstall.sh"
DATA_DIR="${DATA_HOME:+$DATA_HOME/mikanplus}"
CACHE_DIR="${CACHE_HOME:+$CACHE_HOME/mikanplus}"

# Refuse broad paths before any recursive operation. The install directory may be /opt/mikanplus,
# but its parent /opt and other filesystem roots are never valid targets.
guard_path() {
    local path=$1
    case "/$path/" in
        */../*) die "路径不能包含 ..: $path" ;;
    esac
    case "$path" in
        ""|/|/bin|/boot|/dev|/etc|/home|/lib|/lib64|/opt|/proc|/root|/run|/sbin|/sys|/tmp|/usr|/usr/local|/var)
            die "拒绝操作受保护目录: $path"
            ;;
    esac
    case "$path" in
        /*/*) ;;
        *) die "路径必须是绝对路径且不能是顶层目录: $path" ;;
    esac
}

download() {
    local url=$1
    local destination=$2
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --retry 3 --connect-timeout 15 -o "$destination" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$destination" "$url"
    else
        die "需要 curl 或 wget"
    fi
}

latest_version() {
    local response="$TMP_DIR/latest.json"
    download "https://api.github.com/repos/$REPO/releases/latest" "$response" \
        || die "无法查询最新版本，请使用 --version 指定版本"
    sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\{0,1\}\([^"]*\)".*/\1/p' "$response" | head -n 1
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) ARCH="x86_64" ;;
        aarch64|arm64) die "暂未提供 Linux arm64 安装包" ;;
        *) die "不支持的 Linux 架构: $(uname -m)" ;;
    esac
}

refresh_caches() {
    command -v update-desktop-database >/dev/null 2>&1 && \
        update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 && \
        gtk-update-icon-cache -qtf "$ICON_ROOT" >/dev/null 2>&1 || true
}

write_uninstaller() {
    local destination=$1
    cat > "$destination" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 8 ]]; then
    printf '卸载器参数不完整\n' >&2
    exit 1
fi

INSTALL_DIR=$1
BIN_LINK=$2
DESKTOP_FILE=$3
ICON_FILE=$4
ICON_ROOT=$5
DATA_DIR=$6
CACHE_DIR=$7
DEFAULT_DOWNLOAD_DIR=$8
shift 8

PURGE=0
REMOVE_DOWNLOADS=0
DOWNLOAD_DIR=""
YES=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --purge) PURGE=1; shift ;;
        --remove-downloads) REMOVE_DOWNLOADS=1; shift ;;
        --download-dir)
            [[ $# -ge 2 ]] || { printf '%s\n' '--download-dir 需要路径' >&2; exit 1; }
            DOWNLOAD_DIR=$2
            shift 2
            ;;
        --yes) YES=1; shift ;;
        --help|-h)
            printf '%s\n' \
                '用法: mikanplus --uninstall [--purge] [--remove-downloads --download-dir <path>] [--yes]'
            exit 0
            ;;
        *) printf '未知卸载参数: %s\n' "$1" >&2; exit 1 ;;
    esac
done

guard_path() {
    local path=$1
    case "/$path/" in
        */../*) printf '路径不能包含 ..: %s\n' "$path" >&2; exit 1 ;;
    esac
    case "$path" in
        ""|/|/bin|/boot|/dev|/etc|/home|/lib|/lib64|/opt|/proc|/root|/run|/sbin|/sys|/tmp|/usr|/usr/local|/var)
            printf '拒绝操作受保护目录: %s\n' "$path" >&2
            exit 1
            ;;
    esac
    case "$path" in
        /*/*) ;;
        *) printf '路径必须是绝对路径且不能是顶层目录: %s\n' "$path" >&2; exit 1 ;;
    esac
}

confirm() {
    local prompt=$1
    [[ "$YES" -eq 1 ]] && return 0
    if [[ -r /dev/tty ]]; then
        local answer
        read -r -p "$prompt [y/N] " answer </dev/tty
        [[ "$answer" == "y" || "$answer" == "Y" ]]
    else
        printf '%s\n' '卸载操作需要确认；非交互执行请添加 --yes' >&2
        return 1
    fi
}

[[ -f "$INSTALL_DIR/.mikanplus-install" ]] || {
    printf '未找到 MikanPlus 安装清单: %s\n' "$INSTALL_DIR" >&2
    exit 1
}
guard_path "$INSTALL_DIR"

printf '%s\n' '将移除:'
printf '  - %s\n' "$INSTALL_DIR"
printf '  - %s\n' "$BIN_LINK"
printf '  - %s\n' "$DESKTOP_FILE"
printf '  - %s\n' "$ICON_FILE"

if [[ "$PURGE" -eq 1 ]]; then
    if [[ -n "$DATA_DIR" || -n "$CACHE_DIR" ]]; then
        printf '%s\n' '同时删除:'
        [[ -z "$DATA_DIR" ]] || printf '  - %s\n' "$DATA_DIR"
        [[ -z "$CACHE_DIR" ]] || printf '  - %s\n' "$CACHE_DIR"
    else
        printf '%s\n' '系统级安装不会自动删除任何用户的配置和缓存。'
    fi
fi

if [[ "$REMOVE_DOWNLOADS" -eq 1 ]]; then
    DOWNLOAD_DIR="${DOWNLOAD_DIR:-$DEFAULT_DOWNLOAD_DIR}"
    [[ -n "$DOWNLOAD_DIR" ]] || {
        printf '%s\n' '删除下载文件必须使用 --download-dir 明确指定目录。' >&2
        exit 1
    }
    guard_path "$DOWNLOAD_DIR"
    printf '同时删除下载目录: %s\n' "$DOWNLOAD_DIR"
fi

confirm '确认继续卸载?' || { printf '%s\n' '已取消卸载。'; exit 0; }

if [[ "$REMOVE_DOWNLOADS" -eq 1 && -d "$DOWNLOAD_DIR" ]]; then
    rm -rf -- "$DOWNLOAD_DIR"
fi
if [[ "$PURGE" -eq 1 ]]; then
    [[ -z "$DATA_DIR" ]] || { guard_path "$DATA_DIR"; rm -rf -- "$DATA_DIR"; }
    [[ -z "$CACHE_DIR" ]] || { guard_path "$CACHE_DIR"; rm -rf -- "$CACHE_DIR"; }
fi

rm -f -- "$BIN_LINK" "$DESKTOP_FILE" "$ICON_FILE"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$(dirname "$DESKTOP_FILE")" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -qtf "$ICON_ROOT" >/dev/null 2>&1 || true
fi
rm -rf -- "$INSTALL_DIR"
printf '%s\n' 'MikanPlus 卸载完成。用户数据和下载文件按选择处理。'
EOF
    chmod 755 "$destination"
}

write_launcher() {
    local destination=$1
    local quoted_install quoted_appimage quoted_uninstaller quoted_bin quoted_desktop quoted_icon quoted_root quoted_data quoted_cache quoted_download
    printf -v quoted_install '%q' "$INSTALL_DIR"
    printf -v quoted_appimage '%q' "$APPIMAGE"
    printf -v quoted_uninstaller '%q' "$UNINSTALLER"
    printf -v quoted_bin '%q' "$BIN_LINK"
    printf -v quoted_desktop '%q' "$DESKTOP_FILE"
    printf -v quoted_icon '%q' "$ICON_FILE"
    printf -v quoted_root '%q' "$ICON_ROOT"
    printf -v quoted_data '%q' "$DATA_DIR"
    printf -v quoted_cache '%q' "$CACHE_DIR"
    printf -v quoted_download '%q' "$DOWNLOAD_DIR"
    cat > "$destination" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "--system" ]]; then shift; fi
if [[ "\${1:-}" == "--uninstall" ]]; then
    shift
    if [[ "\$(id -u)" -ne 0 && ! -w $quoted_install ]]; then
        command -v sudo >/dev/null 2>&1 || { printf '%s\n' '卸载系统级安装需要 sudo' >&2; exit 1; }
        exec sudo $quoted_uninstaller $quoted_install $quoted_bin $quoted_desktop $quoted_icon $quoted_root $quoted_data $quoted_cache $quoted_download "\$@"
    fi
    exec $quoted_uninstaller $quoted_install $quoted_bin $quoted_desktop $quoted_icon $quoted_root $quoted_data $quoted_cache $quoted_download "\$@"
fi
exec $quoted_appimage "\$@"
EOF
    chmod 755 "$destination"
}

install_app() {
    guard_path "$INSTALL_DIR"
    detect_arch
    VERSION="${VERSION#v}"
    TMP_DIR=$(mktemp -d)

    if [[ -z "$VERSION" ]]; then
        info "查询最新版本..."
        VERSION=$(latest_version)
        [[ -n "$VERSION" ]] || die "无法解析最新版本，请使用 --version 指定版本"
    fi

    local release_base="https://github.com/$REPO/releases/download/v$VERSION"
    local asset="$APP_NAME-$VERSION-linux-$ARCH.AppImage"
    local archive="$TMP_DIR/$asset"
    local checksums="$TMP_DIR/SHA256SUMS"
    local icon="$TMP_DIR/mikanplus.png"
    local expected actual

    info "下载 $asset"
    download "$release_base/$asset" "$archive" || die "下载失败: $release_base/$asset"
    download "$release_base/SHA256SUMS" "$checksums" || die "无法下载 SHA256SUMS"
    expected=$(awk -v file="$asset" '$2 == file { print $1; exit }' "$checksums")
    [[ "$expected" =~ ^[[:xdigit:]]{64}$ ]] || die "SHA256SUMS 中没有 $asset 的校验值"
    command -v sha256sum >/dev/null 2>&1 || die "需要 sha256sum 才能校验安装包"
    actual=$(sha256sum "$archive" | awk '{print $1}')
    [[ "$actual" == "$expected" ]] || die "AppImage 校验失败，已停止安装"

    if ! download "$release_base/MikanPlus-linux-icon.png" "$icon"; then
        warn "未找到发布图标，将跳过桌面图标安装"
        icon=""
    fi

    local stage="${INSTALL_DIR}.staging.$$"
    local backup="${INSTALL_DIR}.backup.$$"
    [[ ! -e "$stage" && ! -e "$backup" ]] || die "临时安装目录已存在，请清理后重试"
    mkdir -p "$stage"
    cp "$archive" "$stage/$APP_NAME.AppImage"
    chmod 755 "$stage/$APP_NAME.AppImage"
    printf 'mode=%s\nversion=%s\n' "$MODE" "$VERSION" > "$stage/.mikanplus-install"
    write_uninstaller "$stage/uninstall.sh"

    if [[ -e "$INSTALL_DIR" ]]; then
        [[ -f "$INSTALL_DIR/.mikanplus-install" ]] || die "目标目录不是 MikanPlus 安装目录: $INSTALL_DIR"
        mv "$INSTALL_DIR" "$backup"
    fi
    if ! mv "$stage" "$INSTALL_DIR"; then
        [[ -e "$backup" ]] && mv "$backup" "$INSTALL_DIR"
        die "无法切换到新版本"
    fi
    [[ ! -e "$backup" ]] || rm -rf -- "$backup"

    mkdir -p "$BIN_DIR"
    local launcher_tmp="$BIN_DIR/.${COMMAND_NAME}.staging.$$"
    write_launcher "$launcher_tmp"
    mv -f "$launcher_tmp" "$BIN_LINK"

    mkdir -p "$DESKTOP_DIR"
    cat > "$DESKTOP_FILE.tmp.$$" <<EOF
[Desktop Entry]
Type=Application
Name=MikanPlus
Comment=A desktop anime tracker built with GPUI
Comment[zh_CN]=基于 GPUI 的桌面追番应用
Exec=$BIN_LINK
Icon=mikanplus
Terminal=false
Categories=AudioVideo;Video;Network;
EOF
    mv -f "$DESKTOP_FILE.tmp.$$" "$DESKTOP_FILE"

    if [[ -n "$icon" ]]; then
        mkdir -p "$ICON_DIR"
        cp "$icon" "$ICON_FILE"
    fi
    refresh_caches

    ok "MikanPlus $VERSION 安装完成"
    printf '启动命令: %s\n' "$COMMAND_NAME"
    printf '卸载命令: %s --uninstall\n' "$COMMAND_NAME"
    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *) warn "$BIN_DIR 不在 PATH 中，请重新打开终端或将其加入 PATH" ;;
    esac
}

if [[ "$UNINSTALL" -eq 1 ]]; then
    [[ -x "$UNINSTALLER" ]] || die "未找到本地卸载器: $UNINSTALLER"
    args=()
    [[ "$PURGE" -eq 1 ]] && args+=(--purge)
    [[ "$REMOVE_DOWNLOADS" -eq 1 ]] && args+=(--remove-downloads)
    [[ -n "$DOWNLOAD_DIR" ]] && args+=(--download-dir "$DOWNLOAD_DIR")
    [[ "$YES" -eq 1 ]] && args+=(--yes)
    exec "$UNINSTALLER" "$INSTALL_DIR" "$BIN_LINK" "$DESKTOP_FILE" "$ICON_FILE" "$ICON_ROOT" "$DATA_DIR" "$CACHE_DIR" "$DOWNLOAD_DIR" "${args[@]}"
else
    install_app
fi
