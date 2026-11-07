# Packaged tools integration, issue 69

Baseline: `139d9ceec841a415eae75f10339438e547fc11c9`, fetched from `origin/integration/58-opul-managed-tools`. Parent #58 and its empty comment thread were read. All sibling tickets are closed, and their delivered commits are ancestors of that integration head. Main and the parent issue remain unchanged. This record does **not** close #69: the open gates below are acceptance requirements.

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

## Changes for final verification

The glossary now establishes Tools, Built-in tools, Video tools, Python runtime, Setup and all five states. ADR 0026 explains the signed pointer, pair activation, retention/rollback and built-in versus optional parts, explicitly preserving ADR 0024 ownership, ADR 0006 playlist rules and the Tauri shell. Contributor and release guidance describes consent, immediate Settings changes, the hidden override, architecture-specific packages, runtime minimum app version and exact verification commands. The module guide lists the delivered command/snapshot contract and removes stale sibling handoffs.

`tools/verify_package.py` audits every native file in a copied or mounted app, including the main executable. It rejects extra architectures, deployment targets newer than macOS 13, escaping links, invalid signatures and accidentally bundled optional executables. App and DMG packaging invoke it. Reports contain byte counts, native-file hashes and explicit limits on OS evidence. The existing arm64 package passed; negative runs rejected the wrong architecture and a copied app with its minimum changed to macOS 14. These were real artifact checks, not mocked command results.

The opt-in packaged smoke now saves a video before consent, verifies its waiting state, installs real signed tools, downloads the permitted content through the application reducer/media engine, removes tools into Turned off while preserving completed bytes, explicitly sets up again, and relaunches the same isolated library to verify Ready and output persistence. It records each phase separately. The app closes its final window through Tauri; the harness verifies a Stopped checkpoint after each ordinary quit. Faults requiring a new signed runtime are exercised through the existing fixture seam rather than tampering with published releases.

Independent code-review Standards and Spec axes reviewed this diff against the baseline. Standards found no violations or material smells. Spec found that the original restart harness terminated the process rather than exercising normal quit. The final-window close and Stopped-checkpoint assertion correct that gap; the reviewer confirmed the correction, and both architectures passed the actual path. The remaining Spec findings are the explicitly open distribution gates below.

## Runtime measurements

Inspected on Apple silicon macOS 26.2. Intel execution is through Rosetta. Runtime archives were already downloaded from the signed pointer and authenticated in #68; this rerun inspects those exact extracted copies, rather than claiming a fresh download.

| Runtime 1 | Compressed bytes | Sum of regular-file bytes on disk | Native files |
| --- | --: | --: | --: |
| arm64 | 17,143,107 | 35,401,156 | 47 |
| x64 | 17,654,743 | 35,118,983 | 49 |

`python3.13 tools/python-runtime/verify.py <runtime> <arch> --no-download` passed for both: every native file has only the expected slice, an inspected deployment target no newer than 13.0, valid signatures and permitted library dependencies. Both actual interpreters passed package imports, CA configuration and absence checks; each QuickJS passed seven sandbox tests. These numbers exclude separately installed yt-dlp, symlink storage, filesystem allocation overhead and retained rollback versions.

## Installer and real application measurements

| Package | App version | DMG bytes | Sum of app regular-file bytes | Report |
| --- | --- | --: | --: | --- |
| Stable arm64 | 0.2.2 | 17,676,580 | 40,923,424 | [Audit](video-tools-69-package-arm64.json) |
| Nightly x64 | 0.2.2-nightly.20261108.69 | 18,651,233 | 43,586,334 | [Audit](video-tools-69-package-x64.json) |

Each audit includes the installer SHA-256 and all three native-file hashes. Each installer contains only its matching app executable, FFmpeg and ffprobe, with deployment targets of 13.0, valid ad-hoc signatures, license texts and exact source pointers. The audit ran against both mounted DMGs during packaging. No optional video tools are bundled. Built-in tools copied from both packages passed all five jobs with generated CC0 fixtures. Native launch checks rendered each packaged frontend, connected to Rust and resolved both built-in executables; x64 ran through Rosetta on this Apple silicon host.

Both [arm64](video-tools-69-smoke-arm64.json) and [x64](video-tools-69-smoke-x64.json) passed the expanded real application journey and ordinary quit/relaunch. They downloaded runtime 1 and stable yt-dlp 2026.10.26 after consent, completed the 11,053,871-byte CC BY 3.0 Blender trailer, preserved it through removal to Turned off and explicit re-setup, and reopened Ready with the completed output present. Each Tools root was excluded from Time Machine. These are temporary isolated libraries, not the person's data. The x64 app's default stable script was verified; the Nightly source switch has separate #68 evidence.

