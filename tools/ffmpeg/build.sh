#!/usr/bin/env bash
# Only the dedicated FFmpeg workflow runs this recipe. Consumers fetch its release.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
source "$ROOT/tools/ffmpeg/sources.env"
ARCH=${1:?Usage: tools/ffmpeg/build.sh arm64|x64 output-directory}
mkdir -p "${2:?output-directory required}"
OUT=$(cd "$2" && pwd)
WORK="$ROOT/.build/ffmpeg/$ARCH"
case "$ARCH" in
  arm64) CPU=arm64; HOST=aarch64-apple-darwin ;;
  x64) CPU=x86_64; HOST=x86_64-apple-darwin ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac
export MACOSX_DEPLOYMENT_TARGET="$MACOS_MIN_VERSION"
SDK=$(xcrun --sdk macosx --show-sdk-path)
FLAGS="-arch $CPU -isysroot $SDK -mmacosx-version-min=$MACOS_MIN_VERSION"
mkdir -p "$WORK/downloads" "$WORK/prefix" "$OUT/sources" "$OUT/$ARCH"

fetch() {
  local url="$1" digest="$2" file="$WORK/downloads/$(basename "$1")"
  if ! shasum -a 256 -c <<<"$digest  $file" >/dev/null 2>&1; then
    curl -fsSL --retry 5 --retry-all-errors "$url" -o "$file.part"
    mv "$file.part" "$file"
  fi
  shasum -a 256 -c <<<"$digest  $file" >&2
  cp "$file" "$OUT/sources/"
  echo "$file"
}
tar -xf "$(fetch "$LAME_URL" "$LAME_SHA256")" -C "$WORK"
(
  cd "$WORK/lame-$LAME_VERSION"
  # The decoder in this release is GPL; only the LGPL encoder is built.
  CC="$(xcrun -f clang)" CPPFLAGS="-isysroot $SDK" CFLAGS="$FLAGS -Os" LDFLAGS="$FLAGS" ./configure \
    --prefix="$WORK/prefix" --host="$HOST" --disable-shared --enable-static \
    --disable-frontend --disable-decoder --disable-gtktest --enable-nasm=no
  make -C libmp3lame -j"$(sysctl -n hw.ncpu)"
  make -C libmp3lame install
  make -C include install
)
tar -xf "$(fetch "$FFMPEG_URL" "$FFMPEG_SHA256")" -C "$WORK"
CONFIGURE=(
  --prefix="$WORK/prefix" --arch="$CPU" --target-os=darwin --enable-cross-compile
  --cc="$(xcrun -f clang)" --sysroot="$SDK" --pkg-config=false
  --extra-cflags="$FLAGS -I$WORK/prefix/include"
  --extra-ldflags="$FLAGS -L$WORK/prefix/lib"
  --disable-shared --enable-static --disable-debug --disable-doc --enable-small
  --disable-autodetect --disable-everything --disable-gpl --disable-nonfree
  --disable-x86asm --disable-ffplay --disable-avdevice
  --enable-ffmpeg --enable-ffprobe
  --enable-avcodec --enable-avformat --enable-avfilter --enable-swscale --enable-swresample
  --enable-zlib --enable-securetransport --enable-libmp3lame
  --enable-muxer=mp4,mov,ipod,matroska,webm,mp3,srt,webvtt,ass,image2,mjpeg,mpegts
  --enable-demuxer=mov,matroska,mp3,aac,ogg,hls,mpegts,webvtt,srt,ass,image2,webp_pipe,png_pipe,jpeg_pipe,wav,data
  --enable-decoder=aac,mp3,mp3float,opus,vorbis,pcm_s16le,webvtt,srt,subrip,ass,ssa,movtext,webp,vp8,png,mjpeg
  --enable-encoder=libmp3lame,aac,mjpeg,srt,subrip,movtext,webvtt,ass,ssa
  --enable-parser=aac,h264,hevc,vp9,av1,opus,vorbis,mpegaudio,mjpeg,png,webp
  --enable-bsf=aac_adtstoasc,h264_mp4toannexb,hevc_mp4toannexb,vp9_superframe,extract_extradata,mjpeg2jpeg,setts,null
  --enable-filter=aresample,aformat,anull,scale,format,null,copy
  --enable-protocol=file,pipe,http,https,tls,tcp,crypto,data
)
(
  cd "$WORK/ffmpeg-$FFMPEG_VERSION"
  ./configure "${CONFIGURE[@]}" | tee "$OUT/$ARCH/configure.txt"
  grep -q '^License: LGPL version 2.1 or later' "$OUT/$ARCH/configure.txt"
  make -j"$(sysctl -n hw.ncpu)" ffmpeg ffprobe
  cp ffmpeg ffprobe "$OUT/$ARCH/"
)
cp "$WORK/ffmpeg-$FFMPEG_VERSION/COPYING.LGPLv2.1" "$OUT/$ARCH/FFmpeg-LICENSE.txt"
cp "$WORK/lame-$LAME_VERSION/COPYING" "$OUT/$ARCH/LAME-LICENSE.txt"
cp "$ROOT/tools/ffmpeg/build.sh" "$ROOT/tools/ffmpeg/sources.env" "$OUT/sources/"
{
  echo "FFmpeg $FFMPEG_VERSION and ffprobe, with LAME $LAME_VERSION. LGPL 2.1 or later."
  echo "Built for $CPU, macOS $MACOS_MIN_VERSION and later. No source modifications."
  echo "Exact source and build recipe: https://github.com/slovakian/opul/releases/tag/$FFMPEG_RELEASE"
  echo "FFmpeg source SHA-256: $FFMPEG_SHA256"
  echo "LAME source SHA-256: $LAME_SHA256"
  echo 'Configure arguments:'
  printf '  %s\n' "${CONFIGURE[@]}"
} > "$OUT/$ARCH/SOURCE.txt"
for tool in ffmpeg ffprobe; do
  strip -x "$OUT/$ARCH/$tool"
  codesign --force --sign - "$OUT/$ARCH/$tool"
done
(cd "$OUT/$ARCH" && shasum -a 256 ffmpeg ffprobe *LICENSE.txt SOURCE.txt configure.txt > SHA256SUMS)
"$ROOT/Scripts/verify_ffmpeg.sh" "$OUT/$ARCH" "$ARCH"
"$ROOT/Scripts/test_ffmpeg.sh" "$OUT/$ARCH"
tar -czf "$OUT/$FFMPEG_RELEASE-$ARCH.tar.gz" -C "$OUT/$ARCH" .
