# Pinned Rune preview verification

Issue [#100](https://github.com/slovakian/opul/issues/100), checked 2026-11-17 on macOS 26.2, Apple Silicon. See the [behavior baseline](baseline.md) and [disposable source](../../prototypes/rune-baseline/).

## Reproduce

From the repository root:

```sh
Scripts/rune-preview.sh
```

This builds the locked standalone preview and opens its native window. No separate Rune checkout is needed. The fixture creates only `.build/rune-preview/data` and `.build/rune-preview/Downloads`, prints their parent location, and never opens an Opul library or starts an engine. Closing the window ends the process. `rune::orderly_exit()` opts out of Rune's fast-exit default. This preview has no transfer shutdown or persistence to exercise.

Run headless captures, scripted input and semantic assertions through the same view and renderer:

```sh
python3 Scripts/check-rune-preview.py

# A single paused fixture, PNG and semantic tree from one render:
Scripts/rune-preview.sh \
  --script prototypes/rune-baseline/scripts/pause.json \
  --snap .build/rune-preview/paused.png \
  --tree .build/rune-preview/paused.json --at 1.2 --scale 1

# Native window: collect 120 frames, then close it automatically.
Scripts/rune-preview.sh --native \
  --trace .build/rune-preview/evidence/native-arm64.jsonl --frames 120
```

The check captures initial Downloading/Pause fixture, taps `toggle-transfer` and observes Paused/Resume fixture, then runs a two-tap resume and appearance switch. It asserts rendered status text, accessible button names/roles, focusability, nonzero geometry, changed surface color, and empty isolated storage directories. PNGs and full semantic trees are written to `.build/rune-preview/evidence`. Failure exits nonzero. The controls mutate fixture state through Rune event dispatch; the check does not reconstruct callbacks or mock the renderer.

Retained samples: [light preview](evidence/preview-light.png), [dark preview after scripted resume](evidence/preview-dark.png), [paused semantic tree](evidence/preview-paused.json). The source intentionally remains small and disposable. These are tooling demonstrations, not production parity evidence or a chosen visual design.

## Dependency and toolchain record

The standalone [manifest](../../prototypes/rune-baseline/Cargo.toml) pins Rune to `cca5df6fafb4721d96fe105f126ebfd2ec351bb6` and repeats all five patches from that revision's workspace: `winit`, `parley_engine`, `wgpu-hal`, `accesskit_windows`, `accesskit_macos`. Cargo does not inherit a dependency's patches. [Rune's dependency instructions](https://github.com/anomalyco/rune/blob/cca5df6fafb4721d96fe105f126ebfd2ec351bb6/crates/rune/README.md#depending-on-rune) describe this requirement. The checked-in preview lockfile fixes the transitive resolution independently of production `Cargo.lock`. `tiny-skia-path` is an upstream path dependency and resolves from the same git revision automatically.

Tested compiler: rustc 1.99.0, LLVM 23.1.1; macOS SDK 26.5 via Command Line Tools. Full Xcode is not installed: `xcodebuild -version` reports that the active developer directory is a Command Line Tools instance. This did not block the default preview build or native launch. Upstream's own toolchain file asks for 1.99.1, but the external preview was checked with the available 1.99.0 compiler. The pinned default renderer uses Rune's Metal backend; no local framework patch was introduced.

## Build and run coverage

Preferred architecture builds use these exact commands:

```sh
MACOSX_DEPLOYMENT_TARGET=13.0 cargo build --locked \
  --manifest-path prototypes/rune-baseline/Cargo.toml \
  --target-dir target/rune-preview --target aarch64-apple-darwin -j 2
MACOSX_DEPLOYMENT_TARGET=13.0 cargo build --locked \
  --manifest-path prototypes/rune-baseline/Cargo.toml \
  --target-dir target/rune-preview --target x86_64-apple-darwin -j 2
```

The host arm64 debug build and native launch passed. The native trace recorded 120 presents, zero failed surface acquires, and display-link pacing at 60 Hz; 119 frames had presentation timestamps and one was skipped. Observed presentation interval p50/p95 was 16.67ms with a 100ms maximum. Concurrent first-build work caused a debug startup warning of 4706ms before the first frame; this is not an app performance benchmark. The process exited after the requested frames. Headless scripted renders passed at 900 × 560, scale 1; native trace used scale 2.

| Build | Build result | Actual run coverage |
| --- | --- | --- |
| Default `aarch64-apple-darwin` debug | Passed | Native macOS 26.2, 120 presents; headless initial/pause/resume/dark checks passed. |
| `aarch64-apple-darwin`, deployment target 13.0 | Passed; `vtool -show-build` confirms Mach-O `minos 13.0`, SDK 26.5 | Native macOS 26.2, 120 presents, zero failed acquires. |
| `x86_64-apple-darwin`, deployment target 13.0 | Passed; `vtool -show-build` confirms Mach-O `minos 13.0`, SDK 26.5 | Native window under Rosetta on Apple Silicon/macOS 26.2, 120 presents, zero failed acquires. No physical Intel run. |
| macOS 13 operating system | No host available | Not run; neither deployment metadata nor Rosetta proves macOS 13 compatibility. |

The Intel/Rosetta window needed foreground activation before its trace completed. After activation it recorded 117 presentation timestamps and three skipped timestamps, with interval p50 16.67ms and p95 33.33ms. The explicit arm64 deployment-target build recorded 118 timestamps and two skipped timestamps with p50/p95 16.67ms. These debug runs establish build/launch coverage, not production performance. Both processes exited after their 120-frame runs.

To repeat the architecture-specific launches after building:

```sh
target/rune-preview/aarch64-apple-darwin/debug/opul-rune-preview \
  --native --trace .build/rune-preview/evidence/native-arm64-13.jsonl --frames 120
arch -x86_64 target/rune-preview/x86_64-apple-darwin/debug/opul-rune-preview \
  --native --trace .build/rune-preview/evidence/native-intel-rosetta.jsonl --frames 120
xcrun vtool -show-build target/rune-preview/aarch64-apple-darwin/debug/opul-rune-preview
xcrun vtool -show-build target/rune-preview/x86_64-apple-darwin/debug/opul-rune-preview
```

Bring a traced window to the foreground if it is occluded. A deployment target in a build command is not evidence of running on macOS 13. Physical Intel hardware and macOS 13 are not available on this host. Packaging, signing/notarization, actual download output, native dialogs/menus, VoiceOver operation, and production shutdown remain follow-up work in #102 and #118; this ticket does not claim them from a fixture or semantic tree.

## Baseline checks

The focused baseline check passed 12 WebKit cases from `app.spec.ts` and `workspace-branches.spec.ts`. Its captures provide the baseline inspector and dark-interface images. The baseline inventory check passed all 70 entries. TypeScript checking passed before implementation and as part of the production build afterward. `vp check`, all 107 frontend unit tests, the production build, root/preview Rust formatting, the preview typecheck, and the renderer script passed.

The final Rust run passed 248 tests: 205 core and 43 desktop, with two existing ignored desktop tests. An earlier unconstrained run, concurrent with architecture compilation, hit five-second timeouts in `built_in_tools_enable_audio_formats_and_require_ffprobe` and `guided_hls_and_dash_reach_the_scoped_yt_dlp_handoff`. Both passed individually and the complete workspace then passed with two test threads. No test or production source was changed to obtain this result.

Two parallel WebKit attempts exposed baseline instability: the first passed 275/276 with a folder-focus setup timeout; the second passed 274/276 with timeouts in Custom preview initialization and hue dragging. The first failing case passed in a focused rerun, whose last case then hit an artifact-cleanup error because another invocation shared `test-results`. Keep concurrent invocations' output directories separate. The final single-worker command below uses its own results directory. These findings describe the unchanged baseline tests; the Rune preview does not import or change the frontend.

The final single-worker full run passed 275/276 in 7.3 minutes. `color-suggestions.spec.ts:249`, folder suggestion stability, failed with `Execution context was destroyed, most likely because of a navigation` while reading the picker. An isolated single-worker rerun passed in 7.3 seconds. All other cases, including the three earlier timeouts, passed in the final full run. This records an unstable browser baseline, not a fully green full-suite result. No existing assertions were changed or weakened. The full-run failure artifacts remain in `.build/rune-preview/baseline-webkit`; this instability should be resolved before the final parity gate in #118.

```sh
OPUL_TEST_PORT=1552 vp run test --project=webkit \
  tests/ui/color-suggestions.spec.ts --grep='folder suggestions stay stable' \
  --workers=1 --output=.build/rune-preview/baseline-focused
```

```sh
vp install
vp exec tsc --noEmit
vp run features:check
vp run test --project=webkit tests/ui/app.spec.ts tests/ui/workspace-branches.spec.ts --workers=2
vp check
vp test
vp run build
OPUL_TEST_PORT=1551 vp run test --project=webkit --workers=1 \
  --output=.build/rune-preview/baseline-webkit
Scripts/fetch_ffmpeg.sh
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace -- --test-threads=2
cargo fmt --all --check
cargo fmt --manifest-path prototypes/rune-baseline/Cargo.toml --check
cargo check --locked --manifest-path prototypes/rune-baseline/Cargo.toml \
  --target-dir target/rune-preview
```

The preview is an additive experiment. Production Rust, frontend behavior, root dependency manifests, and existing tests remain at baseline. The two pre-existing untracked migration research/spec documents were preserved and are not part of this delivery; the authoritative migration scope is #99.

Standards review found no hard violations and one optional duplicated-label cleanup, which was applied. Spec review found no missing or incorrect implementation requirements. Both axes have zero remaining findings.
