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

# 2. Verify channel definitions and updater configuration
STABLE_ID=$(node -e 'const conf = JSON.parse(require("fs").readFileSync("./src-tauri/tauri.conf.json", "utf8")); console.log(conf.identifier);')
STABLE_NAME=$(node -e 'const conf = JSON.parse(require("fs").readFileSync("./src-tauri/tauri.conf.json", "utf8")); console.log(conf.productName);')
STABLE_UPDATER=$(node -e 'const conf = JSON.parse(require("fs").readFileSync("./src-tauri/tauri.conf.json", "utf8")); console.log(conf?.plugins?.updater?.endpoints?.[0] ?? "");')

if [[ "$STABLE_ID" != "dev.opul.app" ]]; then
  echo "FAIL: Expected stable identifier dev.opul.app, got '$STABLE_ID'" >&2
  exit 1
fi
if [[ "$STABLE_NAME" != "Opul" ]]; then
  echo "FAIL: Expected stable productName Opul, got '$STABLE_NAME'" >&2
  exit 1
fi
if [[ "$STABLE_UPDATER" != "https://github.com/slovakian/opul/releases/latest/download/latest.json" ]]; then
  echo "FAIL: Expected stable updater endpoint pointing to latest.json, got '$STABLE_UPDATER'" >&2
  exit 1
fi
echo "PASS: Stable channel identity (dev.opul.app / Opul) and updater endpoint verified"

NIGHTLY_ID=$(node -e 'const conf = JSON.parse(require("fs").readFileSync("./src-tauri/tauri.nightly.conf.json", "utf8")); console.log(conf.identifier);')
NIGHTLY_NAME=$(node -e 'const conf = JSON.parse(require("fs").readFileSync("./src-tauri/tauri.nightly.conf.json", "utf8")); console.log(conf.productName);')
NIGHTLY_UPDATER=$(node -e 'const conf = JSON.parse(require("fs").readFileSync("./src-tauri/tauri.nightly.conf.json", "utf8")); console.log(conf?.plugins?.updater?.endpoints?.[0] ?? "");')

if [[ "$NIGHTLY_ID" != "dev.opul.app.nightly" ]]; then
  echo "FAIL: Expected nightly identifier dev.opul.app.nightly, got '$NIGHTLY_ID'" >&2
  exit 1
fi
if [[ "$NIGHTLY_NAME" != "Opul Nightly" ]]; then
  echo "FAIL: Expected nightly productName Opul Nightly, got '$NIGHTLY_NAME'" >&2
  exit 1
fi
if [[ "$NIGHTLY_UPDATER" != "https://github.com/slovakian/opul/releases/download/nightly/nightly.json" ]]; then
  echo "FAIL: Expected nightly updater endpoint pointing to nightly.json, got '$NIGHTLY_UPDATER'" >&2
  exit 1
fi
echo "PASS: Nightly channel identity (dev.opul.app.nightly / Opul Nightly) and updater endpoint verified"

# 3. Verify --help output and target/channel parameters support
Scripts/package_app.sh --help | grep -q "aarch64-apple-darwin"
Scripts/package_dmg.sh --help | grep -q "x86_64-apple-darwin"
Scripts/package_app.sh --help | grep -q -- "--channel"
Scripts/package_dmg.sh --help | grep -q -- "--channel"
echo "PASS: Packaging scripts document target and channel flags"

# 4. Verify rejection of invalid parameters and channels
if Scripts/package_app.sh --invalid-flag 2>/dev/null; then
  echo "FAIL: package_app.sh should fail on invalid flag" >&2
  exit 1
fi
if Scripts/package_dmg.sh --invalid-flag 2>/dev/null; then
  echo "FAIL: package_dmg.sh should fail on invalid flag" >&2
  exit 1
fi
if Scripts/package_app.sh --channel invalid-channel 2>/dev/null; then
  echo "FAIL: package_app.sh should fail on invalid channel" >&2
  exit 1
fi
if Scripts/package_dmg.sh --channel invalid-channel 2>/dev/null; then
  echo "FAIL: package_dmg.sh should fail on invalid channel" >&2
  exit 1
fi
echo "PASS: Packaging scripts reject invalid options and channels"

# 5. Verify existing built single-arch DMGs if available
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
