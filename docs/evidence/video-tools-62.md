# Video-tools recovery, issue 62

Baseline: `f90db49` on `integration/58-opul-managed-tools`. Prerequisite #61 is closed and its implementation is present at this baseline. Parent #58 and its comments were read before implementation. The parent spec remains open and unchanged.

## Behavior and evidence

The public module now recovers interrupted payload and metadata bodies with validated HTTP byte ranges, retries transient failures after 2, 10 and 30 seconds, and waits offline without spending attempts. A fake reachability source and clock drive the real local fixture HTTP server, test signing keys, fixture interpreters and temporary Tools root. Every scenario operates through module commands, snapshots, diagnostics, invocation and on-disk state.

Two added regression cases were run against the original production code. The interrupted runtime case failed with `NotWorking { reason: Download, details: "error decoding response body" }`; startup pruning failed because `python-runtime/999` survived. Both pass with this implementation.

Coverage includes runtime, script and metadata resume offsets; servers ignoring Range; invalid Content-Range rejection; transient HTTP exhaustion; permanent HTTP failures; offline setup and reconnection; offline backoff without losing the retry budget; offline cancellation; one-time corruption that recovers; repeated signature failures; a correctly signed runtime with a mismatched signed checksum; missing, non-executable and OS-rejected interpreters; explicit version retention and orphan pruning. Existing module cases also prove verification happens before execution and a failed placement cannot change committed state.json.

The crash test launches a second test process, waits until its runtime is unpacked and its script fetch has begun, then kills it. The parent verifies byte-for-byte committed state preservation, observes the abandoned staging directory, reopens the module, checks cleanup and the lack of an invocation/network requests, then explicitly retries to a complete Ready pair. This tests termination without Drop handlers. Cancellation of a Ready installation also preserves its invocation and state.json.

WebKit tests exercise offline progress, automatic rendering of resumed progress, cancellation, all three recovery messages, Try again, and clipboard content containing the specific error, versions and chip. Clipboard writes use a recording browser API fixture; the actual Settings component requests diagnostics through the desktop bridge. Accessibility audits cover the offline screen and existing tools states.

[Settings waiting for connectivity](../screenshots/settings-video-tools-offline.png)

## Validation

On Apple silicon macOS, 2026-11-07:

- `vp install`, `vp check`, `vp exec tsc --noEmit` and `cargo fmt --all --check` passed.
- `RUST_TEST_THREADS=4 Scripts/test.sh` passed all 20 desktop tests, 129 core tests, 37 frontend unit tests, the production frontend build and all 207 WebKit tests. The core run includes 25 video-tools tests.
- The initial unrestricted full Rust run hit the existing five-second media fixture deadlines in `built_in_tools_enable_audio_formats_and_require_ffprobe` and `guided_hls_and_dash_reach_the_scoped_yt_dlp_handoff`. Both passed in the complete rerun with four Rust test threads. Their tests and production paths were unchanged.
- `cargo clippy -p opul_core --all-targets -- -D warnings` passed.
- `Scripts/test_packaging.sh` passed. `Scripts/smoke_test.sh target/debug/bundle/macos/Opul.app` passed packaged frontend/Rust connection, built-in tool resolution and native icon checks.
- The native debug app built successfully with `MACOSX_DEPLOYMENT_TARGET=13.0 vp exec tauri build --debug --bundles app --config '{"version":"0.2.2"}'`. Version 0.2.2 satisfies the published runtime's existing minimum. Local ad-hoc signing and `codesign --verify --deep --strict` passed. This does not change the release version or publish an app.

`Scripts/smoke_video_tools.sh target/debug/bundle/macos/Opul.app` passed with a fresh isolated data directory and no developer tool override. It downloaded and verified the published runtime 1 and yt-dlp 2026.10.26, activated Python 3.13.18, and used the ordinary application state machine and MediaEngine to download the permitted W3C trailer, 11,053,871 bytes. Built-in FFmpeg and ffprobe reported 9.0.2. The actual Tools directory was excluded from Time Machine. This check ran on the current Apple silicon host, not physical Intel or macOS 13 hardware.

The fresh-context standards review found two lifecycle issues: offline optional metadata lookup held the Tools lock, and cancelled metadata resolution could overwrite a newer candidate in diagnostics. Both are fixed. The first has a public-operation regression proving root release; candidate mutation now checks the existing operation identity. Focused follow-up review confirmed both fixes in the staged diff. The separate spec review found no outstanding gaps after the fixes.

## Boundaries and handoff

- Resume retains prefixes during the active operation. Restart intentionally cleans temporary downloads and waits for fresh consent, as required by the startup cleanup policy.
- The default reachability source uses macOS SystemConfiguration via scutil. Fixture tests inject reachability and time; they do not alter the host's connection.
- #65 owns update selection and predecessor lifetime. Its persisted references are `previous_runtime` and `previous_yt_dlp`; startup now preserves those exact directories and prunes older/unselected versions. No updater was added here.
- #64 should render `waiting_for_internet` through the shared `VideoToolsProgress` component.
- The OS-blocked fixture exits with status 126. This verifies error handling and recovery; it does not simulate Apple's trust service or claim notarization.
