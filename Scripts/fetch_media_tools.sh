#!/usr/bin/env bash
# Download the pinned media-tools release, check every file against MediaTools/release.env,
# and print the folder that holds ffmpeg, ffprobe and qjs. Files are cached under .build/.
#
# MEDIA_TOOLS_DIR=<folder> uses tools you built with Scripts/build_media_tools.sh instead.
# Their checksums differ from the release, so they are checked by running them.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
source "$ROOT/MediaTools/release.env"
REPOSITORY=${MEDIA_TOOLS_REPOSITORY:-slovakian/opul}

if [[ -n "${MEDIA_TOOLS_DIR:-}" ]]; then
  echo "warning: using unpinned media tools from $MEDIA_TOOLS_DIR" >&2
  "$ROOT/Scripts/test_media_tools.sh" "$MEDIA_TOOLS_DIR" >&2
  cd "$MEDIA_TOOLS_DIR" && pwd
  exit 0
fi

CACHE="$ROOT/.build/media-tools/$MEDIA_TOOLS_TAG"
mkdir -p "$CACHE"
for tool in ffmpeg ffprobe qjs; do
  variable="$(tr '[:lower:]' '[:upper:]' <<<"$tool")_SHA256"
  expected=${!variable}
  file="$CACHE/$tool"
  if [[ ! -f "$file" ]] || [[ "$(shasum -a 256 "$file" | cut -d' ' -f1)" != "$expected" ]]; then
    curl -fsSL --retry 5 --retry-all-errors -o "$file.part" \
      "https://github.com/$REPOSITORY/releases/download/$MEDIA_TOOLS_TAG/$tool"
    mv "$file.part" "$file"
  fi
  actual=$(shasum -a 256 "$file" | cut -d' ' -f1)
  if [[ "$actual" != "$expected" ]]; then
    rm -f "$file"
    echo "ERROR: $tool from $MEDIA_TOOLS_TAG has SHA-256 $actual, but MediaTools/release.env pins $expected" >&2
    exit 1
  fi
  chmod 755 "$file"
done
echo "$CACHE"
