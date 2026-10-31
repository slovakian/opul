# Releasing Opul

Opul now packages with Tauri. SwiftPM, UniFFI bindings, and Sparkle are no longer part of the build. The first Tauri release must be installed manually; there is currently no in-app updater or Sparkle appcast.

## Local packages

```bash
npm ci
npx playwright install webkit
Scripts/test.sh
Scripts/package_app.sh debug
Scripts/smoke_test.sh
```

For dedicated single-architecture disk images (reducing installer size by ~50% down to ~12–25MB):

```bash
Scripts/package_dmg.sh --target aarch64-apple-darwin  # Opul-arm64.dmg (Apple Silicon)
Scripts/package_dmg.sh --target x86_64-apple-darwin   # Opul-x64.dmg (Intel)
```

For a universal release, install both Rust targets and build:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
ARCHES="arm64 x86_64" MARKETING_VERSION=0.2.0 Scripts/package_dmg.sh
```

The output files are named with distinct architecture suffixes (`Opul-arm64.dmg` and `Opul-x64.dmg`), with a matching `Opul.dmg` maintained for backwards compatibility. The version goes into Tauri's configuration at build time; Git tags remain the release version source.

## GitHub release

The Release workflow (`.github/workflows/release.yml`) supports dual release triggers:

1. Pushing a Git version tag matching `v*.*.*`.
2. Manual `workflow_dispatch` selecting a version bump (`patch`, `minor`, `major`).

The workflow compiles and packages `Opul-arm64.dmg` (Apple Silicon) and `Opul-x64.dmg` (Intel) concurrently across matrix runners without running redundant Playwright browser tests (guaranteed by PR CI). A matching `Opul.dmg` is published for backwards compatibility with direct links.

Production updater manifests (`latest.json`) pointing to the dual-architecture packages and cryptographic signatures are automatically generated via `Scripts/generate-updater-manifest.mjs` and published with each release. A concluding release verification step validates the public download integrity and checksums of the release assets.

Optional Developer ID signing uses the `DEVELOPER_ID_CERTIFICATE_P12` and `DEVELOPER_ID_CERTIFICATE_PASSWORD` secrets. For notarization, configure `NOTARY_APPLE_ID`, `NOTARY_TEAM_ID`, and `NOTARY_PASSWORD`. Tauri updater signing keys use `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
