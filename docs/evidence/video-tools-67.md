# Scheduled video tools updates, issue 67

Baseline `15ac7b677332db5179315279e417e8983e168db6`, fetched from `origin/integration/58-opul-managed-tools`. Parent #58 and its comments were read; completed blockers #61 through #66 were present. The parent remains open and unchanged.

Ready now checks after one minute and every 24 hours. One immediately persisted preference covers the runtime and yt-dlp. Automatic checks resolve metadata once and reuse the verified manual installer, compatible-pair health checks, active-user deferral, cancellation and rollback. Switching automatic installation off revokes a pending automatic install. Checks continue without installing, and metadata failures remain silent until the next daily attempt. Not set up, Turned off, Setting up and Not working do not arm update-check timers.

The tools store retains availability and atomically claims notification identities before display. Claims survive restart and removal. Ordinary installation failures retain the available marker and Update action; compatibility holds and skipped integrity failures do not offer an ineligible update. Progress stays in Settings. The application actor projects a live update capability for failed managed video Downloads when automatic installation is off. List and details add advice alongside the unchanged upstream error; direct-transfer failures receive no advice.

## Regression and review evidence

The first fake-clock public-interface test failed before implementation because no scheduled work ran. It passes with checks at 60 seconds and 86,400 seconds thereafter. Six scheduler scenarios use signed HTTP fixtures and a controlled clock to verify cadence, silent failures, consent-only checks, durable claims, preference changes, daily install retries, ineligible states, removal and shutdown. Existing updater tests continue to verify signatures, compatibility, activity deferral, atomic activation, cancellation and rollback. A Download projection regression checks the original error, failed-video capability and exclusion of finished and direct Downloads.

Four new mock-desktop WebKit scenarios cover notification Update, repeat launch, later versions, Settings markers, progress confined to Settings and failed-download advice. Axe audits cover notifications, Settings and the advice/error panel. The existing Alert and Badge components provide semantic colors and accessible markup. A selected-list metadata contrast issue predates this work, so the failed-download audit targets the changed advice and inspector; notifications and Settings receive their full relevant audits.

Independent code-review Standards and Spec agents reviewed the staged diff against the baseline. Both reported zero remaining findings. The Standards follow-up verified the final Alert/Badge corrections. The pre-commit token limit required extracting the existing actor core/tests, fake network clock, removal operation and preference handler into focused modules. A further review checked the moves and visibility changes and reported no findings; final workspace tests and strict core Clippy passed again after extraction.

- [Consent notification](../screenshots/video-update-notification.png)
- [Settings availability](../screenshots/video-update-available.png)
- [Settings installation progress](../screenshots/video-update-progress.png)
- [Original error and update advice](../screenshots/video-update-advice.png)

## Validation

Validation ran on Apple silicon macOS on 2026-11-08.

- `vp install`, `vp check`, `vp exec tsc --noEmit`, `cargo fmt --all --check`, strict core Clippy and diff checks passed.
- The complete `RUST_TEST_THREADS=4 OPUL_TEST_PORT=1437 Scripts/test.sh` rerun passed 22 desktop tests, 174 core tests, doc tests, 38 frontend unit tests, the production frontend build and all 230 WebKit scenarios. This includes all six fake-clock cases and all four new browser scenarios, with axe audits.
- `Scripts/test_packaging.sh` passed. `Scripts/package_app.sh debug` built the arm64 stable app. `Scripts/smoke_test.sh` passed icon checks and confirmed the packaged frontend rendered, connected to Rust and resolved bundled FFmpeg/ffprobe.
- Screenshots above were captured by the final mock-desktop scenarios and visually inspected.

An initial unconstrained tools-only run exhausted a short wall-time allowance while many signed fixtures ran together. The eventual-observation allowance now matches the existing fixture's ten-second allowance; the final four-thread workspace run passed. An initial full/package attempt ran out of local disk space and the compiler aborted. `cargo clean -p opul_core -p Opul` reclaimed generated build artifacts, `vp env doctor` passed, and the complete rerun and package/smoke checks above passed. No product assertion was weakened to bypass a failure.

## Limits and handoff

Notification claims favor at-most-once display. A crash between the durable claim and rendering can lose that toast; Settings retains availability and Update. Notification history names the target runtime/script pair, including runtime-only updates, and remains in the channel's tools store. These tests use generated signing keys, fixture interpreters and a mocked desktop bridge. They make no new real-release, Intel execution or macOS 13 hardware claim.

#68 owns the yt-dlp source preference and compatibility workflow. Its source identity must distinguish stable and nightly versions in persisted notification identities and availability. #69 owns final cross-architecture, minimum-OS and complete real-tools journey verification. The channel-local scheduler, `NetworkEnvironment` clock, `take_update_notice`, snapshot `update_available` flag and Download `can_update_video_tools` capability are the integration points. See [update behavior](../video-tool-updates.md).
