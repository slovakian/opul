# Verified video tools, issue 61

The implementation begins at integration commit `2d438a5f92d0b3e8ccc5f03aad34d5e744d7db7a`. Feature commit `de4dcaf` implements Settings setup and the video-tools public seam. Follow-up commits address review evidence and record final validation. See [the public contract and sibling handoff](../video-tools.md).

## Packaged runtime evidence

Host: Apple silicon macOS. Build command:

```sh
MACOSX_DEPLOYMENT_TARGET=13.0 vp exec tauri build --debug --bundles app --config '{"version":"0.2.2"}'
codesign --force --sign - target/debug/bundle/macos/Opul.app
codesign --verify --deep --strict target/debug/bundle/macos/Opul.app
Scripts/smoke_video_tools.sh target/debug/bundle/macos/Opul.app
```

The harness launches the actual packaged application with fresh, isolated `OPUL_DATA_DIR` and `OPUL_DOWNLOAD_DIR` paths. It does not use `OPUL_YT_DLP` or preinstalled tools. The app resolves public HTTPS release URLs, verifies both projects' signatures, verifies checksums, unpacks and health-checks the runtime/script, commits the pair, then uses the ordinary application and MediaEngine commands to probe and download the W3C permitted-content sample.

Observed successful result:

- Python runtime 1; Python 3.13.18; yt-dlp 2026.10.26.
- Built-in FFmpeg and ffprobe 9.0.2.
- `trailer.mp4`, 11,053,871 bytes, Finished.
- Time Machine reports `[Excluded]` for the actual Tools directory.

Runtime 1's signed pointer requires Opul 0.2.2 or later. This is why the packaged test uses an explicit 0.2.2 build configuration. An older development version correctly rejects this runtime. The normal release versioning process remains unchanged. The app was packaged in debug mode, sealed with a local ad-hoc signature, and passed deep/strict code-signature validation. It was not published or notarized by this task. This smoke covers arm64 on the current host; it does not claim a physical macOS 13 or Intel-app test. Both published runtime architectures were independently verified by #60.

## Validation on 2026-11-07

- `vp install` after fetching the integration head; `vp check`; `tsc --noEmit`; `cargo fmt --all --check` passed.
- The full `Scripts/test.sh` run passed 20 Tauri tests, 113 core tests, 37 frontend unit tests and the production frontend build. The later activation-failure case adds one core test. The final focused `cargo test -p opul_core video_tools` run passed all 10 module tests, including that new case and explicit curl impersonation in MediaEngine's arguments.
- The complete WebKit run passed 194 of 203 tests initially. Five failures reported exhausted disk space and four were interaction timeouts while native packaging was rebuilding. After `cargo clean -p opul_core` reclaimed 2.7 GiB of this task's rebuildable cache, all nine failed cases passed with `--last-failed --workers=1`. The final focused tools/built-in suite also passed all nine cases, including axe audits. No application change was needed for those existing UI cases.
- Core Clippy passed with `--all-targets -- -D warnings`. A strict workspace invocation also reported five pre-existing Tauri lint warnings, including long dispatch/main functions and existing test style. These unrelated warnings were left unchanged.
- `Scripts/test_packaging.sh`, `Scripts/test_ffmpeg.sh` and `Scripts/test_ffmpeg_fetch.sh` passed. The FFmpeg suite checked the five supported jobs; fetch tests rejected corruption and universal targets. Packaged FFmpeg checksums, native signatures, architecture and deployment targets passed `Scripts/verify_ffmpeg.sh ... arm64`.
- `Scripts/smoke_video_tools.sh` passed with the final invocation and asserted Time Machine exclusion. `Scripts/smoke_test.sh` passed packaged frontend/Rust connection, built-in tool resolution and all icon checks. `codesign --verify --deep --strict` passed after sealing the debug app.

A repeated network smoke initially hit the sample host's anti-bot challenge after successful verified installation. The final managed invocation explicitly selects Chrome impersonation through bundled curl_cffi, matching the published runtime's own smoke recipe. Its command arguments are fixture-tested, a fresh-context spec review found no issue with the adjustment, and the final packaged download passed. The public sample host remains an external integration dependency.

[Settings setup progress screenshot](../screenshots/settings-video-tools-progress.png)

## Review

Separate fresh-context Standards and Spec reviews found no implementation defects or scope creep. Standards reviewed AGENTS.md and CONTRIBUTING.md; Spec reviewed #61 and the relevant #58 contract, with sibling-ticket boundaries explicit.

Two evidence gaps were addressed:

1. `tmutil isexcluded` exits successfully for included paths too. The packaged smoke now asserts the printed `[Excluded]` status.
2. The original cancellation test stopped before activation. A new public-interface test forces a filesystem error during script placement after verified runtime placement. It asserts the committed state.json remains byte-for-byte unchanged and no invocation is exposed, then retries and verifies the complete pair. A focused second review confirmed this closes the #61 evidence gap.

## File-boundary changes

The repository's existing `Scripts/check_token_limit.sh` rejects every staged source file above 20 KB, including existing large files. The first commit attempt exposed this. To satisfy that guard without disabling it, legacy `src/media.rs` and `src/application/ffi.rs` were mechanically split by responsibility. Their types, engine operations, projection, adapter handling and tests retain their behavior and module-private access. The new Tauri tools commands, UI tools fixture and Rust HTTP fixture also have separate files.

For the substantive media change, inspect `src/media/types.rs`, `open.rs`, `context.rs`, `probe.rs` and `workers.rs`, plus `src/video_tools/`. The application integration is in `src/application/ffi/{types,workspaces,commands}.rs`. Most other moved lines predate #61 and can be compared with the baseline files at `2d438a5`.
