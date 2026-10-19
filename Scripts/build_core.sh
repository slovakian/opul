#!/usr/bin/env bash
# Build the Rust core, regenerate Swift bindings, and stage the dylib SwiftPM links.
set -euo pipefail

CONF=${1:-debug}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

if [[ "$CONF" == "release" ]]; then
  cargo build --release
  LIB_DIR="$ROOT/target/release"
else
  cargo build
  LIB_DIR="$ROOT/target/debug"
fi

DYLIB="$LIB_DIR/libopul_core.dylib"
if [[ ! -f "$DYLIB" ]]; then
  echo "ERROR: missing $DYLIB" >&2
  exit 1
fi

mkdir -p "$ROOT/build/uniffi" "$ROOT/build/lib" \
  "$ROOT/Sources/OpulCore" "$ROOT/Sources/opul_coreFFI/include"

cargo run -p uniffi-bindgen -- generate \
  --library "$DYLIB" \
  --language swift \
  --out-dir "$ROOT/build/uniffi"

cp "$ROOT/build/uniffi/opul_core.swift" "$ROOT/Sources/OpulCore/opul_core.swift"
cp "$ROOT/build/uniffi/opul_coreFFI.h" "$ROOT/Sources/opul_coreFFI/include/opul_coreFFI.h"
cp "$ROOT/build/uniffi/opul_coreFFI.modulemap" "$ROOT/Sources/opul_coreFFI/include/module.modulemap"
perl -pi -e 's/[ \t]+$//' \
  "$ROOT/Sources/OpulCore/opul_core.swift" \
  "$ROOT/Sources/opul_coreFFI/include/opul_coreFFI.h"
cp -L "$DYLIB" "$ROOT/build/lib/libopul_core.dylib"

echo "Staged $ROOT/build/lib/libopul_core.dylib"
