#!/usr/bin/env bash
# Build Opul.app. Signs ad-hoc unless APP_IDENTITY names a Developer ID certificate.
set -euo pipefail

CONF=${1:-release}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

source "$ROOT/app.env"
APP_IDENTITY=${APP_IDENTITY:-}

# Releases pass MARKETING_VERSION; local builds report the most recent release tag.
if [[ -z "${MARKETING_VERSION:-}" ]]; then
  LATEST_TAG=$(git describe --tags --abbrev=0 --match 'v[0-9]*' 2>/dev/null || echo v0.0.0)
  MARKETING_VERSION=${LATEST_TAG#v}
fi
BUILD_NUMBER=${BUILD_NUMBER:-$MARKETING_VERSION}

ARCH_LIST=( ${ARCHES:-} )
if [[ ${#ARCH_LIST[@]} -eq 0 ]]; then
  HOST_ARCH=$(uname -m)
  ARCH_LIST=("$HOST_ARCH")
fi

for ARCH in "${ARCH_LIST[@]}"; do
  case "$ARCH" in
    arm64) RUST_TARGET="aarch64-apple-darwin" ;;
    x86_64) RUST_TARGET="x86_64-apple-darwin" ;;
    *) echo "ERROR: unsupported architecture $ARCH" >&2; exit 1 ;;
  esac
  rustup target add "$RUST_TARGET"
  "$ROOT/Scripts/build_core.sh" "$CONF" "$RUST_TARGET"
  swift build -c "$CONF" --arch "$ARCH"
  mkdir -p "$ROOT/.build/opul-core-$ARCH"
  cp -L "$ROOT/build/lib/libopul_core.dylib" "$ROOT/.build/opul-core-$ARCH/libopul_core.dylib"
done

APP="$ROOT/${APP_NAME}.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"

# macOS icon: full iconset compiled from the 1024px master. The system masks it.
ICON_TARGET=""
if [[ -f "$ROOT/assets/AppIcon.png" ]]; then
  ICON_TARGET="$ROOT/.build/Icon.icns"
  "$ROOT/Scripts/build_icon.sh" "$ROOT/assets/AppIcon.png" "$ICON_TARGET"
fi

BUILD_TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key><string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key><string>${BUNDLE_ID}</string>
    <key>CFBundleExecutable</key><string>${APP_NAME}</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>${MARKETING_VERSION}</string>
    <key>CFBundleVersion</key><string>${BUILD_NUMBER}</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
    <key>LSMinimumSystemVersion</key><string>${MACOS_MIN_VERSION}</string>
    <key>CFBundleIconFile</key><string>Icon</string>
    <key>CFBundleURLTypes</key>
    <array>
        <dict>
            <key>CFBundleURLName</key><string>${BUNDLE_ID}.download</string>
            <key>CFBundleURLSchemes</key><array><string>opul</string></array>
        </dict>
    </array>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key><string>BitTorrent file</string>
            <key>CFBundleTypeRole</key><string>Viewer</string>
            <key>LSHandlerRank</key><string>Alternate</string>
            <key>CFBundleTypeExtensions</key><array><string>torrent</string></array>
        </dict>
    </array>
    <key>NSCameraUsageDescription</key><string>Guided extraction may open sites that use the camera.</string>
    <key>NSMicrophoneUsageDescription</key><string>Guided extraction may open sites that use the microphone.</string>
    <key>SUFeedURL</key><string>${SPARKLE_FEED_URL}</string>
    <key>SUPublicEDKey</key><string>${SPARKLE_PUBLIC_KEY}</string>
    <key>SUEnableAutomaticChecks</key><true/>
    <key>SUAutomaticallyUpdate</key><true/>
    <key>BuildTimestamp</key><string>${BUILD_TIMESTAMP}</string>
    <key>GitCommit</key><string>${GIT_COMMIT}</string>
</dict>
</plist>
PLIST

build_product_path() {
  local name="$1"
  local arch="$2"
  echo ".build/${arch}-apple-macosx/$CONF/$name"
}

verify_binary_arches() {
  local binary="$1"; shift
  local expected=("$@")
  local actual
  actual=$(lipo -archs "$binary")
  local actual_count expected_count
  actual_count=$(wc -w <<<"$actual" | tr -d ' ')
  expected_count=${#expected[@]}
  if [[ "$actual_count" -ne "$expected_count" ]]; then
    echo "ERROR: $binary arch mismatch (expected: ${expected[*]}, actual: ${actual})" >&2
    exit 1
  fi
  for arch in "${expected[@]}"; do
    if [[ "$actual" != *"$arch"* ]]; then
      echo "ERROR: $binary missing arch $arch (have: ${actual})" >&2
      exit 1
    fi
  done
}

install_binary() {
  local name="$1"
  local dest="$2"
  local binaries=()
  for arch in "${ARCH_LIST[@]}"; do
    local src
    src=$(build_product_path "$name" "$arch")
    if [[ ! -f "$src" ]]; then
      echo "ERROR: Missing ${name} build for ${arch} at ${src}" >&2
      exit 1
    fi
    binaries+=("$src")
  done
  if [[ ${#ARCH_LIST[@]} -gt 1 ]]; then
    lipo -create "${binaries[@]}" -output "$dest"
  else
    cp "${binaries[0]}" "$dest"
  fi
  chmod +x "$dest"
  verify_binary_arches "$dest" "${ARCH_LIST[@]}"
}

install_binary "$APP_NAME" "$APP/Contents/MacOS/$APP_NAME"

# Bundle app resources (if any).
APP_RESOURCES_DIR="$ROOT/Sources/$APP_NAME/Resources"
if [[ -d "$APP_RESOURCES_DIR" ]]; then
  for resource in "$APP_RESOURCES_DIR"/*; do
    [[ -e "$resource" ]] || continue
    # Sounds are declared as a SwiftPM resource and arrive in the bundle below.
    [[ "$(basename "$resource")" == "Sounds" ]] && continue
    cp -R "$resource" "$APP/Contents/Resources/"
  done
fi

# SwiftPM resource bundles are emitted next to the built binary.
PREFERRED_BUILD_DIR="$(dirname "$(build_product_path "$APP_NAME" "${ARCH_LIST[0]}")")"
shopt -s nullglob
SWIFTPM_BUNDLES=("${PREFERRED_BUILD_DIR}/"*.bundle)
shopt -u nullglob
for bundle in "${SWIFTPM_BUNDLES[@]+"${SWIFTPM_BUNDLES[@]}"}"; do
  cp -R "$bundle" "$APP/Contents/Resources/"
done

# Sparkle ships as a universal framework. Its XPC services exist only for sandboxed apps.
SPARKLE="$APP/Contents/Frameworks/Sparkle.framework"
ditto "$PREFERRED_BUILD_DIR/Sparkle.framework" "$SPARKLE"
rm -rf "$SPARKLE/Versions/B/XPCServices" "$SPARKLE/XPCServices"

if [[ -n "$ICON_TARGET" && -f "$ICON_TARGET" ]]; then
  cp "$ICON_TARGET" "$APP/Contents/Resources/Icon.icns"
fi

DYLIBS=()
for ARCH in "${ARCH_LIST[@]}"; do
  DYLIB_SRC="$ROOT/.build/opul-core-$ARCH/libopul_core.dylib"
  if [[ ! -f "$DYLIB_SRC" ]]; then
    echo "ERROR: missing $DYLIB_SRC" >&2
    exit 1
  fi
  DYLIBS+=("$DYLIB_SRC")
done
if [[ ${#DYLIBS[@]} -gt 1 ]]; then
  lipo -create "${DYLIBS[@]}" -output "$APP/Contents/Frameworks/libopul_core.dylib"
else
  cp -L "${DYLIBS[0]}" "$APP/Contents/Frameworks/libopul_core.dylib"
fi
verify_binary_arches "$APP/Contents/Frameworks/libopul_core.dylib" "${ARCH_LIST[@]}"
OLD_ID=$(otool -D "${DYLIBS[0]}" | awk 'NR==2 { print }')
install_name_tool -id @rpath/libopul_core.dylib "$APP/Contents/Frameworks/libopul_core.dylib"
install_name_tool -change "$OLD_ID" @rpath/libopul_core.dylib "$APP/Contents/MacOS/$APP_NAME"
install_name_tool -add_rpath @executable_path/../Frameworks "$APP/Contents/MacOS/$APP_NAME"

# Ensure contents are writable before stripping attributes and signing.
chmod -R u+w "$APP"

# Strip extended attributes to prevent AppleDouble files that break code sealing.
xattr -cr "$APP"
find "$APP" -name '._*' -delete

if [[ -n "$APP_IDENTITY" ]]; then
  CODESIGN_ARGS=(--force --timestamp --options runtime --sign "$APP_IDENTITY")
else
  CODESIGN_ARGS=(--force --sign "-")
fi

# Sign from the inside out: nested helpers, then frameworks, then the app.
codesign "${CODESIGN_ARGS[@]}" "$SPARKLE/Versions/B/Autoupdate"
codesign "${CODESIGN_ARGS[@]}" "$SPARKLE/Versions/B/Updater.app"
codesign "${CODESIGN_ARGS[@]}" "$SPARKLE"
codesign "${CODESIGN_ARGS[@]}" "$APP/Contents/Frameworks/libopul_core.dylib"
codesign "${CODESIGN_ARGS[@]}" --entitlements "$ROOT/Opul.entitlements" "$APP"

codesign --verify --deep --strict "$APP"
echo "Created $APP ($MARKETING_VERSION)"
