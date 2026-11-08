#!/usr/bin/env bash
set -euo pipefail
TOOLS=${1:?Usage: scripts/verify_ffmpeg.sh tools-directory arm64|x64}
case "${2:?architecture required}" in
  arm64) CPU=arm64 ;;
  x64) CPU=x86_64 ;;
  *) exit 1 ;;
esac
(cd "$TOOLS" && shasum -a 256 -c SHA256SUMS)
for tool in ffmpeg ffprobe; do
  test -x "$TOOLS/$tool"
  test "$(lipo -archs "$TOOLS/$tool")" = "$CPU"
  # A build with a newer deployment target would exclude supported macOS 13 Macs.
  xcrun vtool -show-build "$TOOLS/$tool" | awk '
    $1 == "minos" { found = 1; if ($2 != "13.0") exit 1 }
    END { if (!found) exit 1 }'
  if otool -L "$TOOLS/$tool" | tail -n +2 | awk '{print $1}' | \
    grep -Ev '^(/usr/lib/|/System/Library/)' ; then
    echo "Unexpected dynamic dependency in $tool" >&2
    exit 1
  fi
  codesign --verify --strict "$TOOLS/$tool"
done
test -s "$TOOLS/FFmpeg-LICENSE.txt"
test -s "$TOOLS/LAME-LICENSE.txt"
test -s "$TOOLS/SOURCE.txt"
