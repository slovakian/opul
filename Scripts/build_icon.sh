#!/usr/bin/env bash
# Build a macOS .icns from a square PNG master (assets/AppIcon.png).
# The master supplies the silhouette: macOS does not remove a baked-in background.
# Regenerate both Tauri icon assets from the same transparent master.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
SRC=${1:-$ROOT/assets/AppIcon.png}
OUT=${2:-$ROOT/src-tauri/icons/icon.icns}

if [[ ! -f "$SRC" ]]; then
  echo "ERROR: missing app icon source: $SRC" >&2
  exit 1
fi

width=$(sips -g pixelWidth "$SRC" | awk 'END { print $2 }')
height=$(sips -g pixelHeight "$SRC" | awk 'END { print $2 }')
if [[ "$width" != "$height" ]]; then
  echo "ERROR: app icon must be square (got ${width}x${height})" >&2
  exit 1
fi
if [[ "$width" -lt 1024 ]]; then
  echo "ERROR: app icon must be at least 1024x1024 (got ${width})" >&2
  exit 1
fi
if [[ "$(sips -g hasAlpha "$SRC" | awk 'END { print $2 }')" != "yes" ]]; then
  echo "ERROR: app icon must preserve its transparent background" >&2
  exit 1
fi

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT
iconset="$workdir/AppIcon.iconset"
mkdir -p "$iconset"

# iconutil requires both names even when the pixel size matches
# (icon_16x16@2x and icon_32x32 are both 32px).
write_size() {
  local px="$1"
  local name="$2"
  local dest="$iconset/$name"
  sips -z "$px" "$px" "$SRC" --out "$dest" >/dev/null
  local got
  got=$(sips -g pixelWidth "$dest" | awk 'END { print $2 }')
  if [[ "$got" != "$px" ]]; then
    echo "ERROR: $name is ${got}px, expected ${px}" >&2
    exit 1
  fi
}

write_size 16   icon_16x16.png
write_size 32   icon_16x16@2x.png
write_size 32   icon_32x32.png
write_size 64   icon_32x32@2x.png
write_size 128  icon_128x128.png
write_size 256  icon_128x128@2x.png
write_size 256  icon_256x256.png
write_size 512  icon_256x256@2x.png
write_size 512  icon_512x512.png
write_size 1024 icon_512x512@2x.png

mkdir -p "$(dirname "$OUT")"
if ! iconutil --convert icns --output "$OUT" "$iconset" 2>/dev/null; then
  # iconutil on some macOS 26 releases rejects otherwise valid iconsets,
  # including iconsets it just extracted. Keep the release build reproducible
  # by packing the same PNG-backed ICNS entries directly.
  rm -f "$OUT"
  /usr/bin/perl "$ROOT/Scripts/pack_icns.pl" "$iconset" "$OUT"
fi
sips -z 512 512 "$SRC" --out "$(dirname "$OUT")/icon.png" >/dev/null
echo "Wrote $OUT"
