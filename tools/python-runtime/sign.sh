#!/usr/bin/env bash
# Apple signing is independent of the mandatory Opul Ed25519 artifact signatures.
set -euo pipefail
RUNTIME=${1:?Usage: sign.sh runtime-directory}
IDENTITY=${APPLE_SIGNING_IDENTITY:--}
while IFS= read -r -d '' binary; do
  if file -b "$binary" | grep -q 'Mach-O'; then
    if [[ "$IDENTITY" = - ]]; then
      codesign --force --sign - --timestamp=none "$binary"
    else
      # CPython and cffi dynamically load extensions; no JIT or unsigned execution.
      codesign --force --sign "$IDENTITY" --timestamp --options runtime "$binary"
    fi
    codesign --verify --strict "$binary"
  fi
done < <(find "$RUNTIME" -type f -print0)
if [[ "$IDENTITY" != - ]]; then
  : "${NOTARY_APPLE_ID:?Developer ID publication requires notarization credentials}"
  : "${NOTARY_PASSWORD:?Notarization password required}"
  : "${NOTARY_TEAM_ID:?Notarization team required}"
  ARCHIVE=$(mktemp -d)
  trap 'rm -rf "$ARCHIVE"' EXIT
  ditto -c -k --keepParent "$RUNTIME" "$ARCHIVE/runtime.zip"
  xcrun notarytool submit "$ARCHIVE/runtime.zip" --wait --output-format json \
    --apple-id "$NOTARY_APPLE_ID" --password "$NOTARY_PASSWORD" --team-id "$NOTARY_TEAM_ID" \
    > "$ARCHIVE/notarization.json"
  cat "$ARCHIVE/notarization.json"
  python3 - "$ARCHIVE/notarization.json" <<'PY'
import json, sys
result = json.load(open(sys.argv[1]))
if result.get('status') != 'Accepted':
    raise SystemExit('Refusing publication: Apple did not accept the runtime')
PY
fi
