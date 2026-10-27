#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ -d "$ROOT_DIR/Opul.app" ]] || { echo "Run Scripts/package_app.sh debug first." >&2; exit 1; }
open "$ROOT_DIR/Opul.app"
