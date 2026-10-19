# Opul

Opul is a native download manager for macOS. It keeps downloads in Workspaces, supports saved folders and custom views, and handles video pages, direct URLs, and BitTorrent downloads.

Opul is free and open source under the MIT License. If it makes your day better, you can support its continued development through [GitHub Sponsors](https://github.com/sponsors/slovakian).

## Download

Download the latest build from [GitHub Releases](https://github.com/slovakian/opul/releases/latest).

Current releases are ad-hoc signed, not notarized, because the project does not yet participate in Apple's paid Developer Program. On first launch, macOS may require you to Control-click Opul, choose **Open**, and confirm. Only download builds from this repository and verify the published SHA-256 checksum.

Requires macOS 14 Sonoma or newer.

## Build

You need the Apple Command Line Tools or Xcode, Swift 6 or newer, and a stable Rust toolchain.

```bash
Scripts/test.sh
Scripts/compile_and_run.sh
```

Build a release app without launching it:

```bash
SIGNING_MODE=adhoc Scripts/package_app.sh release
```

The app bundle is written to `Opul.app`.

## Project structure

- `Sources/Opul`: SwiftUI application
- `src`: Rust state machine and download engines
- `Sources/OpulCore`: generated UniFFI bindings checked into source control
- `Scripts`: build, test, packaging, and local launch commands
- `docs/adr`: technical architecture decisions

The project website lives separately in [slovakian/opul-website](https://github.com/slovakian/opul-website).

## Contributing

Bug reports and focused pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before starting a larger change.
