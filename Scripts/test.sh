#!/usr/bin/env bash
# Run macOS app test suites: Rust core tests and Swift checks.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

RUN_RUST=1
RUN_SWIFT=1

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
success() { printf '\033[1;32m✓\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31mERROR:\033[0m %s\n' "$*" >&2; exit 1; }

for arg in "$@"; do
  case "${arg}" in
    --rust|-r)
      RUN_RUST=1
      RUN_SWIFT=0
      ;;
    --swift|-s)
      RUN_RUST=0
      RUN_SWIFT=1
      ;;
    --help|-h)
      printf "Usage: %s [--rust] [--swift]\n" "$(basename "$0")"
      printf "  --rust, -r   Run only Rust core unit tests\n"
      printf "  --swift, -s  Run only Swift checks (OpulPasteChecks & OpulCoreChecks)\n"
      printf "  --help, -h   Show this help message\n"
      exit 0
      ;;
    *)
      fail "Unknown option: ${arg}. Use --help for usage."
      ;;
  esac
done

if [[ "${RUN_RUST}" == "1" ]]; then
  log "Running Rust core unit tests (cargo test)"
  cargo test
  success "Rust tests passed"
fi

if [[ "${RUN_SWIFT}" == "1" ]]; then
  # Ensure the Rust core dylib and Swift UniFFI bindings are staged for Swift targets
  if [[ ! -f "$ROOT_DIR/build/lib/libopul_core.dylib" ]]; then
    log "Staging Rust core dylib and UniFFI bindings"
    "$ROOT_DIR/Scripts/build_core.sh" debug
  fi

  log "Running Swift paste checks (swift run OpulPasteChecks)"
  swift run OpulPasteChecks
  success "Opul paste checks passed"

  log "Running Swift core checks (swift run OpulCoreChecks)"
  swift run OpulCoreChecks
  success "OpulCore checks passed"

  log "Running Swift storage checks (swift run Opul --check-storage)"
  swift run Opul --check-storage
  success "OpulStorage checks passed"
fi

success "All macOS app checks passed successfully!"
