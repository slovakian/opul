#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
BUILD_CONF="${1:-release}"
ARGS=(build --bundles app)
case "$BUILD_CONF" in
  debug) ARGS+=(--debug) ;;
  release) ;;
  *) echo "Usage: Scripts/package_app.sh [debug|release]" >&2; exit 1 ;;
esac
TARGET_DIR="$ROOT_DIR/target"
if [[ "${ARCHES:-}" == "arm64 x86_64" ]]; then
  ARGS+=(--target universal-apple-darwin)
  TARGET_DIR="$TARGET_DIR/universal-apple-darwin"
fi
if [[ -n "${MARKETING_VERSION:-}" ]]; then
  ARGS+=(--config "{\"version\":\"$MARKETING_VERSION\"}")
fi
npm run tauri -- "${ARGS[@]}"
ditto "$TARGET_DIR/$BUILD_CONF/bundle/macos/Opul.app" "$ROOT_DIR/Opul.app"
echo "Built $ROOT_DIR/Opul.app"
