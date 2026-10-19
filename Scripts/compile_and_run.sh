#!/usr/bin/env bash
# Kill running instances, package, relaunch, verify.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# version.env lives at the repository root.
GIT_ROOT="$ROOT_DIR"
if [[ -f "$GIT_ROOT/version.env" ]]; then
  # shellcheck disable=SC1091
  source "$GIT_ROOT/version.env"
fi
APP_NAME=${APP_NAME:-MyApp}
APP_BUNDLE="${ROOT_DIR}/${APP_NAME}.app"
APP_PROCESS_PATTERN="${APP_NAME}.app/Contents/MacOS/${APP_NAME}"
DEBUG_PROCESS_PATTERN="${ROOT_DIR}/.build/debug/${APP_NAME}"
RELEASE_PROCESS_PATTERN="${ROOT_DIR}/.build/release/${APP_NAME}"
RUN_TESTS=0
BUILD_CONF="debug"
RELEASE_ARCHES=""

log() { printf '%s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

for arg in "$@"; do
  case "${arg}" in
    --test|-t) RUN_TESTS=1 ;;
    --debug|-d) BUILD_CONF="debug" ;;
    --release|-r) BUILD_CONF="release" ;;
    --release-universal)
      BUILD_CONF="release"
      RELEASE_ARCHES="arm64 x86_64"
      ;;
    --release-arches=*)
      BUILD_CONF="release"
      RELEASE_ARCHES="${arg#*=}"
      ;;
    --help|-h)
      log "Usage: $(basename "$0") [--debug|--release] [--test] [--release-universal] [--release-arches=\"arm64 x86_64\"]"
      log "  --debug, -d              Build in debug configuration (default, faster for local preview)"
      log "  --release, -r            Build in release configuration"
      log "  --test, -t               Run the test suite before packaging and launching"
      log "  --release-universal      Build release for both arm64 and x86_64"
      log "  --release-arches=ARCHES  Build release for specified architectures"
      exit 0
      ;;
    *)
      fail "Unknown option: ${arg}. Use --help for usage."
      ;;
  esac
done

log "==> Killing existing ${APP_NAME} instances"
pkill -f "${APP_PROCESS_PATTERN}" 2>/dev/null || true
pkill -f "${DEBUG_PROCESS_PATTERN}" 2>/dev/null || true
pkill -f "${RELEASE_PROCESS_PATTERN}" 2>/dev/null || true
pkill -x "${APP_NAME}" 2>/dev/null || true

if [[ "${RUN_TESTS}" == "1" ]]; then
  log "==> Running test suite"
  "${ROOT_DIR}/Scripts/test.sh"
fi

HOST_ARCH="$(uname -m)"
ARCHES_VALUE="${HOST_ARCH}"
if [[ -n "${RELEASE_ARCHES}" ]]; then
  ARCHES_VALUE="${RELEASE_ARCHES}"
fi

log "==> Package app (${BUILD_CONF})"
SIGNING_MODE=adhoc ARCHES="${ARCHES_VALUE}" "${ROOT_DIR}/Scripts/package_app.sh" "${BUILD_CONF}"

log "==> Launch app"
if ! open "${APP_BUNDLE}"; then
  log "WARN: open failed; launching binary directly."
  "${APP_BUNDLE}/Contents/MacOS/${APP_NAME}" >/dev/null 2>&1 &
  disown
fi

for _ in {1..10}; do
  if pgrep -f "${APP_PROCESS_PATTERN}" >/dev/null 2>&1; then
    log "OK: ${APP_NAME} is running."
    exit 0
  fi
  sleep 0.4
done
fail "App exited immediately. Check crash logs in Console.app (User Reports)."
