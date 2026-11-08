# Packaged tools integration, issue 69

The complete real packaged journey and local validation now pass for both architectures on macOS 26.2. Runtime 2 resolves the native notice gap. **The remaining acceptance blocker is actual macOS 13 execution.** The user confirmed that no macOS 13 Mac/VM or external volume is available. Package deployment targets and Rosetta execution do not establish that minimum. Leave #69 open until that requirement is verified; keep parent #58 open and unchanged, and do not merge to main.

Baseline: `139d9ceec841a415eae75f10339438e547fc11c9`, fetched from `origin/integration/58-opul-managed-tools`. Parent #58 and its empty comment thread were read. All sibling deliveries below are present on the integration branch. The issue progress comment records the final pushed SHA and its CI result; this record and every required implementation file are committed to that branch.

## Sibling delivery audit

| Ticket | Delivered integration checkpoint | Acceptance evidence |
| --- | --- | --- |
| #59 | `45589a4` | [Built-in tools and both packages](../built-in-tools-verification.md) |
| #60 | `2d438a5` | [Published runtime signatures, sources, sizes and workflow](../python-runtime.md#runtime-1-verification) |
| #61 | `f90db49` | [Verified setup and packaged download](video-tools-61.md) |
| #62 | `0efdf8f` | [Interruption, retry, tamper and restart recovery](video-tools-62.md) |
| #63 | `2934561` | [Waiting video/playlist, reducer and checkpoint](video-tools-63.md) |
| #64 | `4ce16b0` | [Consent, skip, Turned off, re-entry and accessibility](video-tools-64.md) |
| #65 | `cacd13d` | [Updates, activity, retention and rollback](video-tools-65.md) |
| #66 | `15ac7b6` | [Removal, actual process exit, partial output and resumed completion](../../src/video_tools/tests/waiting_downloads.rs) and [issue completion record](https://github.com/slovakian/opul/issues/66#issuecomment-6709833203) |
| #67 | `cdf1383` | [Automatic checks, consent notices and update advice](video-tools-67.md) |
| #68 | `139d9ce` | [Nightly, real-tool combinations and final CI](video-tools-68.md) and [CI follow-up](https://github.com/slovakian/opul/issues/68#issuecomment-6710723851) |

Each checkpoint passed `git merge-base --is-ancestor <checkpoint> origin/integration/58-opul-managed-tools`. These records distinguish mocked bridge/signed fixture coverage from actual published-tool execution. The shared destination is the integration branch; no per-ticket PR or main merge is needed.

## Delivered implementation and guidance

The glossary establishes Tools, Built-in tools, Video tools, Python runtime, Setup and all five states. ADR 0026 documents verification, pairing, pointer activation, disk lifecycle and rollback, superseding ADR 0021's discovery paragraph while preserving ADR 0024 ownership, ADR 0006 playlist rules and the Tauri shell. Contributor, release and module guides cover built-in/optional tools, consent, immediate Settings changes and the hidden developer override.

The [combined packaged driver](../packaged-video-journey.md) exercises the actual React dialogs and public application state machine with production signed tools. It exchanges real HTTP bytes and BitTorrent pieces. It uncovered and fixed [two integration regressions](video-tools-69-integration-fixes.md): shared HTML5 playlist page URLs lost distinct entries, and completed single-file torrents could remain Fetching without an output path. Five focused regression tests cover these fixes.

Independent Standards and Spec reviews found no remaining blockers in the implementation. Review corrections tightened ordinary quit/checkpoint verification, restart identities/destinations/formats/output hashes, actual SettingUp/Updating phase assertions, modal dismissal and the visible Settings switch selector. These assertions passed against both rebuilt packages.

## Runtime publication and notices

