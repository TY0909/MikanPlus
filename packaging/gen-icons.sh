#!/usr/bin/env bash
# 从 assets/mikan-pic.png 重新生成 Windows 图标 assets/mikan_icon.ico。
# 更换应用图标时:替换 mikan-pic.png → 运行本脚本 → 提交。
# macOS 的 .icns 与 Linux 图标在发布流水线中自动从 png 生成,无需手动处理。
set -euo pipefail

SRC="assets/mikan-pic.png"
OUT="assets/mikan_icon.ico"

command -v magick >/dev/null 2>&1 || { echo "需要 ImageMagick (magick)"; exit 1; }

magick "$SRC" -define icon:auto-resize=256,128,64,48,32,16 "$OUT"
echo "done: $OUT"
