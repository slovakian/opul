# First-launch Setup, issue 64

Baseline `2934561` on `integration/58-opul-managed-tools`. Blocker #61 was closed and present at this baseline, along with #62 and #63. Parent #58 and its comments were read before implementation. The parent remains open and unchanged.

## Behavior and evidence

Setup fills the app window through the existing dialog primitive, without changing macOS window mode. Its Trace list distinguishes direct files and torrents from optional video-page and audio downloads. The offer shows the module's published size, falling back to about 23 MB, and the immediate keep-up-to-date checkbox.

The real desktop preference store atomically claims the first offer and preserves that flag through stale Settings saves and process restart. Not now, Esc and Close dismiss without downloading. Don't show this again runs the existing Turn off operation and dismisses only after success. Already Ready or Turned off installations receive no automatic offer. Explicit Help and Settings entry points still open Setup.

Download and Try again call the existing module. Setup and Settings render the same Python and yt-dlp steps, statuses, hints, progress and finished checkmarks from module snapshots. Continue to Opul closes the view while installation continues. The sidebar presents aggregate step progress and reopens the view. Ready removes the sidebar line and emits one completion toast; repeated snapshots and reopening an already Ready app do not repeat it.

The first browser regression failed before implementation because Setup was absent. The desktop preference regression initially failed to compile against the missing field and command, then passed after implementing persistence. Mock-desktop WebKit tests cover each dismissal, consent, exact/fallback size, repeat launch, Help and Settings re-entry, pushed steps, offline waiting, verification, installation, failure/retry, Continue, sidebar re-entry and completion. Direct-file and magnet submissions remain usable during setup and reach the existing Direct and Torrent commands. Axe audits cover the offer and progress views.

[Setup progress](../screenshots/video-setup-progress.png)

## Review

The code-review skill ran independent Standards and Spec reviews against the baseline. The Spec review found no gaps. Standards identified a feature-list color override and custom alert markup. Trace now owns the available-feature visual state, and Setup uses Alert/AlertDescription. Follow-up review found no remaining findings.

The repository's commit hook limits text files to 20,000 bytes. The test preference merge and first-offer behavior were placed in the existing preference fixture module to keep the desktop fixture within that limit while preserving its live-preference semantics.

## Boundaries

The video-tools module still owns installation, verification, cancellation and durable tool state. No second installer, automatic updater or removal lifecycle was added. These UI scenarios use the established mock-desktop seam; the existing signed-runtime verification and real-content smoke evidence from #61/#62 remains applicable. This ticket does not claim new Intel or macOS 13 hardware testing.

## Validation

On Apple silicon macOS, 2026-11-07:

- `vp install`, `vp check`, `vp exec tsc --noEmit`, and `cargo fmt --all --check` passed.
- `RUST_TEST_THREADS=4 Scripts/test.sh` passed 22 desktop tests, 141 core tests, 38 frontend unit tests, the production frontend build and all 223 WebKit tests. The ten new Setup scenarios passed, including offer/progress axe audits and direct-file/torrent submission during installation. The final shared Trace variant and Alert changes were present before the browser scenarios ran.
- `cargo clippy -p opul_core --all-targets -- -D warnings` passed. The optional broader `cargo clippy --workspace --all-targets -- -D warnings` reports existing desktop warnings: long functions in `commands.rs` and `main.rs`, a missing statement semicolon in `commands/tests.rs`, an empty-collection assertion in `desktop_tests.rs`, and default-field reassignment in an older preference test. Those existing responsibilities were left outside this UI ticket.
- `Scripts/test_packaging.sh` passed. `Scripts/package_app.sh debug` built the arm64 stable bundle. `Scripts/smoke_test.sh` passed its icon checks, loaded the packaged frontend, connected it to Rust and resolved built-in FFmpeg/ffprobe. Reading its isolated `desktop-preferences.json` confirmed `video_tools_setup_shown: true` after the real native launch.
- Standards and Spec review follow-ups have no remaining findings. The staged diff and repository token-limit check pass.

The interaction test's initial toast selector expected a nonexistent workspace name; the fixture's visible added-download row and accepted Direct/Torrent commands now verify the intended behavior. No product behavior or existing assertions were weakened for that correction. No new networked installation smoke was needed because this ticket reuses the existing installer unchanged.