[Runtime 2 publication evidence](video-tools-69-runtime-2.md) records both signed archives, hashes, sizes, native checks, complete curl dependency notices and exact source provenance. [The release](https://github.com/slovakian/opul/releases/tag/python-runtime-2) and signed production pointer are published. Runtime 1 remains immutable; update acceptance reconstructs its verified historical pair using checked-in trust keys, without repointing production or overriding trust. [arm64 baseline verification](video-tools-69-baseline-arm64.log), [x64 baseline verification](video-tools-69-baseline-x64.log).

## Final package measurements

| Package | App version | DMG bytes | Sum of app regular-file bytes | Report |
| --- | --- | --: | --: | --- |
| Stable arm64 | 0.2.2 | 17,743,200 | 41,021,056 | [Audit](video-tools-69-package-arm64.json) |
| Nightly x64 | 0.2.2-nightly.20261108.69 | 18,707,220 | 43,712,094 | [Audit](video-tools-69-package-x64.json) |

Each audit records installer SHA-256 and all three native-file hashes. Both copied apps and mounted DMGs contain only the matching app executable, FFmpeg and ffprobe, with 13.0 deployment targets, valid ad-hoc signatures, license texts and exact source pointers. Optional video executables are absent. Both built-in pairs passed all five generated-CC0 fixture jobs. Both packaged frontends rendered, connected to Rust and resolved both built-in tools. Intel execution on this Apple silicon host used Rosetta.

## Complete real packaged acceptance

Both architectures passed all seven launches against stable yt-dlp 2026.10.26 and the signed runtime releases. Every launch closes the last window normally, exits successfully and records a Stopped checkpoint. Each subsequent launch verifies all prior Download IDs, destinations, formats and completed-file hashes. Tools roots remain excluded from Time Machine. [Measured summary](video-tools-69-journey-summary.json).

| Launch | arm64 report | x64 report |
| --- | --- | --- |
| Initial offer, skip, waiting video/playlist, Turned off | [Report](video-tools-69-journey-arm64-lifecycle-offer-report.json) | [Report](video-tools-69-journey-x64-lifecycle-offer-report.json) |
| Setup, playlist expansion, removal/Cancel, partial retention and re-setup | [Report](video-tools-69-journey-arm64-lifecycle-setup-report.json) | [Report](video-tools-69-journey-x64-lifecycle-setup-report.json) |
| Lifecycle restart | [Report](video-tools-69-journey-arm64-lifecycle-restart-report.json) | [Report](video-tools-69-journey-x64-lifecycle-restart-report.json) |
| Manual update, rollback, retry and successful video | [Report](video-tools-69-journey-arm64-manual-manual-report.json) | [Report](video-tools-69-journey-x64-manual-manual-report.json) |
| Manual-update restart | [Report](video-tools-69-journey-arm64-manual-restart-report.json) | [Report](video-tools-69-journey-x64-manual-restart-report.json) |
| Automatic update and successful video | [Report](video-tools-69-journey-arm64-automatic-automatic-report.json) | [Report](video-tools-69-journey-x64-automatic-automatic-report.json) |
| Automatic-update restart | [Report](video-tools-69-journey-arm64-automatic-restart-report.json) | [Report](video-tools-69-journey-x64-automatic-restart-report.json) |

Not now dismisses without installing; Don't show this again persists Turned off. A single video and two-entry playlist wait before consent and survive restart. Setup expands the playlist into distinct entries while preserving original identity, destination and Original format. All three video outputs match the exact CC0 fixture bytes.

Removing tools while one real video is active shows the exact count. Cancel retains Ready. Confirmed removal stops the worker and preserves written partial bytes; re-setup resumes the same Download with a positive HTTP Range and completes with the exact 11,053,871-byte CC BY 3.0 Blender trailer. [arm64 traffic](video-tools-69-journey-arm64-traffic.json), [x64 traffic](video-tools-69-journey-x64-traffic.json).

Manual Check reports runtime 2 without installing it. Update activates 1→2. A controlled local HTTP 404 on the first real video Download triggers the production pre-success rollback 2→1; the upstream error remains verbatim in the report. A second explicit update and successful video complete, and the previous runtime is pruned. The automatic path enables the real Settings switch, waits for the production scheduler and updates 1→2 with no manual check/update command or clock override.

Each architecture completed direct HTTP and actual BitTorrent transfers in 19 phases: before consent, Turned off, after restart, during actual Setup, Ready, video running, removed, re-setup, manual-before/available/Updating/rolled-back/finished, automatic waiting/Updating/Ready and the three final restarts. Every direct/torrent output is byte-checked. Each run records 19 real peer pieces totaling 18,016 bytes. Snapshots prove that transfers began during actual SettingUp and Updating states.

## Validation and audit

The final source passed `vp check`, `vp exec tsc --noEmit`, `cargo fmt --all --check`, `cargo check --workspace`, strict core Clippy and `git diff --check`. `vp install` was run before implementation; no dependencies changed in the final slice.

- Rust workspace: 188 core tests, 22 desktop tests and doc tests passed.
- Frontend: all 38 unit tests and the production TypeScript/Vite build passed.
- WebKit: all 232 scenarios passed in one uninterrupted run with `OPUL_TEST_PORT=1457 vp run test --workers=2`.
- Offline contracts: four native-notice tests, four release/signature tests and eight compatibility tests passed.
- Both release DMGs built, mounted and passed the package audit. Both copied packages passed native frontend/bridge/tool launch checks and all five built-in FFmpeg jobs.
- Runtime 2's two-architecture publication workflow passed structural/import/signature/sandbox and actual permitted-content download checks; its app CI passed all seven jobs. Final integration CI is linked with the pushed SHA in the issue comment.

App-owned copy, README, contributor/release guidance and workflow text contain no branded video-site wording or obsolete user path/restart instructions. Vision OCR found no known site-brand text in all 14 saved screenshots; [audit output](video-tools-69-screenshot-audit.log). Setup progress, Settings and failed-download advice were visually inspected. No frontend copy changed after that audit. Fixtures are generated CC0, synthetic UI data or the attributed Creative Commons trailer. Upstream diagnostics are preserved.

Earlier attempts are not substituted for these final passing runs. They exposed the two fixed product regressions, a damaged scratch baseline missing Python's zip library (replaced by authenticated release reconstruction), and driver assertions/selectors that needed correction. One fresh setup exceeded the existing 30-second version-check timeout; the unchanged retry and final runs passed. An earlier browser run passed 231 cases but could not save one case's trace after ENOSPC; the final full 232-case run passed after generated-cache cleanup. A cache cleanup also interrupted an earlier Rust doc-test run; the subsequent complete workspace run passed. Assertions and product timeouts were not relaxed.

## Remaining macOS 13 gate

No macOS 13 Mac/VM or external volume is available, as confirmed by the user. This host runs macOS 26.2. Ventura VM image candidates require 18–23 GB compressed plus a virtual disk, exceeding available local storage. Do not treat modern-host or Rosetta success, native deployment metadata, or signed artifacts as actual macOS 13 execution.

The handoff is to run both final packages' built-in checks and [the same complete packaged journey](../packaged-video-journey.md) on macOS 13, recording OS/hardware, hashes, reports and any fixes. Then check the minimum-OS acceptance criterion and close only #69. Developer ID/notarization was not exercised and is not claimed; mandatory Opul signatures and ad-hoc Apple signatures were verified. Parent #58 remains open and unchanged; main remains untouched.
