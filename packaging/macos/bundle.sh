#!/usr/bin/env bash
# 从 arm64 二进制构建 macOS .app 并打包 .dmg(仅 Apple Silicon M 系列)
# 前置条件(见 release.yml):
#   - target/aarch64-apple-darwin/release/MikanPlus 已构建
#   - assets/ 存在于仓库根目录
# 产物: MikanPlus-<VERSION>-macos-arm64.dmg
set -euo pipefail

VERSION="${VERSION:-0.2.0}"
APP="MikanPlus.app"
DMG="MikanPlus-${VERSION}-macos-arm64.dmg"

rm -rf "$APP" dmg-staging "$DMG"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# 可执行文件
cp target/aarch64-apple-darwin/release/MikanPlus "$APP/Contents/MacOS/MikanPlus"

# 资源目录(代码通过可执行文件相对路径找到它)
cp -R assets "$APP/Contents/Resources/assets"

# 应用图标:mikan-pic.png(400x400)→ 放大到 512 标准尺寸 → icns
sips -z 512 512 assets/mikan-pic.png --out icon-512.png >/dev/null
sips -s format icns icon-512.png --out "$APP/Contents/Resources/icon.icns" >/dev/null
rm -f icon-512.png

# Info.plist
cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>MikanPlus</string>
    <key>CFBundleDisplayName</key><string>MikanPlus</string>
    <key>CFBundleExecutable</key><string>MikanPlus</string>
    <key>CFBundleIdentifier</key><string>io.github.ty0909.mikanplus</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleIconFile</key><string>icon.icns</string>
    <key>LSMinimumSystemVersion</key><string>10.15</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSApplicationCategoryType</key><string>public.app-category.entertainment</string>
    <key>NSHumanReadableCopyright</key><string>Apache-2.0</string>
</dict>
</plist>
EOF

# Apple Silicon 上未签名的二进制无法直接运行;ad-hoc 签名即可绕过
codesign --force --deep --sign - "$APP"

# 打包 dmg(内含 /Applications 快捷方式)
mkdir -p dmg-staging
cp -R "$APP" dmg-staging/
ln -s /Applications dmg-staging/Applications
hdiutil create -volname "MikanPlus" -srcfolder dmg-staging -ov -format UDZO "$DMG"

echo "done: $DMG"
