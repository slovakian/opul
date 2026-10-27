# Releasing Opul

Opul now packages with Tauri. SwiftPM, UniFFI bindings, and Sparkle are no longer
part of the build. The first Tauri release must be installed manually; there is
currently no in-app updater or Sparkle appcast.

## Local packages

```bash
npm ci
npx playwright install webkit
Scripts/test.sh
Scripts/package_app.sh debug
Scripts/smoke_test.sh
```

For a universal release, install both Rust targets and build:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
ARCHES="arm64 x86_64" MARKETING_VERSION=0.2.0 Scripts/package_dmg.sh
```

The output is `Opul.dmg`. The version goes into Tauri's configuration at build
time; Git tags remain the release version source.

## GitHub release

Run the Release workflow and select the version bump. It tests the code, builds
a universal app and DMG, verifies the code signature, and publishes `Opul.dmg`
with release notes. The website's existing latest-download link keeps working.

Optional signing uses the existing `DEVELOPER_ID_CERTIFICATE_P12` and
`DEVELOPER_ID_CERTIFICATE_PASSWORD` secrets. For notarization, configure
`NOTARY_APPLE_ID`, `NOTARY_TEAM_ID`, and `NOTARY_PASSWORD`.

A future Tauri updater needs a Tauri signing key and update manifest. Do not reuse
the Sparkle feed or claim automatic updates until those are configured.
