#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
ARGS=(build --bundles dmg)
TARGET_DIR="$ROOT_DIR/target"
if [[ "${ARCHES:-}" == "arm64 x86_64" ]]; then
  ARGS+=(--target universal-apple-darwin)
  TARGET_DIR="$TARGET_DIR/universal-apple-darwin"
fi
if [[ -n "${MARKETING_VERSION:-}" ]]; then
  ARGS+=(--config "{\"version\":\"$MARKETING_VERSION\"}")
fi
export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
npm run tauri -- "${ARGS[@]}"
shopt -s nullglob
DMGS=("$TARGET_DIR/release/bundle/dmg/"*.dmg)
[[ ${#DMGS[@]} -eq 1 ]] || { echo "Expected one Tauri disk image." >&2; exit 1; }
cp "${DMGS[0]}" "$ROOT_DIR/Opul.dmg"
echo "Built $ROOT_DIR/Opul.dmg"
