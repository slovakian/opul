#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BUILD_CONF="release"
RAW_TARGET=""
CHANNEL="${CHANNEL:-${OPUL_CHANNEL:-stable}}"

usage() {
  cat <<'EOF'
Usage: Scripts/package_dmg.sh [OPTIONS] [debug|release] [stable|nightly] [TARGET]

Options:
  -t, --target <TARGET>   Target triple or architecture (aarch64-apple-darwin, x86_64-apple-darwin,
                          universal-apple-darwin, arm64, x64, x86_64)
  -c, --channel <CHANNEL> Release channel (stable, nightly; default: stable)
  -d, --debug             Build debug DMG
  -r, --release           Build release DMG (default)
  -h, --help              Show this help message

Environment variables:
  CHANNEL                 Release channel (stable or nightly, default: stable)
  OPUL_CHANNEL            Release channel override
  TARGET                  Target triple (e.g. aarch64-apple-darwin)
  ARCH                    Target architecture (e.g. arm64, x64)
  ARCHES                  Target architectures (e.g. "arm64 x86_64" for universal)
  MARKETING_VERSION       Marketing version override for the bundle
  APPLE_SIGNING_IDENTITY  Code signing identity (defaults to "-")
EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --channel=*)
      CHANNEL="${1#*=}"
      shift
      ;;
    --channel|-c)
      CHANNEL="${2:-}"
      shift 2
      ;;
    --target=*)
      RAW_TARGET="${1#*=}"
      shift
      ;;
    --target|-t)
      RAW_TARGET="${2:-}"
      shift 2
      ;;
    --debug|-d)
      BUILD_CONF="debug"
      shift
      ;;
    --release|-r)
      BUILD_CONF="release"
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    debug|release)
      BUILD_CONF="$1"
      shift
      ;;
    stable|nightly)
      CHANNEL="$1"
      shift
      ;;
    aarch64-apple-darwin|x86_64-apple-darwin|universal-apple-darwin|arm64|aarch64|x86_64|x64|intel|universal)
      RAW_TARGET="$1"
      shift
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

# Validate release channel
case "$CHANNEL" in
  stable)
    APP_NAME="Opul"
    BUNDLE_NAME="Opul.app"
    CHANNEL_CONFIG=""
    DMG_PREFIX="Opul"
    ;;
  nightly)
    APP_NAME="Opul Nightly"
    BUNDLE_NAME="Opul Nightly.app"
    CHANNEL_CONFIG="src-tauri/tauri.nightly.conf.json"
    DMG_PREFIX="Opul-Nightly"
    ;;
  *)
    echo "Unsupported channel: $CHANNEL (expected stable or nightly)" >&2
    exit 1
    ;;
esac

# If no target specified via CLI, check environment
if [[ -z "$RAW_TARGET" ]]; then
  if [[ -n "${TARGET:-}" ]]; then
    RAW_TARGET="$TARGET"
  elif [[ -n "${ARCH:-}" ]]; then
    RAW_TARGET="$ARCH"
  elif [[ "${ARCHES:-}" == "arm64 x86_64" ]]; then
    RAW_TARGET="universal-apple-darwin"
  elif [[ -n "${ARCHES:-}" ]]; then
    RAW_TARGET="$ARCHES"
  fi
fi

# Normalize target triple, arch suffix, and expected lipo arch
case "${RAW_TARGET:-}" in
  aarch64-apple-darwin|arm64|aarch64)
    TARGET_TRIPLE="aarch64-apple-darwin"
    ARCH_SUFFIX="arm64"
    EXPECTED_LIPO_ARCH="arm64"
    ;;
  x86_64-apple-darwin|x86_64|x64|intel)
    TARGET_TRIPLE="x86_64-apple-darwin"
    ARCH_SUFFIX="x64"
    EXPECTED_LIPO_ARCH="x86_64"
    ;;
  universal-apple-darwin|universal)
    TARGET_TRIPLE="universal-apple-darwin"
    ARCH_SUFFIX="universal"
    EXPECTED_LIPO_ARCH="arm64 x86_64"
    ;;
  "")
    HOST_ARCH="$(uname -m)"
    if [[ "$HOST_ARCH" == "arm64" || "$HOST_ARCH" == "aarch64" ]]; then
      TARGET_TRIPLE="aarch64-apple-darwin"
      ARCH_SUFFIX="arm64"
      EXPECTED_LIPO_ARCH="arm64"
    else
      TARGET_TRIPLE="x86_64-apple-darwin"
      ARCH_SUFFIX="x64"
      EXPECTED_LIPO_ARCH="x86_64"
    fi
    ;;
  *)
    echo "Unsupported target: $RAW_TARGET" >&2
    exit 1
    ;;
