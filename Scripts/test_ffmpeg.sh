#!/usr/bin/env bash
# Every input was generated for Opul and dedicated to CC0; no network media.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TOOLS=$(cd "${1:?tools-directory required}" && pwd)
FIXTURES="$ROOT/tools/ffmpeg/fixtures"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
FF="$TOOLS/ffmpeg"
PROBE="$TOOLS/ffprobe"
# On Apple silicon this executes an Intel binary through Rosetta. Fail if unavailable.
"$FF" -version
"$PROBE" -version
"$FF" -L | grep 'GNU Lesser General Public' > /dev/null
"$FF" -y -i "$FIXTURES/tone.wav" -c:a aac "$WORK/audio.m4a"
"$FF" -y -i "$FIXTURES/video.mp4" -i "$WORK/audio.m4a" -map 0:v -map 1:a -c copy "$WORK/merged.mp4"
"$FF" -y -i "$WORK/audio.m4a" -c:a libmp3lame "$WORK/audio.mp3"
"$FF" -y -i "$FIXTURES/captions.vtt" "$WORK/captions.srt"
"$FF" -y -i "$WORK/merged.mp4" -i "$WORK/captions.srt" -map 0 -map 1 -c copy "$WORK/subtitles.mkv"
"$FF" -y -i "$WORK/merged.mp4" -i "$WORK/captions.srt" -map 0 -map 1 -c copy -c:s mov_text "$WORK/subtitles.mp4"
for format in png webp; do
  "$FF" -y -i "$FIXTURES/thumbnail.$format" -frames:v 1 "$WORK/$format.jpg"
done
python3 - "$PROBE" "$WORK" <<'PY'
import json, pathlib, subprocess, sys
probe, work = sys.argv[1], pathlib.Path(sys.argv[2])
expected = {
    'merged.mp4': {'h264', 'aac'}, 'audio.mp3': {'mp3'}, 'audio.m4a': {'aac'},
    'captions.srt': {'subrip'}, 'subtitles.mkv': {'h264', 'aac', 'subrip'},
    'subtitles.mp4': {'h264', 'aac', 'mov_text'}, 'png.jpg': {'mjpeg'}, 'webp.jpg': {'mjpeg'},
}
for name, codecs in expected.items():
    result = json.loads(subprocess.check_output([probe, '-v', 'error', '-show_streams', '-of', 'json', str(work / name)]))
    actual = {stream['codec_name'] for stream in result['streams']}
    assert actual == codecs, (name, actual, codecs)
    assert (work / name).stat().st_size > 0
    print(f'PASS {name}: {sorted(actual)}')
assert 'Opul test caption' in (work / 'captions.srt').read_text()
PY
