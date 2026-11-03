#!/usr/bin/env bash
# Exercise the fetch boundary with real pinned tools and a corrupt response.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
source "$ROOT/tools/ffmpeg/sources.env"
"$ROOT/Scripts/fetch_ffmpeg.sh" arm64
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/Scripts" "$WORK/tools/ffmpeg" "$WORK/.build/ffmpeg-cache" "$WORK/bin"
cp "$ROOT/Scripts/fetch_ffmpeg.sh" "$ROOT/Scripts/verify_ffmpeg.sh" "$WORK/Scripts/"
cp "$ROOT/tools/ffmpeg/"*.env "$WORK/tools/ffmpeg/"
cp "$ROOT/.build/ffmpeg-cache/$FFMPEG_RELEASE-arm64.tar.gz" "$WORK/.build/ffmpeg-cache/"
"$WORK/Scripts/fetch_ffmpeg.sh" arm64
EXPECTED=$(shasum -a 256 "$WORK/src-tauri/resources/media-tools/ffmpeg")
cat > "$WORK/bin/codesign" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == --force ]]; then
  [[ $# -eq 7 && "$2" == --options && "$3" == runtime && "$4" == --timestamp && "$5" == --sign && "$6" == Test-Identity ]]
  # Exercise real signature mutation without requiring a Developer ID certificate.
  exec /usr/bin/codesign --force --options runtime --sign - "$7"
fi
exec /usr/bin/codesign "$@"
SH
chmod +x "$WORK/bin/codesign"
PATH="$WORK/bin:$PATH" APPLE_SIGNING_IDENTITY=Test-Identity "$WORK/Scripts/fetch_ffmpeg.sh" arm64
test -s "$WORK/src-tauri/resources/media-tools/ARCHIVE.txt"
test "$EXPECTED" != "$(shasum -a 256 "$WORK/src-tauri/resources/media-tools/ffmpeg")"
"$WORK/Scripts/verify_ffmpeg.sh" "$WORK/src-tauri/resources/media-tools" arm64
"$WORK/Scripts/fetch_ffmpeg.sh" arm64
echo corrupt-cache > "$WORK/.build/ffmpeg-cache/$FFMPEG_RELEASE-arm64.tar.gz"
cat > "$WORK/bin/curl" <<'SH'
#!/usr/bin/env bash
while [[ $# -gt 0 ]]; do
  if [[ "$1" == -o ]]; then echo corrupt-response > "$2"; exit 0; fi
  shift
done
exit 1
SH
chmod +x "$WORK/bin/curl"
if PATH="$WORK/bin:$PATH" "$WORK/Scripts/fetch_ffmpeg.sh" arm64; then
  echo 'FAIL: corrupt archive was accepted' >&2
  exit 1
fi
test "$EXPECTED" = "$(shasum -a 256 "$WORK/src-tauri/resources/media-tools/ffmpeg")"
if "$WORK/Scripts/fetch_ffmpeg.sh" universal-apple-darwin; then
  echo 'FAIL: universal tools were accepted' >&2
  exit 1
fi
echo 'PASS: signing refreshes checksums; corrupt downloads preserve resources; universal targets rejected'
