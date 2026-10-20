#!/usr/bin/env bash
# Wrap Opul.app in a drag-to-Applications disk image. Run Scripts/package_app.sh first.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"
source "$ROOT/app.env"
APP_IDENTITY=${APP_IDENTITY:-}

APP="$ROOT/${APP_NAME}.app"
DMG="$ROOT/${APP_NAME}.dmg"
VENV="$ROOT/.build/dmgbuild"

if [[ ! -d "$APP" ]]; then
  echo "ERROR: $APP not found. Run Scripts/package_app.sh first." >&2
  exit 1
fi

if [[ ! -x "$VENV/bin/dmgbuild" ]]; then
  /usr/bin/python3 -m venv "$VENV"
  "$VENV/bin/pip" install --quiet --disable-pip-version-check dmgbuild==1.6.5
fi

rm -f "$DMG"
"$VENV/bin/dmgbuild" -s "$ROOT/Scripts/dmg_settings.py" -D app="$APP" "$APP_NAME" "$DMG"

if [[ -n "$APP_IDENTITY" ]]; then
  codesign --force --timestamp --sign "$APP_IDENTITY" "$DMG"
fi

hdiutil verify -quiet "$DMG"
echo "Created $DMG"
