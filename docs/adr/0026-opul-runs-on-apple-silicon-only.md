# 0026: Opul runs on Apple silicon only

Status: accepted

Decided in issue #5.

Opul's app, its Rust core, and every tool it bundles are built for arm64 only. The release no longer builds a universal app.

Bundling our own FFmpeg, ffprobe and QuickJS driver (ADR 0025) would mean building, testing and shipping each one twice to keep Intel support, and a universal FFmpeg roughly doubles the part of the disk image it takes. Sparkle 2.10 can only express "Apple silicon only" in an appcast (`sparkle:hardwareRequirements`), not "Intel only", and there are no Intel installs of Opul to keep updating. macOS 26 is also the last macOS release for Intel Macs.

Intel support comes back only if people ask for it.

## Consequences

- `release.yml` builds with `ARCHES="arm64"`, installs only the `aarch64-apple-darwin` Rust target, and checks that the app and core are arm64. `Scripts/package_app.sh` rejects other architectures.
- The README and the download page say "Requires a Mac with Apple silicon."
- `LSMinimumSystemVersion` stays at macOS 14.
