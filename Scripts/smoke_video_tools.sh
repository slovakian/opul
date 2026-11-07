#!/usr/bin/env bash
# Explicitly consent to Setup in a packaged app, isolated from the person's data.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="${1:-$ROOT_DIR/Opul.app}"
APP_NAME="$(basename "$APP" .app)"
EXEC="$APP/Contents/MacOS/$APP_NAME"
[[ -x "$EXEC" ]] || { echo "Package Opul first: $EXEC" >&2; exit 1; }
SMOKE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/opul-video-smoke.XXXXXX")"
cleanup() {
  if [[ -n "${SMOKE_PID:-}" ]]; then
    kill "$SMOKE_PID" 2>/dev/null || true
    wait "$SMOKE_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT
# The W3C sample is Blender's openly licensed Big Buck Bunny trailer.
env -u OPUL_YT_DLP OPUL_DATA_DIR="$SMOKE_DIR/state" OPUL_DOWNLOAD_DIR="$SMOKE_DIR/downloads" \
  OPUL_SMOKE_READY_FILE="$SMOKE_DIR/frontend-ready" \
  OPUL_SMOKE_VIDEO_URL="https://media.w3.org/2010/05/bunny/trailer.mp4" \
  OPUL_SMOKE_VIDEO_REPORT="$SMOKE_DIR/video-report.json" \
  "$EXEC" >"$SMOKE_DIR/launch.log" 2>&1 &
SMOKE_PID=$!
for _ in {1..660}; do
  kill -0 "$SMOKE_PID" 2>/dev/null || { cat "$SMOKE_DIR/launch.log"; exit 1; }
  if [[ -f "$SMOKE_DIR/video-report.json" && -f "$SMOKE_DIR/frontend-ready" ]]; then
    echo "Smoke artifacts: $SMOKE_DIR"
    python3 - "$SMOKE_DIR/video-report.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
print(json.dumps(report, indent=2))
raise SystemExit(0 if report['ok'] else 1)
PY
    /usr/bin/tmutil isexcluded "$SMOKE_DIR/state/Tools"
    exit 0
  fi
  sleep 0.5
done
echo "Packaged video smoke timed out. Logs: $SMOKE_DIR/launch.log" >&2
exit 1
