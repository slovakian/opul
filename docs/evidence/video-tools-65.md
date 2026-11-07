# Manual video tools updates, issue 65

Baseline `4ce16b0` on `integration/58-opul-managed-tools`. Blocker #61 was closed and present along with #62, #63 and #64. Parent #58 and all comments were read before implementation. The parent remains open and unchanged.

Settings now has working Check for updates and Update actions. Ready and the installed versions stay visible throughout preparation, with one exclusive status line for availability, progress, failure or a compatibility hold. The module persists successful check timestamps, holds, component integrity skips and the complete rollback pair. Setup and updates share verification, retry and health-check code.

MediaEngine acquires the selected pair at execution rather than retaining an enqueue-time version. Acquisition, idle installation and activation share the module lock. Managed video Downloads report completion, while direct transfers, overrides and cancelled or paused work do not affect runtime probation. A new runtime retains the last working pair until its first successful Download. Failures before success, including failed startup validation, restore that pair. File cleanup waits for active users to drain. Shutdown calls the same cancellation method available to Remove.

## Regression evidence

The first Rust update test failed before implementation because `check_for_updates` was an unknown operation. It passes after implementing checks and script-only updates. The first browser interaction test failed waiting for the missing Check for updates button. It now passes through check, availability, Update, live progress and completion, and includes an axe audit. The restart rollback regression initially returned no invocation when the new interpreter was missing; it now restores the old complete invocation.

Signed fixture-server tests exercise script-only, runtime-only and combined updates, runtime-before-script preparation, installed-runtime and newer-runtime health checks, each compatibility rejection, durable holds and rechecks, old-pair execution, active-download deferral, script rotation, runtime retention, rollback, cancellation, transient retries and twice-failed integrity skips. Real MediaEngine worker tests verify activity and both completion outcomes. A killed subprocess verifies that a crash after runtime unpacking and before script completion leaves the committed old pair usable at restart, with abandoned staging cleaned up. A filesystem failure after runtime placement verifies that the old selection remains atomic when script placement fails.

## Handoff

- #67 can use CheckForUpdates for metadata, then Update when automatic updates are enabled. Operations coalesce during a job and do nothing outside Ready. Update retries ordinary failures; skips and compatibility holds suppress ineligible candidates. The scheduler and notifications remain #67's responsibility.
- #66 should call `VideoTools::cancel()`, stop and drain media users, then remove files. Cancellation stops setup, update checks, preparation, idle waits and health processes without changing the selected pair. Removal must preserve finished Download files.
- #68 still owns source switching and scheduled compatibility checks. This ticket uses the stable source and the existing per-channel roots.

## Validation

Final commands and review results are recorded after the complete validation run. These tests use local signed fixtures and a mock desktop bridge. They do not claim new real-release compatibility, Intel execution or macOS 13 hardware coverage.
