#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
export MACOSX_DEPLOYMENT_TARGET=13.0
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
BUILD_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
APP_DIR="$ROOT_DIR/.build/rune/Opul Rune.app"
cargo build --locked -p opul-rune --target-dir "$BUILD_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp "$BUILD_DIR/debug/opul-rune" "$APP_DIR/Contents/MacOS/opul-rune"
cp src-rune/resources/rune-resources.json src-rune/LICENSE-Rune src-rune/NOTICE.md "$APP_DIR/Contents/Resources/"
cp assets/AppIcon-macOS.png "$APP_DIR/Contents/Resources/AppIcon.png"
cat > "$APP_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>opul-rune</string>
  <key>CFBundleIdentifier</key><string>com.opul.rune</string>
  <key>CFBundleName</key><string>Opul Rune</string>
  <key>CFBundleDisplayName</key><string>Opul Rune</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSPrincipalClass</key><string>NSApplication</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
</dict></plist>
PLIST
codesign --force --sign - "$APP_DIR"
codesign --verify --deep --strict "$APP_DIR"
printf '%s\n' "$APP_DIR"
