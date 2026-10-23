#!/usr/bin/env bash
# Launch the packaged app the way a user's Mac would and fail if it does not stay running.
# The build folder is moved aside first because SwiftPM's resource lookup falls back to it,
# which hides missing-resource crashes on the machine that built the app.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
source "$ROOT/app.env"
APP=${1:-"$ROOT/${APP_NAME}.app"}
HIDDEN_BUILD="$ROOT/../.opul-build-hidden-$$"

restore() {
  [[ -n "${PID:-}" ]] && kill "$PID" 2>/dev/null || true
  [[ -d "$HIDDEN_BUILD" ]] && mv "$HIDDEN_BUILD" "$ROOT/.build"
}
trap restore EXIT

# The bundled tools must run from the packaged app, and the QuickJS driver must stay sandboxed.
"$ROOT/Scripts/test_media_tools.sh" "$APP/Contents/Helpers"

[[ -d "$ROOT/.build" ]] && mv "$ROOT/.build" "$HIDDEN_BUILD"

"$APP/Contents/MacOS/$APP_NAME" >/dev/null 2>&1 &
PID=$!
sleep 10
if ! kill -0 "$PID" 2>/dev/null; then
  wait "$PID" || status=$?
  echo "ERROR: $APP_NAME exited during launch (status ${status:-0}). Check ~/Library/Logs/DiagnosticReports." >&2
  exit 1
fi
echo "$APP_NAME stayed running after launch."
