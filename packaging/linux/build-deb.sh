#!/usr/bin/env bash
# 构建 Linux .deb 安装包(不依赖 cargo-deb,直接用 dpkg-deb)
# 前置条件(见 release.yml):
#   - target/release/MikanPlus 已构建
#   - assets/ 存在于仓库根目录
# 产物: mikanplus_<VERSION>_amd64.deb
set -euo pipefail

VERSION="${VERSION:-0.1.0}"
ARCH="${ARCH:-amd64}"
PKG="mikanplus_${VERSION}_${ARCH}.deb"
STAGE="deb-stage"
LIBDIR="usr/lib/mikanplus"

rm -rf "$STAGE" "$PKG"
mkdir -p "$STAGE/DEBIAN"
mkdir -p "$STAGE/$LIBDIR"
mkdir -p "$STAGE/usr/share/applications"

# 二进制 + 资源(并排安装;代码通过可执行文件位置找到 assets)
cp target/release/MikanPlus "$STAGE/$LIBDIR/"
cp -R assets "$STAGE/$LIBDIR/assets"

# 图标(mikan-pic.png → 标准 256x256 尺寸;需要 ImageMagick,CI 中已安装)
# 注意:Ubuntu 22.04 的 imagemagick 是 6.x,只有 convert 命令(magick 是 7.x)
mkdir -p "$STAGE/usr/share/icons/hicolor/256x256/apps"
convert assets/mikan-pic.png -resize 256x256 "$STAGE/usr/share/icons/hicolor/256x256/apps/mikanplus.png"

# .desktop 入口(与 AppImage 共用一份源文件,deb 安装到系统后用绝对路径)
sed 's|^Exec=.*|Exec=/usr/lib/mikanplus/MikanPlus|' \
  packaging/linux/mikanplus.desktop > "$STAGE/usr/share/applications/mikanplus.desktop"

# control 文件
cat > "$STAGE/DEBIAN/control" <<EOF
Package: mikanplus
Version: ${VERSION}
Section: video
Priority: optional
Architecture: ${ARCH}
Maintainer: TY0909 <ty0909@users.noreply.github.com>
Depends: libxkbcommon0, libxkbcommon-x11-0, libgl1, libegl1, libfontconfig1, libwayland-client0, libxcb1, libx11-6
Description: Desktop anime tracker (GPUI) - Mikan Project client
 A desktop anime tracking application built with GPUI,
 with built-in BitTorrent download support.
EOF

dpkg-deb --root-owner-group --build "$STAGE" "$PKG"

echo "done: $PKG"
