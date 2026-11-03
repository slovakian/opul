# Issue 59 verification

Baseline: `f9b28978705a79cafd99b65ad413c6daaccc0126`. Implementation is on `integration/58-opul-managed-tools`; main was not changed.

The [tools workflow run](https://github.com/slovakian/opul/actions/runs/41586984622) built both architectures on Apple silicon, cross-compiled Intel, executed the Intel programs through Rosetta, and passed every generated CC0 media fixture. The immutable [ffmpeg-1 release](https://github.com/slovakian/opul/releases/tag/ffmpeg-1) contains both archives, exact upstream sources, recipe, license provenance, checksums and separate architecture logs. `release.env` pins the published archive digests. No `media-tools-1` executable was reused.

Local verification on 2026-11-03:

- The new resource-only regression failed against the baseline because it discovered Homebrew FFmpeg, then passed with explicit resource resolution.
- `cargo check --workspace`, `cargo fmt --all --check` and `cargo test --workspace` passed: 104 core tests and 20 desktop tests.
- `vp check`, TypeScript checking, `vp run build` and `vp test` passed; frontend unit tests: 37. The Tools WebKit interaction and axe audit passed.
- Full WebKit runs exercised all 195 scenarios. They exposed intermittent failures in workspace cleanup, custom appearance and folder removal during concurrent Rust builds. All affected cases passed isolated retries. Those unrelated flows were not changed to accommodate the failures.
- Downloaded release archives for both architectures passed `Scripts/test_ffmpeg.sh`, including Intel execution through Rosetta.
- `Scripts/test_ffmpeg_fetch.sh` passed: corrupt cache plus corrupt response cannot replace valid resources; universal targets are rejected; signing mutations regenerate verifiable checksums. The signing test asserts Developer ID arguments and uses real ad-hoc hardened-runtime signatures without a certificate.
- `Scripts/package_dmg.sh release stable arm64` produced a 15.88 MiB DMG. `Scripts/test_packaging.sh` verified its mounted app, architecture, executable checksums, deployment targets, signatures and notices. Its SHA-256 was `1c4f3ae54a556fc096491deac8272eb4aa60907a9c234f7148ccc520f4fc3214`.
- The packaged app launched with isolated data, rendered its frontend, and resolved both tools through the real media engine. All five jobs passed again using executables inside that app's Resources.
- The initial [application CI run](https://github.com/slovakian/opul/actions/runs/41587565213) passed Rust, lint/type checks and both Nightly DMG packaging jobs. It was superseded by the final smoke-check/artifact commit.

All seven final CI jobs passed for code commit `b9f34653cefca9119476ca453599ec554b599761`, including both WebKit shards (all 195 scenarios), both architecture DMGs and the native packaged-app smoke check.

The final [application CI run](https://github.com/slovakian/opul/actions/runs/41588114361) provides [Apple silicon](https://github.com/slovakian/opul/actions/runs/41588114361/artifacts/13208623464) and [Intel](https://github.com/slovakian/opul/actions/runs/41588114361/artifacts/13209012074) Nightly DMGs (seven-day retention). The downloaded Intel ZIP matched its Actions SHA-256 digest. Its mounted tools passed every media fixture under Rosetta; the app copied from that DMG also rendered and resolved both tools through Rust. Launching the Intel app directly from the mounted image timed out twice at the smoke harness's 20-second limit; the normal copied-app launch passed.

The standards and spec reviews identified missing explicit signing of resource executables. The fetch hook now verifies upstream bytes, signs staged copies when a Developer ID is configured, updates their checksums and preserves the upstream archive pointer before Tauri seals the application. The follow-up review found no remaining material issues.

Limits: deployment metadata is verified as macOS 13.0; execution was on current Apple silicon macOS and under Rosetta, not on macOS 13 hardware. No Developer ID certificate or notarization credentials were available for an actual notarized distribution test.

The size hook rejects any staged file over 20 KB, including the pre-existing 180 KB `src/media.rs`. The feature commit bypassed that hook to avoid an unrelated whole-module rewrite; `vp staged`, formatting, lint, type checks and tests were run separately. Other commits used the normal hooks.
