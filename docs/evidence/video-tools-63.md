# Waiting video Downloads, issue 63

Baseline `0efdf8fda127b400753969b0b92e228ef5229984` on `integration/58-opul-managed-tools`. Blocker #61 is closed and present at this baseline, along with #62. Parent #58 and its comments were read before implementation. The parent remains open and unchanged.

## Behavior and evidence

Video submission saves an unprobed `PendingPage` Download before extraction. The Rust reducer checkpoints its identity, Workspace, Saved Folder, format, media policy and source. Unavailable tools produce `WaitingForVideoTools` with no transfer effect. Ready produces ordinary queued transfer effects; repeated Ready facts do not duplicate them. Actor checkpoint tests reopen the library, verify the original identity and destination, and verify one Ready dispatch. Ready received during reconciliation remains fenced until Running.

A waiting playlist stays one row. Its probe result expands it in place, retaining the first unique entry's original Download identity, preserving playlist ordering and the selected destination, format and policy. Deduplication stays local to the Workspace. The reducer ignores repeated or stale results. Direct files and torrents retain their normal paths. Waiting rows remain visible and counted in In progress, without a Pause capability.

The consent dialog shows the published size or fallback. Download saves the waiting Download before starting setup. A rejected add keeps the dialog available and never starts setup. Not now, Esc and closing drop the link; Don't show this again with dismissal persists Turned off. Setting up accepts links without another offer. Turned off disables Video with the Settings explanation. Waiting row labels and Set up / Try again actions come from the projection.

The first regression failed before implementation because unavailable tools still dispatched a media effect. A second failed-before regression exposed resumed pending pages expanding without transfer effects. A focused frontend regression returned an empty In progress list for a waiting Download before its filter correction. All pass after the fixes.

Deferred probes run serially on a separate worker, allowing the normal runner to process direct downloads and controls. The subprocess has a two-minute deadline and concurrent output readers. Pause before a transfer exists records Paused directly; changes in attempt, generation, lifecycle or session cancel the old probe. Both success and failure facts carry generation fences, including failures racing with pause, cancel or shutdown. Process-group cleanup also stops descendants retaining output pipes.

A real application-state-machine test installs a signed fixture pair, substitutes a controlled slow interpreter, and proves an unrelated HTTP file finishes while the page probe is blocked. It verifies immediate pause and shutdown within three seconds. Media tests kill a sleeping probe and a descendant retaining its pipes; both return within three seconds. The desktop command test accepts an unavailable-tool playlist without probing, shuts down, reopens and compares the saved Download.

[Waiting video Download](../screenshots/video-download-waiting.png)

## Validation

On Apple silicon macOS, 2026-11-07:

- `vp install`, `vp check`, `vp exec tsc --noEmit`, and `cargo fmt --all --check` passed.
- `RUST_TEST_THREADS=4 Scripts/test.sh` passed 21 desktop tests, 141 core tests, 37 frontend unit tests, the production frontend build, and 213 WebKit tests before the final In progress filter correction.
- After that correction, `vp test` passed 38 tests in eight files, `vp run build` passed, and the final complete WebKit run passed 213 tests. The six waiting-video interaction tests include consent, rejected additions, dropped links, restart, playlist waiting, In progress visibility/counting, failure recovery and Turned off. Axe audits cover the consent dialog, waiting library, recovery and disabled Video controls. Base UI's generated focus guards use the existing suite's axe exclusion.
- `cargo clippy -p opul_core --all-targets -- -D warnings` passed.
- `Scripts/test_packaging.sh` passed.

Earlier unrestricted runs encountered the existing media fixture deadline and transient workspace/inspector failures. Focused reruns and the complete sequential repository script passed without changes to those tests. One new interaction assertion initially targeted the sidebar button instead of its sibling badge; the corrected assertion and its full rerun pass.

Standards and spec reviews independently identified the deferred-probe pause/resume problem. The standards review also identified blocking the ordinary runner. Follow-up review found the stale-failure generation race and descendants retaining pipes. Every finding has a regression and is fixed; both reviewers confirmed the corrections. The final In progress change also received a spec review with no findings.

## Boundaries and handoff

- #64 can reuse `useVideoTools`, submission consent, `VideoToolsProgress`, the existing tools operations and pushed snapshots. Setup remains owned by the video-tools module.
- #66 owns removal and returning already running transfers to waiting while keeping partial output. Reuse `WaitingForVideoTools` and its projected recovery capabilities.
- #65 and #67 retain ownership of updates and update notifications. No updater or first-launch Setup was added here.
- Runtime release/signature policy and bundled tools were unchanged. This ticket's runtime evidence uses signed local fixtures and the real application/desktop seams; it does not repeat the networked packaged-content smoke from #61/#62 or claim physical Intel/macOS 13 testing.

The local commit hook rejected touched files exceeding 5,000 tokens. Existing application reducer responsibilities and tests, Download domain types, and desktop command tests were moved into focused modules to satisfy it. Assertions and behavior in those mechanical moves were preserved. This explains the larger structural diff.
