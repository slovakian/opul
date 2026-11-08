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
# The W3C sample is Blender's openly licensed Big Buck Bunny trailer, CC BY 3.0:
# https://peach.blender.org/about/
launch() {
env -u OPUL_YT_DLP -u OPUL_SMOKE_VIDEO_RESTART OPUL_DATA_DIR="$SMOKE_DIR/state" OPUL_DOWNLOAD_DIR="$SMOKE_DIR/downloads" \
  OPUL_SMOKE_READY_FILE="$SMOKE_DIR/frontend-ready" \
  OPUL_SMOKE_VIDEO_URL="https://media.w3.org/2010/05/bunny/trailer.mp4" \
  OPUL_SMOKE_VIDEO_REPORT="$SMOKE_DIR/$1-report.json" \
  "${@:2}" \
  "$EXEC" >"$SMOKE_DIR/$1-launch.log" 2>&1 &
SMOKE_PID=$!
for _ in {1..1860}; do
  if [[ -f "$SMOKE_DIR/$1-report.json" && -f "$SMOKE_DIR/frontend-ready" ]]; then
    echo "Smoke artifacts: $SMOKE_DIR"
    python3 - "$SMOKE_DIR/$1-report.json" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
print(json.dumps(report, indent=2))
raise SystemExit(0 if report['ok'] else 1)
PY
    BACKUP_STATUS="$(/usr/bin/tmutil isexcluded "$SMOKE_DIR/state/Tools")"
    printf '%s\n' "$BACKUP_STATUS"
    [[ "$BACKUP_STATUS" == "[Excluded]"* ]] || { echo "Tools was not excluded from Time Machine." >&2; exit 1; }
    return 0
  fi
  kill -0 "$SMOKE_PID" 2>/dev/null || { cat "$SMOKE_DIR/$1-launch.log"; exit 1; }
  sleep 0.5
done
echo "Packaged video smoke timed out. Logs: $SMOKE_DIR/$1-launch.log" >&2
exit 1
}
wait_for_quit() {
  for _ in {1..120}; do
    if ! kill -0 "$SMOKE_PID" 2>/dev/null; then
      wait "$SMOKE_PID"
      SMOKE_PID=""
      python3 - "$SMOKE_DIR/state/application-state.json" <<'PY'
import json, sys
checkpoint = json.load(open(sys.argv[1]))
assert checkpoint['state']['session'] == 'Stopped', 'App did not checkpoint ordinary shutdown'
print('Ordinary window-close shutdown committed Stopped.')
PY
      return 0
    fi
    sleep 0.5
  done
  echo "Packaged app did not quit normally." >&2
  exit 1
}
launch journey
wait_for_quit
rm -f "$SMOKE_DIR/frontend-ready"
launch restart OPUL_SMOKE_VIDEO_RESTART=1
wait_for_quit
echo "Packaged video journey and restart passed. Evidence: $SMOKE_DIR"
