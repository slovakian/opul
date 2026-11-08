# Manual video tools updates, issue 65

Baseline `4ce16b0` on `integration/58-opul-managed-tools`. Blocker #61 was closed and present along with #62, #63 and #64. Parent #58 and all comments were read before implementation. The parent remains open and unchanged.

Settings now has working Check for updates and Update actions. Ready and the installed versions stay visible throughout preparation, with one exclusive status line for availability, progress, failure or a compatibility hold. The module persists successful check timestamps, holds, component integrity skips and the complete rollback pair. Setup and updates share verification, retry and health-check code.

MediaEngine acquires the selected pair at execution rather than retaining an enqueue-time version. Probes retain their pair through the same usage guard, without reporting a Download success. Acquisition, idle installation and activation share the module lock. Managed video Downloads report completion, while direct transfers, overrides and cancelled or paused work do not affect runtime probation. A new runtime retains the last working pair until its first successful Download. Failures before success, including failed startup validation, restore that pair. File cleanup waits for active users to drain. Shutdown calls the same cancellation method available to Remove.

## Regression evidence

The first Rust update test failed before implementation because `check_for_updates` was an unknown operation. It passes after implementing checks and script-only updates. The first browser interaction test failed waiting for the missing Check for updates button. It now passes through check, availability, Update, live progress and completion, and includes an axe audit. The restart rollback regression initially returned no invocation when the new interpreter was missing; it now restores the old complete invocation.

Signed fixture-server tests exercise script-only, runtime-only and combined updates, runtime-before-script preparation, installed-runtime and newer-runtime health checks, each compatibility rejection, durable holds and rechecks, old-pair execution, active-download deferral, script rotation, runtime retention, rollback, cancellation, transient retries and twice-failed integrity skips. Real MediaEngine worker tests verify activity and both completion outcomes. A killed subprocess verifies that a crash after runtime unpacking and before script completion leaves the committed old pair usable at restart, with abandoned staging cleaned up. A filesystem failure after runtime placement verifies that the old selection remains atomic when script placement fails.

## Handoff

- #67 can use CheckForUpdates for metadata, then Update when automatic updates are enabled. Operations coalesce during a job and do nothing outside Ready. Update retries ordinary failures; skips and compatibility holds suppress ineligible candidates. The scheduler and notifications remain #67's responsibility.
- #66 should call `VideoTools::cancel()`, stop and drain media users, then remove files. Cancellation stops setup, update checks, preparation, idle waits and health processes without changing the selected pair. Removal must preserve finished Download files.
- #68 still owns source switching and scheduled compatibility checks. This ticket uses the stable source and the existing per-channel roots.

## Review

The code-review skill ran independent Standards and Spec agents against the baseline. Standards found an unleased-probe lifetime hazard and suggested clearer transaction boundaries. All executing managed users now hold a usage guard, and preparation, candidate selection and activation are separate functions. Setup and updates share pair placement. Spec found runtime minimum-app gating could block an independently compatible script, and extraction I/O errors could incorrectly persist an integrity skip. Runtime eligibility is now independent, holds recheck after an app upgrade, and skips require an exhausted signature/checksum retry. Public-seam regression tests cover each correction, including an unchanged signed pointer after upgrading Opul. Follow-up review found a rollback race during updater health checks and transient process failures being treated as incompatibility. Updater health checks now retain their pair and reject a changed selection before persisting a hold; execution failures have a separate typed error and remain retryable. Both have additional public-seam regressions. Final follow-up reports have zero remaining Standards findings and zero remaining Spec findings.

[Settings during an update](../screenshots/video-tools-update.png)

## Validation

On Apple silicon macOS, 2026-11-08:

- `vp install`, `vp check`, `vp exec tsc --noEmit`, `cargo fmt --all --check`, `git diff --check`, and `cargo clippy -p opul_core --all-targets -- -D warnings` passed.
- The complete `RUST_TEST_THREADS=4 OPUL_TEST_PORT=1433 Scripts/test.sh` run passed 22 desktop tests, 151 core tests, 38 frontend unit tests and the production frontend build. WebKit passed 221 of 224 scenarios, including all video-tools scenarios. Three unrelated existing emoji-picker focus scenarios failed in the parallel run. `vp run test tests/ui/skin-tones-recents.spec.ts --workers=1` then passed all six scenarios in that file, including those three failures, without product or assertion changes.
- After the initial review corrections, `cargo test -p opul_core video_tools -- --nocapture` passed all 42 then-selected tests. This includes five new regressions for probe lifetime, independent runtime eligibility, app-upgrade hold rechecks, retryable extraction failure and persisted runtime integrity skips. The two final health regressions bring the core total to 158. All seven focused review regressions pass, including retrying identical published versions after an execution failure and rolling back during an active health check. Strict core Clippy, formatting and type checks passed again on the corrected implementation.
- Final `RUST_TEST_THREADS=4 cargo test --workspace` passed all 22 desktop tests and all 158 core tests, plus doc tests, after the health corrections and placement-failure harness update.
- The focused `vp run test tests/ui/video-tools.spec.ts` passed all 13 scenarios with Ready's exclusive status flags, the manual actions and axe audits. The screenshot above was captured through those same mock-bridge interactions and visually inspected.
- `Scripts/test_packaging.sh` passed. `Scripts/package_app.sh debug` built and ad-hoc signed the arm64 stable app. `Scripts/smoke_test.sh` passed its icon checks and confirmed the packaged frontend rendered, connected to Rust, and resolved built-in FFmpeg and ffprobe.

These tests use local signed fixtures and a mock desktop bridge. They do not claim new real-release compatibility, Intel execution or macOS 13 hardware coverage. The parallel emoji focus failures are documented rather than hidden by weakening their assertions. The final Rust run exposed a test-injection mismatch: health-lease cleanup correctly removed the dummy file used to block script placement. The test now makes the script parent read-only, which exercises a real placement permission failure while leaving the old files readable, and explicitly asserts the failed flag as well as the unchanged selection. Its focused rerun passed.