esac

ARGS=(build --bundles dmg --target "$TARGET_TRIPLE")
if [[ "$BUILD_CONF" == "debug" ]]; then
  ARGS+=(--debug)
fi
if [[ -n "$CHANNEL_CONFIG" ]]; then
  ARGS+=(--config "$CHANNEL_CONFIG")
fi
TARGET_DIR="$ROOT_DIR/target/$TARGET_TRIPLE"

if [[ -n "${MARKETING_VERSION:-}" ]]; then
  ARGS+=(--config "{\"version\":\"$MARKETING_VERSION\"}")
fi

export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
npm run tauri -- "${ARGS[@]}"

shopt -s nullglob
DMGS=("$TARGET_DIR/$BUILD_CONF/bundle/dmg/"*.dmg)
if [[ ${#DMGS[@]} -eq 0 && -d "$ROOT_DIR/target/$BUILD_CONF/bundle/dmg" ]]; then
  DMGS=("$ROOT_DIR/target/$BUILD_CONF/bundle/dmg/"*.dmg)
fi
[[ ${#DMGS[@]} -ge 1 ]] || { echo "Expected at least one Tauri disk image in $TARGET_DIR/$BUILD_CONF/bundle/dmg." >&2; exit 1; }

OUT_DMG="$ROOT_DIR/${DMG_PREFIX}-${ARCH_SUFFIX}.dmg"
cp "${DMGS[0]}" "$OUT_DMG"
# Maintain friendly DMG name compatibility
cp "$OUT_DMG" "$ROOT_DIR/${DMG_PREFIX}.dmg"

# Verify clean code signature and lipo architecture on the packaged DMG
MOUNT_DIR=$(mktemp -d "${TMPDIR:-/tmp}/opul-dmg-mount.XXXXXX")
cleanup() {
  hdiutil detach "$MOUNT_DIR" -force >/dev/null 2>&1 || true
  rm -rf "$MOUNT_DIR"
}
trap cleanup EXIT

hdiutil attach -nobrowse -readonly -mountpoint "$MOUNT_DIR" "$OUT_DMG" >/dev/null

APP_IN_DMG="$MOUNT_DIR/$BUNDLE_NAME"
[[ -d "$APP_IN_DMG" ]] || { echo "$BUNDLE_NAME missing in mounted DMG." >&2; exit 1; }

codesign --verify --deep --strict "$APP_IN_DMG"

ACTUAL_ARCH=$(lipo -archs "$APP_IN_DMG/Contents/MacOS/$APP_NAME" | tr ' ' '\n' | sort | xargs)
if [[ "$EXPECTED_LIPO_ARCH" == "arm64 x86_64" ]]; then
  if [[ "$ACTUAL_ARCH" != "arm64 x86_64" && "$ACTUAL_ARCH" != "x86_64 arm64" ]]; then
    echo "Architecture mismatch: expected universal (arm64 x86_64), got $ACTUAL_ARCH" >&2
    exit 1
  fi
else
  if [[ "$ACTUAL_ARCH" != "$EXPECTED_LIPO_ARCH" ]]; then
    echo "Architecture mismatch: expected $EXPECTED_LIPO_ARCH, got $ACTUAL_ARCH" >&2
    exit 1
  fi
fi

hdiutil detach "$MOUNT_DIR" -force >/dev/null
rm -rf "$MOUNT_DIR"
trap - EXIT

DMG_SIZE_MB=$(du -m "$OUT_DMG" | cut -f1)
echo "Built and verified $OUT_DMG (${DMG_SIZE_MB}MB, arch: $ACTUAL_ARCH, channel: $CHANNEL)"
