#!/usr/bin/env bash
# Build the tools Opul bundles in Contents/Helpers: a minimal LGPL ffmpeg and ffprobe, and the
# sandboxed QuickJS driver installed as qjs. Apple silicon only.
#
#   Scripts/build_media_tools.sh [output-folder]
#
# The output folder receives ffmpeg, ffprobe, qjs, SHA256SUMS, BUILD-INFO.txt and the exact
# source tarballs, which the LGPL asks us to publish beside the binaries.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${1:-"$ROOT/.build/media-tools-build/out"}
WORK="$ROOT/.build/media-tools-build"
JOBS=$(sysctl -n hw.ncpu)
source "$ROOT/MediaTools/sources.env"
source "$ROOT/app.env"

export MACOSX_DEPLOYMENT_TARGET="$MACOS_MIN_VERSION"
ARCH_FLAGS="-arch arm64 -mmacosx-version-min=$MACOS_MIN_VERSION"

mkdir -p "$WORK/downloads" "$WORK/prefix" "$OUT/sources"
OUT=$(cd "$OUT" && pwd)

fetch() {
  local url="$1" sha="$2" file="$WORK/downloads/$(basename "$1")"
  if [[ ! -f "$file" ]] || ! shasum -a 256 -c <<<"$sha  $file" >/dev/null 2>&1; then
    curl -fsSL --retry 5 --retry-all-errors -o "$file.part" "$url"
    mv "$file.part" "$file"
  fi
  if ! shasum -a 256 -c <<<"$sha  $file" >/dev/null; then
    echo "ERROR: $(basename "$file") does not match its pinned SHA-256" >&2
    exit 1
  fi
  cp "$file" "$OUT/sources/"
  echo "$file"
}

unpack() {
  local archive="$1" folder="$2"
  rm -rf "$WORK/$folder"
  tar -xf "$archive" -C "$WORK"
  test -d "$WORK/$folder"
}

echo "==> LAME $LAME_VERSION"
unpack "$(fetch "$LAME_URL" "$LAME_SHA256")" "lame-$LAME_VERSION"
(
  cd "$WORK/lame-$LAME_VERSION"
  # lame.sym names a function that 3.100 no longer builds; only the shared library reads it.
  sed -i '' '/lame_init_old/d' include/libmp3lame.sym
  CFLAGS="$ARCH_FLAGS -O2" ./configure --prefix="$WORK/prefix" --host=aarch64-apple-darwin \
    --disable-shared --enable-static --disable-frontend --disable-decoder --disable-gtktest \
    --enable-nasm=no >/dev/null
  make -j"$JOBS" >/dev/null
  make install >/dev/null
)

echo "==> FFmpeg $FFMPEG_VERSION"
unpack "$(fetch "$FFMPEG_URL" "$FFMPEG_SHA256")" "ffmpeg-$FFMPEG_VERSION"
FFMPEG_CONFIGURE=(
  --prefix="$WORK/prefix"
  --arch=arm64 --target-os=darwin --cc=clang
  --extra-cflags="$ARCH_FLAGS -I$WORK/prefix/include"
  --extra-ldflags="$ARCH_FLAGS -L$WORK/prefix/lib"
  --pkg-config=false
  --disable-shared --enable-static --disable-debug --disable-doc --enable-small
  --disable-autodetect --disable-everything
  --disable-ffplay --disable-avdevice
  --enable-ffmpeg --enable-ffprobe
  --enable-avcodec --enable-avformat --enable-avfilter --enable-swscale --enable-swresample
  --enable-zlib --enable-audiotoolbox --enable-securetransport --enable-libmp3lame
  --enable-muxer=mp4,mov,ipod,matroska,webm,mp3,srt,webvtt,ass,image2,mjpeg,mpegts
  --enable-demuxer=mov,matroska,mp3,aac,ogg,hls,mpegts,webvtt,srt,ass,image2,image_webp_pipe,image_png_pipe,image_jpeg_pipe,data
  --enable-decoder=aac,mp3,mp3float,opus,vorbis,webvtt,srt,subrip,ass,ssa,movtext,webp,vp8,png,mjpeg
  --enable-encoder=libmp3lame,aac,mjpeg,srt,subrip,movtext,webvtt,ass,ssa
  --enable-parser=aac,h264,hevc,vp9,av1,opus,vorbis,mpegaudio,mjpeg,png,webp
  --enable-bsf=aac_adtstoasc,h264_mp4toannexb,hevc_mp4toannexb,vp9_superframe,extract_extradata,mjpeg2jpeg,setts,null
  --enable-filter=aresample,aformat,anull,scale,format,null,copy
  --enable-protocol=file,pipe,http,https,tls,tcp,crypto,data
)
(
  cd "$WORK/ffmpeg-$FFMPEG_VERSION"
  ./configure "${FFMPEG_CONFIGURE[@]}" >"$WORK/ffmpeg-configure.log"
  # Fail if anything licensed under the GPL or as non-free slipped into the build.
  grep -q '^License: LGPL version 2.1 or later' "$WORK/ffmpeg-configure.log" || {
    echo "ERROR: FFmpeg is not configured as LGPL" >&2
    grep '^License:' "$WORK/ffmpeg-configure.log" >&2 || true
    exit 1
  }
  make -j"$JOBS" ffmpeg ffprobe >/dev/null
  cp ffmpeg ffprobe "$OUT/"
)

echo "==> QuickJS $QUICKJS_VERSION"
unpack "$(fetch "$QUICKJS_URL" "$QUICKJS_SHA256")" "quickjs-$QUICKJS_VERSION"
(
  cd "$WORK/quickjs-$QUICKJS_VERSION"
  make -j"$JOBS" CC="clang $ARCH_FLAGS" CONFIG_LTO= libquickjs.a >/dev/null
  clang $ARCH_FLAGS -O2 -Wall -Werror -DCONFIG_VERSION="\"$QUICKJS_VERSION\"" -I. \
    "$ROOT/MediaTools/qjs-driver.c" libquickjs.a -lm -o "$OUT/qjs"
)
cp "$ROOT/MediaTools/qjs-driver.c" "$OUT/sources/"

strip -x "$OUT/ffmpeg" "$OUT/ffprobe" "$OUT/qjs"
for tool in ffmpeg ffprobe qjs; do
  test "$(lipo -archs "$OUT/$tool")" = "arm64"
done

{
  echo "Opul media tools, built $(date -u +%Y-%m-%dT%H:%M:%SZ) on $(sw_vers -productVersion) with $(clang --version | head -1)"
  echo
  echo "FFmpeg $FFMPEG_VERSION (LGPL 2.1 or later), LAME $LAME_VERSION (LGPL), QuickJS $QUICKJS_VERSION (MIT)"
  echo
  echo "FFmpeg configure arguments:"
  printf '  %s\n' "${FFMPEG_CONFIGURE[@]}"
} >"$OUT/BUILD-INFO.txt"

(
  cd "$OUT"
  shasum -a 256 ffmpeg ffprobe qjs BUILD-INFO.txt sources/* | sed 's#sources/##' >SHA256SUMS
)
"$ROOT/Scripts/test_media_tools.sh" "$OUT"
cat "$OUT/SHA256SUMS"
echo "Built media tools in $OUT"
