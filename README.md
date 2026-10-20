# Opul

Opul is a native download manager for macOS. It keeps downloads in Workspaces, supports saved folders and custom views, and handles video pages, direct URLs, and BitTorrent downloads.

Opul is free and open source under the MIT License. If it makes your day better, you can support its continued development through [GitHub Sponsors](https://github.com/sponsors/slovakian).

## Download

[Download Opul](https://github.com/slovakian/opul/releases/latest/download/Opul.dmg) (the newest release, always), or visit [opul.procka.org](https://opul.procka.org). Requires macOS 14 Sonoma or newer on Apple silicon or Intel.

Open `Opul.dmg` and drag Opul to Applications. Opul is not notarized by Apple yet, so macOS asks you to approve it the first time:

1. Open Opul from Applications. macOS says it can't verify the app. Close the warning; don't choose **Move to Trash**.
2. Open **System Settings → Privacy & Security**, scroll down to **Security**, and click **Open Anyway** next to the message about Opul.
3. Confirm when macOS asks again. It may ask for your password or Touch ID.

You only do this once. Opul then checks for updates and installs them by itself; you can change that in Preferences.

## Build

You need the Apple Command Line Tools or Xcode, Swift 6 or newer, and a stable Rust toolchain.

```bash
Scripts/test.sh
Scripts/compile_and_run.sh
```

Build a release app and disk image without launching it:

```bash
Scripts/package_app.sh release
Scripts/package_dmg.sh
```

These write `Opul.app` and `Opul.dmg`. Releases are published by the Release workflow; see [docs/releasing.md](docs/releasing.md).

## Project structure

- `Sources/Opul`: SwiftUI application
- `src`: Rust state machine and download engines
- `Sources/OpulCore`: generated UniFFI bindings checked into source control
- `Scripts`: build, test, packaging, and local launch commands
- `docs/adr`: technical architecture decisions

The project website lives separately in [slovakian/opul-website](https://github.com/slovakian/opul-website).

## Contributing

Bug reports and focused pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before starting a larger change.
