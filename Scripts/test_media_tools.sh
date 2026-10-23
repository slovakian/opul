#!/usr/bin/env bash
# Check the bundled media tools in a folder (default: the packaged app's Contents/Helpers):
# each runs, FFmpeg is LGPL, and the QuickJS driver keeps scripts away from the system.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
source "$ROOT/app.env"
source "$ROOT/MediaTools/sources.env"
TOOLS=${1:-"$ROOT/${APP_NAME}.app/Contents/Helpers"}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

fail() { echo "ERROR: $*" >&2; exit 1; }

for tool in ffmpeg ffprobe; do
  "$TOOLS/$tool" -hide_banner -version >"$WORK/$tool.txt" || fail "$tool -version failed"
  head -1 "$WORK/$tool.txt"
  grep -q -- '--enable-gpl\|--enable-nonfree' "$WORK/$tool.txt" && fail "$tool is not LGPL"
done
grep -q -- "--enable-libmp3lame" "$WORK/ffmpeg.txt" || fail "ffmpeg lacks libmp3lame"

# Configure skips a misspelled component without complaint, so check what Opul relies on.
require() {
  local kind="$1"; shift
  "$TOOLS/ffmpeg" -hide_banner "-$kind" >"$WORK/$kind.txt" 2>/dev/null
  for name in "$@"; do
    grep -Eq "(^| )$name( |$)" "$WORK/$kind.txt" || fail "ffmpeg lacks the $name ${kind%s}"
  done
}
require encoders libmp3lame aac mjpeg mov_text srt webvtt ass
require decoders aac mp3 opus vorbis webvtt subrip mov_text webp png mjpeg
require muxers mp4 ipod matroska webm mp3 srt webvtt image2 mjpeg mpegts
require demuxers mov,mp4,m4a,3gp,3g2,mj2 matroska,webm mp3 aac ogg hls mpegts webvtt srt image2 webp_pipe png_pipe jpeg_pipe
require bsfs aac_adtstoasc h264_mp4toannexb extract_extradata mjpeg2jpeg setts
require filters aresample aformat scale format
require protocols https tls crypto

# yt-dlp reads the version from --help, which must exit with 1 like stock qjs.
status=0
"$TOOLS/qjs" --help >"$WORK/help.txt" 2>"$WORK/help.err" || status=$?
[[ $status -eq 1 ]] || fail "qjs --help exited with $status, expected 1"
[[ ! -s "$WORK/help.err" ]] || fail "qjs --help wrote to stderr"
grep -Eq "^QuickJS version $QUICKJS_VERSION\$" "$WORK/help.txt" || fail "qjs --help did not report $QUICKJS_VERSION"
head -1 "$WORK/help.txt"

# The sandbox: nothing reaches the file system or other processes.
cp "$ROOT/MediaTools/sandbox-test.js" "$WORK/sandbox-test.js"
"$TOOLS/qjs" --script "$WORK/sandbox-test.js" >"$WORK/sandbox.out" 2>"$WORK/sandbox.err" \
  || fail "the sandbox test script failed: $(cat "$WORK/sandbox.err")"
[[ ! -s "$WORK/sandbox.err" ]] || fail "the sandbox test wrote to stderr: $(cat "$WORK/sandbox.err")"
cat >"$WORK/sandbox.expected" <<'EXPECTED'
std: missing
os: missing
__loadScript: missing
require: missing
Deno: missing
process: missing
import os: missing
import std: missing
import qjs:os: missing
import qjs:std: missing
import /etc/hosts: missing
stdout: ok
EXPECTED
diff -u "$WORK/sandbox.expected" "$WORK/sandbox.out" || fail "the QuickJS driver is not sandboxed"

# Failures go to stderr with a non-zero status, which is how yt-dlp notices them.
echo 'throw new Error("boom")' >"$WORK/throw.js"
if "$TOOLS/qjs" --script "$WORK/throw.js" >/dev/null 2>"$WORK/throw.err"; then
  fail "an uncaught exception exited with 0"
fi
grep -q boom "$WORK/throw.err" || fail "an uncaught exception was not written to stderr"
echo 'Promise.reject(new Error("later"))' >"$WORK/reject.js"
if "$TOOLS/qjs" --script "$WORK/reject.js" >/dev/null 2>&1; then
  fail "an unhandled rejection exited with 0"
fi

echo "Media tools in $TOOLS passed."
