# Contributing to Opul

Focused pull requests and bug reports are welcome. Discuss large changes in an issue before starting, unless the maintainer has already requested the work.

Install dependencies with `vp install --frozen-lockfile`, then run:

```bash
pnpm exec playwright install webkit
Scripts/test.sh
Scripts/package_app.sh debug
Scripts/smoke_test.sh
```

The UI uses the official shadcn skill installed in `.agents/skills/shadcn`. Use Base UI primitives, semantic colors, Sidebar popover menus, and the existing Rust commands. Keep persisted download state in Rust.

FFmpeg and ffprobe are fetched by checksum from the pinned tools release before native builds. Contributors do not install them separately. Optional video tools are downloaded from Setup or Settings ▸ Tools after consent; settings apply immediately. Use a build version of at least `0.2.2` for runtime 1, for example `MARKETING_VERSION=0.2.2 Scripts/package_app.sh debug stable arm64`. The default development version is older and will correctly reject that runtime.

`OPUL_DATA_DIR` and `OPUL_DOWNLOAD_DIR` isolate local testing from your library. Use distinct data directories for stable and Nightly. The hidden `OPUL_YT_DLP=/absolute/path/to/yt-dlp` override is for testing the media engine; it bypasses managed selection and cannot verify Setup, signatures, updates, or recovery. It is not a supported app setting.

Read [the glossary](GLOSSARY.md), [tools architecture](docs/adr/0026-managed-video-tools.md), [module contract](docs/video-tools.md), and [release checks](docs/releasing.md) before changing tools. Test the public commands/snapshots, reducer, or existing mocked desktop bridge. Use generated CC0 fixtures or attributed public-domain/Creative Commons media. App-owned text says “video sites” and “video pages”; preserve upstream yt-dlp errors verbatim.

Run `vp check`, `vp test`, `vp exec tsc --noEmit`, `cargo test --workspace`, and `vp run test` (the package's Playwright script). `Scripts/test.sh` combines Rust tests, unit tests, frontend build, and Playwright. Packaging and network smoke tests are separate; record which app, architecture, macOS version, and commit each result covers.

Describe the user-visible behavior, tradeoffs, and verification in your PR. Contributions are licensed under the MIT License.
