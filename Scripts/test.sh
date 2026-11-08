#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
case "${1:-}" in
  --rust|-r) Scripts/fetch_ffmpeg.sh; cargo test --workspace; exit ;;
  --help|-h) echo "Usage: Scripts/test.sh [--rust]"; exit ;;
  "") ;;
  *) echo "Unknown option: $1" >&2; exit 1 ;;
esac
Scripts/fetch_ffmpeg.sh
cargo fmt --all --check
cargo test --workspace
vp test
vp run build
vp run test
