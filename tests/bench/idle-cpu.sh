#!/usr/bin/env bash
# Measures the idle CPU and memory of a running Opul build and its WebContent process.
# Usage: tests/bench/idle-cpu.sh <binary> <seed-dir> <label> [warmup-seconds] [window-seconds]
set -euo pipefail
bin=$1; seed=$2; label=$3; warmup=${4:-20}; window=${5:-60}
work="${TMPDIR:-/tmp}/opul-idle-$label"
rm -rf "$work"; cp -R "$seed" "$work"; rm -f "$work/application-state.lock"
cpu() { ps -o time= -p "$1" 2>/dev/null | python3 -c '
import sys
t=sys.stdin.read().strip()
if not t: print(0); sys.exit()
parts=t.replace("-",":").split(":")
s=0.0
for p in parts: s=s*60+float(p)
print(s)'; }
rss() { ps -o rss= -p "$1" 2>/dev/null | tr -d " " || echo 0; }
before=$(pgrep -f com.apple.WebKit.WebContent | sort || true)
OPUL_DATA_DIR="$work" OPUL_DOWNLOAD_DIR="$work/downloads" "$bin" >"$work.log" 2>&1 &
app=$!
sleep "$warmup"
web=$(comm -13 <(echo "$before") <(pgrep -f com.apple.WebKit.WebContent | sort) | tr '\n' ' ')
start_app=$(cpu $app); start_web=0
for p in $web; do start_web=$(python3 -c "print($start_web+$(cpu $p))"); done
sleep "$window"
end_app=$(cpu $app); end_web=0; web_rss=0
for p in $web; do end_web=$(python3 -c "print($end_web+$(cpu $p))"); web_rss=$((web_rss+$(rss $p))); done
app_rss=$(rss $app)
python3 - <<PY
w=$window
a=$end_app-$start_app; b=$end_web-$start_web
print(f"$label: app {a:.2f}s CPU ({100*a/w:.1f}%), webcontent [{'$web'.strip()}] {b:.2f}s CPU ({100*b/w:.1f}%), total {100*(a+b)/w:.1f}% of a core; RSS app {$app_rss/1024:.0f} MB, webcontent {$web_rss/1024:.0f} MB")
PY
kill $app 2>/dev/null || true; wait $app 2>/dev/null || true
