#!/usr/bin/env bash
# Isolate the native launch from the user's library and download folder.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="${1:-}"
if [[ -z "$APP" ]]; then
  if [[ -d "$ROOT_DIR/Opul.app" ]]; then
    APP="$ROOT_DIR/Opul.app"
  elif [[ -d "$ROOT_DIR/Opul Nightly.app" ]]; then
    APP="$ROOT_DIR/Opul Nightly.app"
  else
    APP="$ROOT_DIR/Opul.app"
  fi
fi

APP_NAME="$(basename "$APP" .app)"
EXEC="$APP/Contents/MacOS/$APP_NAME"
[[ -x "$EXEC" ]] || { echo "Package $APP_NAME first ($EXEC not found)." >&2; exit 1; }
swift "$ROOT_DIR/Scripts/check_macos_icon.swift" "$APP"
SMOKE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/opul-smoke.XXXXXX")"
cleanup() {
  if [[ -n "${SMOKE_PID:-}" ]]; then
    kill "$SMOKE_PID" 2>/dev/null || true
    wait "$SMOKE_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT
env OPUL_DATA_DIR="$SMOKE_DIR/state" OPUL_DOWNLOAD_DIR="$SMOKE_DIR/downloads" \
  OPUL_SMOKE_READY_FILE="$SMOKE_DIR/frontend-ready" \
  "$EXEC" >"$SMOKE_DIR/launch.log" 2>&1 &
SMOKE_PID=$!
for _ in {1..40}; do
  kill -0 "$SMOKE_PID" 2>/dev/null || { cat "$SMOKE_DIR/launch.log"; exit 1; }
  if [[ -f "$SMOKE_DIR/frontend-ready" ]]; then
    echo "$APP_NAME's packaged frontend rendered and connected to Rust."
    echo "Smoke artifacts: $SMOKE_DIR"
    exit 0
  fi
  sleep 0.5
done
echo "The packaged frontend did not become ready. Logs: $SMOKE_DIR/launch.log" >&2
exit 1
