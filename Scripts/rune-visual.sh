#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
exec cargo run --locked --manifest-path prototypes/rune-visual/Cargo.toml \
  --target-dir target/rune-preview -j 2 -- "$@"
