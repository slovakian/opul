# Packaged video-tools acceptance

Run this against copied release apps, with production tool sources and an isolated library. The driver clicks the packaged React dialogs through WKWebView, uses the public application state machine for Downloads, and closes the final window normally. It verifies the Stopped checkpoint and compares Download IDs, destinations, formats and completed-file hashes on each relaunch.

The local fixture serves generated CC0 videos, a two-entry HTML5 playlist, direct files, a BitTorrent tracker and a real peer. The peer exchanges actual pieces with librqbit. The slow transfer uses the Blender Foundation's Big Buck Bunny trailer under [CC BY 3.0](https://peach.blender.org/about/). Removal retains its partial bytes, and re-setup must issue a positive HTTP Range and finish with exactly the original bytes.

## Prepare an authenticated historical pair

Manual and automatic updates need an older installed version. Reconstruct runtime 1 from its immutable release, without changing the production pointer or trust configuration. The helper verifies the manifest and runtime archive with Opul's checked-in public key, then verifies yt-dlp's upstream signed checksums with the checked-in upstream key. It preserves archive symlinks and runs the resulting interpreter and script.

Use Python 3.13, OpenSSL 3, `gpg` and `gpgv`. On Apple silicon, Intel execution also requires Rosetta. Choose fresh output paths:

```sh
python3.13 tools/prepare_journey_baseline.py arm64 .build/journey-baseline-arm64
python3.13 tools/prepare_journey_baseline.py x64 .build/journey-baseline-x64
curl --fail --location https://media.w3.org/2010/05/bunny/trailer.mp4 \
  --output .build/journey-trailer.mp4
```

## Execute each package

```sh
python3.13 tools/packaged_video_journey.py Opul.app \
  --output .build/journey-arm64 \
  --baseline-tools .build/journey-baseline-arm64/Tools \
  --slow-media .build/journey-trailer.mp4

python3.13 tools/packaged_video_journey.py 'Opul Nightly.app' \
  --output .build/journey-x64 \
  --baseline-tools .build/journey-baseline-x64/Tools \
  --slow-media .build/journey-trailer.mp4
```

The expected current runtime defaults to 2; use `--runtime` after a subsequent numbered release. `--scenario lifecycle`, `manual` or `automatic` limits a diagnostic rerun. Keep each run's directory: per-launch reports contain actual tools snapshots, UI receipts, transfer identities, hashes and Time Machine exclusion; `fixture-traffic.json` records peer traffic and Range requests. `result.json` is emitted only when every selected scenario succeeds.

The lifecycle checks Not now, Don't show this again, waiting single video and playlist across restart, consented Setup, playlist expansion, removal confirmation and cancellation, partial retention, re-setup and restart. Direct HTTP and BitTorrent transfers run before consent, while Turned off, during actual Setup, while video is running, after removal, after re-setup and after restart.

The manual path checks availability without installation, explicitly updates runtime 1 to 2, and injects an ordinary local HTTP 404 into the first video Download. That failure exercises the real pre-success rollback to runtime 1. A subsequent explicit update and successful video Download must prune the old runtime. The automatic path enables the actual Settings switch and waits for the production scheduler; it sends no manual check/update command and does not alter the clock. Both paths verify independent transfers during the actual Updating state, after activation, and after restart. The upstream error is retained verbatim in the report.

Run without concurrent builds, formatting or browser suites, and retain enough free space for packages, traces and temporary tool copies. A timeout is a failed run; retain its report and diagnose or rerun explicitly. Do not weaken assertions to turn an incomplete run green.

## Minimum macOS verification

Package inspection checks architecture slices, signatures and deployment targets. Execute this same journey and the built-in tool checks on macOS 13 to verify the advertised minimum; execution on a newer host does not satisfy that gate. Record `sw_vers`, hardware/translation details, app and installer hashes, and reports beside the acceptance evidence. [Issue 69](evidence/video-tools-69.md) records the verified hosts and any remaining distribution requirement.
