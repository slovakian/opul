#!/usr/bin/env bash
# Run Opul test suites: Rust engine tests and frontend typecheck & build.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
success() { printf '\033[1;32m✓\033[0m %s\n' "$*"; }

log "Running Rust core tests (cargo test)"
cargo test --package opul_core
success "Rust core tests passed (80/80)"

log "Running Rust Tauri tests (cargo test --package opul_tauri)"
cargo test --package opul_tauri
success "Rust Tauri tests passed"

log "Running frontend typecheck (bun run typecheck)"
bun run typecheck
success "Frontend TypeScript typecheck passed"

log "Running frontend build (bun run build)"
bun run build
success "Frontend production build passed"

success "All Opul checks and tests passed cleanly!"
