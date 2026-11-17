# Rune direct-file journey

Issue [#102](https://github.com/slovakian/opul/issues/102) introduces the first production Rune shell in `src-rune`. It uses the existing application state machine, its projection publisher and its direct HTTP downloader. The Tauri interface and disposable prototypes remain available for the later migration tickets.

Build and open the actual macOS bundle:

```sh
Scripts/package-rune.sh
open '.build/rune/Opul Rune.app'
```

The package script builds a debug Apple Silicon app on this host, targets macOS 13, copies resource metadata and attribution, then signs and verifies it ad hoc. It uses the installed Rust toolchain and accepts `CARGO_TARGET_DIR`. This is a local development bundle, without distribution signing or notarization. Resource discovery starts from the executable inside the bundle, independent of the working directory or source tree. The direct-file path requires no optional video tools or FFmpeg. Bundling the media tools for the later video journey remains separate work.

The normal library lives in `~/Library/Application Support/Opul Rune`, with files in `~/Downloads`. Isolate both for manual work:

```sh
OPUL_RUNE_DATA_DIR="$PWD/.build/rune/manual/data" \
OPUL_RUNE_DOWNLOAD_DIR="$PWD/.build/rune/manual/Downloads" \
  '.build/rune/Opul Rune.app/Contents/MacOS/opul-rune'
```

The core creates the ordinary Downloads Workspace and its default Saved Folder. Add a Saved Folder through the native picker, paste a direct HTTP/HTTPS link and press Download. The core retains ownership of identity, destination, status, progress, capabilities, output paths and persistence. Unknown lengths have no invented percentage. Pause, Resume, Cancel, Retry and Remove from list appear only when the core permits them. Cancelled Downloads have no Retry action. Remove from list retains files. Moving files to Trash is not exposed by this slice.

## Command and shutdown ownership

Library opening and commands run on one background worker. A separate subscriber waits on `OpulStateMachine::wait_for_snapshot`; there is no UI snapshot polling or parallel application state machine. Projection sequence and boot identity reject stale results. A command settles its draft only after both its matching receipt and a projection at least as new as that receipt's revision have arrived. Errors retain the input.

Rune's `orderly_exit` disables its fast-exit default. Both `on_quit` and `on_close` also wait for the background shutdown result before returning. This barrier matters on macOS, where native termination can end the process without returning through `main`. The UI waits during termination; the worker performs blocking engine and persistence work. Repeated shutdown requests share one result. A failed shutdown exits nonzero. Headless runs use the same worker and await it after the renderer returns.

The journey exposed two core defects. A quiescent restored media adapter had no worker on Resume, and paused direct workers survived shutdown. Resume now registers a worker without duplicating a live one. Terminal media shutdown closes the concurrency gate, pauses transfers, interrupts HTTP awaits, stops owned process groups and joins writers before the application can commit Stopped. It preserves partial files and deliberate holds. The stalled-body regression checks socket closure and joined workers without waiting for the HTTP read timeout.

## Owned controls and looks

Soft Edge is the default, from #101. The reduced owned button, input, folder icon and progress controls use shared theme tokens, with Quiet line and Inset panel available through the look button. Row dimensions transition when motion is allowed. Reduced motion applies when either the app toggle or the OS requests it. Persisting these preferences belongs to #113, library composition to #108, and full effects to #109.

There is no production dependency on Rune-kit. [Attribution](../../src-rune/NOTICE.md) records the copied construction patterns, upstream revision and retained MIT license. Rune and all five workspace patches retain #100's audited revision. Rune's image animation calls need `image >= 0.25.10`, recorded explicitly because the older Tauri minimum Rust version otherwise makes Cargo select an incompatible image release.

## Verification

The agreed seam is the real Rune interface plus core and filesystem outcomes:

```sh
cargo test --locked -p opul-rune
cargo test --locked -p opul_core shutdown_interrupts_a_stalled_http_body_and_joins_its_writer
python3 Scripts/check-rune-app.py
```

The Rust journeys drive Rune pointer/keyboard dispatch and fixture-dialog answers, then assert semantic status, actual file bytes, folder identity, pause/resume, HTTP-failure retry, cancellation, file retention on removal, saved holds, resumed running intent and rejection of stale projections/receipts. They use loopback HTTP and temporary directories. The folder journey failed before the shutdown and restored Resume fixes, then passed with the same behavioral assertions.

The native script copies the signed bundle outside the repository, launches it with isolated storage and a minimal PATH, chooses a real folder through NSOpenPanel, and performs real HTTP transfers. It quits through Command-Q, launches a fresh process, checks that running intent resumes while a deliberate hold remains held, resumes the held transfer, checks both complete files byte for byte, and closes the last window. Both exit projections must be Stopped and retain the same Download IDs. Native input uses System Events accessibility; macOS must already allow the invoking terminal to control the interface. Generated evidence and logs stay in `.build/rune/evidence`. The output files stay in the printed isolated temporary directory for inspection.

Retained evidence from the final native run: [result and payload digest](evidence/direct-download/result.json), [before Quit](evidence/direct-download/before-quit.txt), [fresh-process semantics](evidence/direct-download/restored.txt), [first Stopped projection](evidence/direct-download/first-stopped.json), and [last-window Stopped projection](evidence/direct-download/second-stopped.json). The [completed screenshot](evidence/direct-download/completed.png) comes from Rune's headless renderer reopening that same completed library. It shows real core state; the native-dialog and Quit evidence comes from the separate native run.

`OPUL_RUNE_STATE_OUTPUT=/absolute/path.json` writes the final authoritative Stopped projection from the worker for diagnostics. It does not replace core persistence.

This ticket's native coverage is Apple Silicon on macOS 26.2. The deployment target does not establish operation on macOS 13. Physical Intel, VoiceOver use, release signing, notarization, app updates and full migration parity remain later acceptance work. See #118 for the final acceptance gate.

## Validation record

Checked on macOS 26.2, arm64, with rustc 1.99.0 and SDK 26.5. The bundle's Mach-O metadata reports macOS 13.0 as its minimum. `vp install`, `vp check`, TypeScript checking, the production frontend build, feature inventory validation, Rust formatting and strict Clippy for the Rune package passed. All 107 frontend unit tests passed.

The final full Rust workspace run passed 253 tests: 207 core, 43 desktop and three Rune journeys, with two existing ignored desktop tests. The review regression for the worker-exit Resume race first failed with `Resume lost its writer: Queued`, then passed after the ownership handoff fix. All 41 focused media tests, the final strict Rune Clippy check and the rebuilt native bundle journey passed after that fix.

The unchanged WebKit suite passed 272/276 in nine minutes. Failures were `color-suggestions.spec.ts:147`, `custom-treatment.spec.ts:221`, `destination.spec.ts:390`, and `icon-picker-visual.spec.ts:29`; they lost picker/dialog state during navigation or timed out waiting for it. All four passed in an isolated single-worker rerun. No frontend source or assertions were changed. This is an unstable full-suite result, not a fully green parity claim. Logs, screenshots and traces remain in `.build/rune/baseline-webkit`; #118 retains the suite-stability gate.

The standards review reported no findings. The spec review found the worker-exit race described above. Both reviewers checked the fix and reported no remaining findings. Native bundle runs prove both Quit routes and two byte-exact 8,224,768-byte outputs, with the payload SHA-256 and stable IDs retained in the linked result record.