One x64 attempt failed with upstream curl DNS error 28 while fetching the permitted media. The unchanged smoke passed on a new isolated retry. An earlier local run coincided with disk exhaustion and was discarded; Cargo's generated-cache cleanup restored space, then compilation, strict core Clippy and the real journeys passed. Neither incident was hidden by changing assertions or product behavior.

Source searches of app copy, README, contributor/release guidance and release workflow text found no branded video-site wording or obsolete user path/restart instructions. macOS Vision OCR found no known site-brand text in all 14 saved screenshots; [audit output](video-tools-69-screenshot-audit.log). Setup progress, Settings and failed-download update advice were also visually inspected. Actual media in these runs is the attributed Creative Commons trailer; built-in fixtures are generated CC0, and UI fixtures are synthetic.

## Open distribution gates

- No macOS 13 execution environment or physical Intel Mac is available here. The app claims a 13.0 minimum, and deployment metadata/modern-host execution are necessary evidence but cannot establish that promise. Run both final packages on supported macOS 13 systems before checking this criterion. Do not close #69 based on Rosetta alone.
- Runtime 1's curl_cffi wheel retains its MIT license but no complete notice inventory for the statically embedded curl-impersonate dependency tree was found. The actual arm64 runtime reports `libcurl/8.23.0-IMPERSONATE BoringSSL zlib/1.3.1 brotli/1.2.1 zstd/1.5.7 c-ares/1.34.9 AppleIDN nghttp2/1.63.0 ngtcp2/1.20.0 nghttp3/1.15.0 curl-impersonate/2.3.1`. Resolve the exact native-source provenance and required notice texts, include them in the distribution, and publish a new numbered runtime if runtime contents change. Do not overwrite runtime 1. Top-level Python/wheel notices and exact FFmpeg/LAME sources are documented in [third-party notices](../../THIRD_PARTY_NOTICES.md).
- The real packaged smoke covers a waiting single video, consented installation, removal, re-setup and restart. The complete packaged UI journey for a waiting playlist, actual version-changing manual/automatic updates and runtime rollback, with direct files/torrents throughout, still needs an acceptance run. Signed fixtures and mocked-bridge scenarios prove those contracts independently; they do not replace this final combined journey.
- Developer ID/notarization was not exercised without credentials. Ad-hoc Apple signatures and mandatory Opul signatures must be described accurately. No notarization claim is made.

## Validation record

Validation used the source tree committed for #69 on 2026-11-08 local time. The issue's progress comment identifies the pushed integration commit and its CI run; the commit containing this record is discoverable with `git log -- docs/evidence/video-tools-69.md`.

- `vp install`, `vp check`, `vp exec tsc --noEmit`, `cargo fmt --all --check`, `cargo check --workspace`, strict core Clippy and `git diff --check` passed. After the cache cleanup, Rust validation used `CARGO_INCREMENTAL=0`.
- `RUST_TEST_THREADS=4 cargo test --workspace` passed all 183 core tests, 22 desktop tests and doc tests. The focused public video-tools suite also passed all 65 scenarios.
- `vp test` passed all 38 frontend unit tests; `vp run build` passed TypeScript checking and the production build.
- `OPUL_TEST_PORT=1457 vp run test --workers=2` ran all 232 WebKit scenarios: 229 passed, and three unrelated emoji-picker cases failed with navigation interrupting an active picker action. `OPUL_TEST_PORT=1457 vp run test --last-failed --workers=1` then passed all three in isolation, with builds and file writes stopped. No assertions or frontend code changed. All 232 have passing results; this is not one uninterrupted green browser run.
- `python3.13 tools/compatibility/test_check.py` passed eight offline contracts, and `tools/python-runtime/test_release.py` passed four signature/publication contracts with OpenSSL 3 on PATH.
- `Scripts/test_ffmpeg_fetch.sh`, packaging configuration checks and shell syntax checks passed. Both release DMGs built, mounted and passed the package audit. Both packaged tool pairs passed `Scripts/test_ffmpeg.sh`; both apps passed native launch and expanded network smoke checks, with the x64 DNS retry disclosed above.
- Both existing published runtimes passed structural/import/sandbox verification again: [arm64 log](video-tools-69-runtime-arm64.log), [x64 log](video-tools-69-runtime-x64.log). Their original signatures/direct-download evidence remains in the #60/#68 records; this structural rerun does not claim a new network download.

No new unit test was added for documentation or the package inspector. The inspector was verified against actual signed apps, a wrong architecture, and an app whose declared minimum was changed to 14.0. The expanded real application smoke exercises the existing public module/reducer seams directly. The independent review's ordinary-shutdown gap was corrected and proven on both actual packages.

Leave #69 open for the distribution gates above. Leave parent #58 open and unchanged; do not merge the integration branch to main without a separate request.
