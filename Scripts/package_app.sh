#!/usr/bin/env bash
# Build Opul.app via Tauri. Signs ad-hoc unless APP_IDENTITY names a Developer ID certificate.
set -euo pipefail

CONF=${1:-release}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

source "$ROOT/app.env"
APP_IDENTITY=${APP_IDENTITY:-}

if [[ "$CONF" == "debug" ]]; then
  bun run tauri build --debug --bundles app
  SOURCE_APP="$ROOT/target/debug/bundle/macos/${APP_NAME}.app"
else
  bun run tauri build --bundles app
  SOURCE_APP="$ROOT/target/release/bundle/macos/${APP_NAME}.app"
fi

APP="$ROOT/${APP_NAME}.app"
rm -rf "$APP"
cp -R "$SOURCE_APP" "$APP"

if [[ -n "$APP_IDENTITY" ]]; then
  codesign --force --timestamp --options runtime --sign "$APP_IDENTITY" "$APP"
else
  codesign --force --sign "-" "$APP"
fi

echo "Created $APP ($CONF)"
