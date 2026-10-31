#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

BUILD_CONF="release"
RAW_TARGET=""

usage() {
  cat <<'EOF'
Usage: Scripts/package_app.sh [OPTIONS] [debug|release] [TARGET]

Options:
  -t, --target <TARGET>  Target triple or architecture (aarch64-apple-darwin, x86_64-apple-darwin,
                         universal-apple-darwin, arm64, x64, x86_64)
  -d, --debug            Build debug bundle
  -r, --release          Build release bundle (default)
  -h, --help             Show this help message

Environment variables:
  TARGET                 Target triple (e.g. aarch64-apple-darwin)
  ARCH                   Target architecture (e.g. arm64, x64)
  ARCHES                 Target architectures (e.g. "arm64 x86_64" for universal)
  MARKETING_VERSION      Marketing version override for the bundle
  APPLE_SIGNING_IDENTITY Code signing identity (defaults to "-")
EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
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

ARGS=(build --bundles app --target "$TARGET_TRIPLE")
if [[ "$BUILD_CONF" == "debug" ]]; then
  ARGS+=(--debug)
fi
TARGET_DIR="$ROOT_DIR/target/$TARGET_TRIPLE"

if [[ -n "${MARKETING_VERSION:-}" ]]; then
  ARGS+=(--config "{\"version\":\"$MARKETING_VERSION\"}")
fi

export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
npm run tauri -- "${ARGS[@]}"

BUNDLE_SRC="$TARGET_DIR/$BUILD_CONF/bundle/macos/Opul.app"
if [[ ! -d "$BUNDLE_SRC" && -d "$ROOT_DIR/target/$BUILD_CONF/bundle/macos/Opul.app" ]]; then
  BUNDLE_SRC="$ROOT_DIR/target/$BUILD_CONF/bundle/macos/Opul.app"
fi

[[ -d "$BUNDLE_SRC" ]] || { echo "Expected app bundle at $BUNDLE_SRC" >&2; exit 1; }

# Replace the generated bundle instead of merging into an existing app. Besides
# retaining obsolete resources, merging can leave Finder showing its cached icon.
rm -rf "$ROOT_DIR/Opul.app"
ditto "$BUNDLE_SRC" "$ROOT_DIR/Opul.app"

# Verify code signature
codesign --verify --deep --strict "$ROOT_DIR/Opul.app"

# Verify architecture with lipo
ACTUAL_ARCH=$(lipo -archs "$ROOT_DIR/Opul.app/Contents/MacOS/Opul" | tr ' ' '\n' | sort | xargs)
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

echo "Built $ROOT_DIR/Opul.app (arch: $ACTUAL_ARCH, target: $TARGET_TRIPLE, profile: $BUILD_CONF)"
