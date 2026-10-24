#!/usr/bin/env bash
# Compile and run Opul locally
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

if [[ "${1:-}" == "dev" ]]; then
  exec bun run tauri dev
else
  bun run tauri build
  open target/release/bundle/macos/Opul.app
fi
