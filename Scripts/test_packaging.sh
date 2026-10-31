#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "Testing packaging configuration and script interfaces..."

# 1. Verify minimumSystemVersion in tauri.conf.json
MIN_SYS_VER=$(node -e 'const fs = require("fs"); const conf = JSON.parse(fs.readFileSync("./src-tauri/tauri.conf.json", "utf8")); console.log(conf?.bundle?.macOS?.minimumSystemVersion ?? "");')
if [[ "$MIN_SYS_VER" != "13.0" ]]; then
  echo "FAIL: Expected minimumSystemVersion 13.0, got '$MIN_SYS_VER'" >&2
  exit 1
fi
echo "PASS: tauri.conf.json specifies minimumSystemVersion: 13.0"

# 2. Verify --help output and target parameters support
Scripts/package_app.sh --help | grep -q "aarch64-apple-darwin"
Scripts/package_dmg.sh --help | grep -q "x86_64-apple-darwin"
echo "PASS: Packaging scripts document target flags"

# 3. Verify rejection of invalid parameters
if Scripts/package_app.sh --invalid-flag 2>/dev/null; then
  echo "FAIL: package_app.sh should fail on invalid flag" >&2
  exit 1
fi
if Scripts/package_dmg.sh --invalid-flag 2>/dev/null; then
  echo "FAIL: package_dmg.sh should fail on invalid flag" >&2
  exit 1
fi
echo "PASS: Packaging scripts reject invalid options"

# 4. Verify existing built single-arch DMGs if available
for ARCH_SUFFIX in arm64 x64; do
  DMG="$ROOT_DIR/Opul-${ARCH_SUFFIX}.dmg"
  if [[ -f "$DMG" ]]; then
    SIZE_MB=$(du -m "$DMG" | cut -f1)
    if [[ $SIZE_MB -gt 35 ]]; then
      echo "FAIL: $DMG size is ${SIZE_MB}MB, expected around ~25MB or less" >&2
      exit 1
    fi
    echo "PASS: $DMG size is ${SIZE_MB}MB (under threshold)"

    MOUNT_DIR=$(mktemp -d "${TMPDIR:-/tmp}/test-dmg.XXXXXX")
    cleanup_mount() {
      hdiutil detach "$MOUNT_DIR" -force >/dev/null 2>&1 || true
      rm -rf "$MOUNT_DIR"
    }
    trap cleanup_mount EXIT

    hdiutil attach -nobrowse -readonly -mountpoint "$MOUNT_DIR" "$DMG" >/dev/null
    codesign --verify --deep --strict "$MOUNT_DIR/Opul.app"
    LIPO_ARCH=$(lipo -archs "$MOUNT_DIR/Opul.app/Contents/MacOS/Opul" | tr ' ' '\n' | sort | xargs)
    
    EXPECTED=""
    if [[ "$ARCH_SUFFIX" == "arm64" ]]; then EXPECTED="arm64"; else EXPECTED="x86_64"; fi
    if [[ "$LIPO_ARCH" != "$EXPECTED" ]]; then
      echo "FAIL: $DMG contains arch $LIPO_ARCH, expected $EXPECTED" >&2
      exit 1
    fi
    echo "PASS: $DMG verified with clean codesign and lipo arch $LIPO_ARCH"

    hdiutil detach "$MOUNT_DIR" -force >/dev/null
    rm -rf "$MOUNT_DIR"
    trap - EXIT
  fi
done

echo "All packaging tests passed successfully."
