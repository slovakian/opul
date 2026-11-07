# Nightly yt-dlp and compatibility monitoring, issue 68

Baseline `cdf1383` was fetched from `origin/integration/58-opul-managed-tools`. Parent #58 and its empty comment thread were read. The listed blocker #65 was closed and its implementation was present, alongside the scheduled updater from #67. The parent remains open and unchanged.

Both app channels default to stable yt-dlp. Opul Nightly exposes an immediate, persisted source preference; stable Opul rejects the command and refuses nightly selected/rollback state. Switching sources reuses the signed, health-checked update path, preserves running users, cancels obsolete jobs and allows a return to an older stable tag. Script directories, retained pairs and notification identities distinguish channels. Pending switches resume after restart. Nightly app versions use their base version when checking the runtime minimum.

The dedicated scheduled workflow tests the current signed runtime against stable yt-dlp every three days and nightly weekly, on arm64 and x64. An elapsed-day gate preserves the stable interval across month boundaries. A pre-release results ledger skips tested combinations; exact markers across all issue states prevent duplicate failure issues. Result publication uses `--prerelease --latest=false`. Normal CI runs only offline contracts. See [the operational documentation](../video-tools-compatibility.md).

## Regression and review

The first stable-command test failed before implementation because the snapshot lacked source availability/preference fields. The first Nightly browser test failed because its switch was absent. The prerelease app-version test reproduced rejection of a runtime requiring the same base app version. All pass after implementation.

Nine Rust source/channel scenarios cover defaults and gating, exact source assets, signature rejection, active-user safety, stable downgrade, immediate persistence, isolated preferences and clocks, setup cancellation, health pairing/new-runtime recovery, copied nightly state rejection and the prerelease version check. Browser scenarios verify switch visibility, persistence outside Settings Save and an axe audit.

Independent Standards and Spec reviews compared all implementation commits against `cdf1383`. Standards reported no findings. Spec found that executable startup errors and malformed archives escaped failure reporting. A new CLI-main regression reproduced four errors before the fix; all now create diagnostic failure issues/results and exit nonzero. The Spec reviewer confirmed the correction and reported no remaining findings.

The repository's pre-commit size limit required extracting source URLs/HTTP client configuration into `sources.rs`. Python bytecode caches are ignored. No product behavior changed in that extraction.

## Validation

Validation ran on Apple silicon macOS on 2026-11-08 local time.

- `vp install`, `vp check`, `vp exec tsc --noEmit`, `cargo check --workspace`, `cargo fmt --all --check`, strict core Clippy and diff checks passed.
- `RUST_TEST_THREADS=4 OPUL_TEST_PORT=1447 Scripts/test.sh` passed 22 desktop tests, 183 core tests, doc tests, all 38 frontend unit tests and the production frontend build. Its WebKit run passed 221 of 232 scenarios. Eleven unrelated icon/folder scenarios encountered navigation/time-out failures while packaging and formatting ran concurrently. Once those writers stopped, `OPUL_TEST_PORT=1447 vp run test --last-failed --workers=2` passed all eleven without code or assertion changes. All 232 scenarios therefore have passing results; this was not a single uninterrupted green browser run.
- `python3 tools/compatibility/test_check.py` passed all eight offline contract tests, including scheduling, skip identities, all-state issue deduplication, release flags, health requirements and broken executable/archive reporting. `actionlint .github/workflows/tools-compatibility.yml` passed.
- `Scripts/test_packaging.sh` passed. `MARKETING_VERSION=0.2.2-nightly.20261108.68 Scripts/package_app.sh debug nightly arm64` built a single-architecture Nightly app. `Scripts/smoke_test.sh 'Opul Nightly.app'` verified icons, frontend/Rust connectivity and bundled FFmpeg/ffprobe.
- `Scripts/smoke_video_tools.sh 'Opul Nightly.app'` used an isolated `OPUL_DATA_DIR`, fetched verified stable yt-dlp (the Nightly app default) with runtime 1, and downloaded the permitted trailer through the actual application. Output was 11,053,871 bytes; `Tools/` was excluded from Time Machine. [Packaged report](video-tools-68-packaged-nightly.json).
- The dedicated workflow's runner was executed locally with real published tools for stable `2026.10.26` and nightly `2026.11.04.232659`, each on arm64 and x64/Rosetta, against signed runtime 1. All four passed signatures/checksums, health checks and the CC BY 3.0 trailer download. [Recorded versions and diagnostic evidence](video-tools-68-real-tools.log).
- [Nightly Settings screenshot](../screenshots/settings-yt-dlp-nightly.png) was captured by the browser test and visually inspected.

An initial local GnuPG import attempted to create an agent socket below the long workspace path. The runner now dearmors the embedded public key directly for `gpgv`, avoiding an agent and the person's keyring; the real nightly/arm64 and both x64 runs passed with that implementation.

## Handoff and limits

GitHub cron schedules activate only after the workflow reaches the default branch. This ticket delivers and locally validates the workflow on the required integration branch; it does not change main or claim a scheduled GitHub run already happened. Result publication and issue creation were validated through offline boundary tests without creating test releases or failure issues. Four local integration runs used `--local`, which suppresses both mutations.

Intel execution used Rosetta, not physical Intel hardware. No macOS 13 hardware or notarization claim is made. #69 retains final integration and distribution verification. `OPUL_DATA_DIR` remains an exact data-directory override; use different directories for simultaneous stable and Nightly runs. The directory lock prevents shared ownership.
