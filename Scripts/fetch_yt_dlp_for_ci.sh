#!/usr/bin/env bash
# Download the latest stable yt-dlp_macos.zip into a folder for the Media Tools workflow's tests,
# checked against the release's SHA2-256SUMS. The app itself downloads and verifies yt-dlp in Rust.
set -euo pipefail

OUT=${1:?usage: fetch_yt_dlp_for_ci.sh <folder>}
location=$(curl -fsSI https://github.com/yt-dlp/yt-dlp/releases/latest | tr -d '\r' | sed -n 's/^location: //Ip')
tag=${location##*/}
base="https://github.com/yt-dlp/yt-dlp/releases/download/$tag"
mkdir -p "$OUT"
curl -fsSL --retry 5 --retry-all-errors -o "$OUT/SHA2-256SUMS" "$base/SHA2-256SUMS"
curl -fsSL --retry 5 --retry-all-errors -o "$OUT/yt-dlp_macos.zip" "$base/yt-dlp_macos.zip"
(cd "$OUT" && grep ' yt-dlp_macos.zip$' SHA2-256SUMS | shasum -a 256 -c -)
/usr/bin/ditto -x -k "$OUT/yt-dlp_macos.zip" "$OUT"
"$OUT/yt-dlp_macos" --version
