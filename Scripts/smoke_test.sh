#!/usr/bin/env bash
# Isolate the native launch from the user's library and download folder.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="${1:-$ROOT_DIR/Opul.app}"
[[ -x "$APP/Contents/MacOS/Opul" ]] || { echo "Package Opul first." >&2; exit 1; }
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
  "$APP/Contents/MacOS/Opul" >"$SMOKE_DIR/launch.log" 2>&1 &
SMOKE_PID=$!
for _ in {1..40}; do
  kill -0 "$SMOKE_PID" 2>/dev/null || { cat "$SMOKE_DIR/launch.log"; exit 1; }
  if [[ -f "$SMOKE_DIR/frontend-ready" ]]; then
    echo "Opul's packaged frontend rendered and connected to Rust."
    echo "Smoke artifacts: $SMOKE_DIR"
    exit 0
  fi
  sleep 0.5
done
echo "The packaged frontend did not become ready. Logs: $SMOKE_DIR/launch.log" >&2
exit 1
