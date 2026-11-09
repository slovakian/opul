#!/usr/bin/env bash
# Fetch only the architecture Tauri is building, verified against committed hashes.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
source "$ROOT/tools/ffmpeg/sources.env"
source "$ROOT/tools/ffmpeg/release.env"
case "${1:-${TAURI_ENV_TARGET_TRIPLE:-$(uname -m)}}" in
  arm64|aarch64-apple-darwin) ARCH=arm64; DIGEST="$ARM64_SHA256" ;;
  x64|x86_64|x86_64-apple-darwin) ARCH=x64; DIGEST="$X64_SHA256" ;;
  *) echo 'Built-in tools require a single macOS architecture (arm64 or x64).' >&2; exit 1 ;;
esac
CACHE="$ROOT/.build/ffmpeg-cache"
NAME="$FFMPEG_RELEASE-$ARCH.tar.gz"
mkdir -p "$CACHE" "$ROOT/src-tauri/resources"
if ! shasum -a 256 -c <<<"$DIGEST  $CACHE/$NAME" >/dev/null 2>&1; then
  curl -fsSL --retry 10 --retry-delay 2 --retry-max-time 180 --retry-connrefused --retry-all-errors \
    "https://github.com/slovakian/opul/releases/download/$FFMPEG_RELEASE/$NAME" -o "$CACHE/$NAME.part"
  shasum -a 256 -c <<<"$DIGEST  $CACHE/$NAME.part"
  mv "$CACHE/$NAME.part" "$CACHE/$NAME"
fi
STAGING=$(mktemp -d "$ROOT/src-tauri/resources/.ffmpeg.XXXXXX")
trap 'rm -rf "$STAGING"' EXIT
tar -xzf "$CACHE/$NAME" -C "$STAGING"
"$ROOT/Scripts/verify_ffmpeg.sh" "$STAGING" "$ARCH"
# Tauri copies ordinary Resources without signing their Mach-O executables.
# Verify upstream bytes first, then sign before Tauri seals/notarizes the app.
if [[ -n "${APPLE_SIGNING_IDENTITY:-}" && "$APPLE_SIGNING_IDENTITY" != - ]]; then
  for tool in ffmpeg ffprobe; do
    codesign --force --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$STAGING/$tool"
  done
  echo "$FFMPEG_RELEASE $ARCH SHA-256 $DIGEST" > "$STAGING/ARCHIVE.txt"
  (cd "$STAGING" && shasum -a 256 ffmpeg ffprobe *LICENSE.txt SOURCE.txt configure.txt ARCHIVE.txt > SHA256SUMS)
  "$ROOT/Scripts/verify_ffmpeg.sh" "$STAGING" "$ARCH"
fi
rm -rf "$ROOT/src-tauri/resources/media-tools"
mv "$STAGING" "$ROOT/src-tauri/resources/media-tools"
trap - EXIT
echo "Fetched $FFMPEG_RELEASE ($ARCH), verified SHA-256 $DIGEST"
