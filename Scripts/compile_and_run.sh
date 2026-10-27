#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if [[ "${1:-}" == "--test" ]]; then Scripts/test.sh; shift; fi
case "${1:-}" in
  --release|-r) Scripts/package_app.sh release; Scripts/launch.sh ;;
  --help|-h) echo "Usage: Scripts/compile_and_run.sh [--test] [--release]" ;;
  ""|--debug|-d) npm run tauri -- dev ;;
  *) echo "Unknown option: $1" >&2; exit 1 ;;
esac
