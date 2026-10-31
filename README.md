# Opul

Opul is a download manager for macOS, built with Tauri, React, shadcn/ui on Base UI, and a Rust download core. Organize direct links, video pages, playlists, and BitTorrent downloads in emoji workspaces.

Folder workspaces receive one of 12 stable color treatments. Existing libraries receive their assignments on startup. Colors stay with each workspace through renames, reordering, theme changes, and restarts. Creation previews the exact appearance that will be saved. See [appearance behavior and verification](docs/workspace-appearances.md).

## Develop

Install Xcode Command Line Tools, a stable Rust toolchain, and Node.js 24 or newer.

```bash
npm ci
npm run tauri -- dev
```

Video pages need [yt-dlp](https://github.com/yt-dlp/yt-dlp). Audio conversion needs [FFmpeg](https://ffmpeg.org). Direct files and torrents work without either.

```bash
brew install yt-dlp ffmpeg
```

Opul detects Homebrew installations. Custom executable paths can be set in Settings.

## Verify and package

```bash
npx playwright install webkit
Scripts/test.sh
Scripts/package_app.sh debug
Scripts/smoke_test.sh
Scripts/launch.sh
```

For a release app and disk image:

```bash
Scripts/package_app.sh release
Scripts/package_dmg.sh
```

The scripts use Tauri's bundler and support standalone architecture targets (`--target aarch64-apple-darwin` or `x86_64-apple-darwin`), producing single-architecture installers (`Opul-arm64.dmg`, `Opul-x64.dmg`) and `Opul.app`. For development, use `npm run tauri -- dev`. `npm run dev` runs only the web interface; downloads require the native shell.

## Your library

The Tauri app opens the existing `~/Library/Application Support/Opul` library. The Rust core imports legacy `library.json` files and retains its checkpoint, backup, recovery, and exclusive-lock behavior. Quit an older Opul before launching this version.

Development and smoke tests can use isolated data:

```bash
OPUL_DATA_DIR=/tmp/opul-dev-state OPUL_DOWNLOAD_DIR=/tmp/opul-dev-downloads npm run tauri -- dev
```

All downloads, In progress, and Completed each remember their own layout, sort, and grouping across restarts. Workspaces and their saved folders continue to share workspace display settings. In progress and Completed have fixed status scopes; other views highlight the status filter button while a filter is active.

The new shell stores settings, including aggregate display preferences, in `desktop-preferences.json`. Old Swift preferences are not imported. This migration replaces the Swift interface and Sparkle updater. Updates currently require a manual download from [GitHub Releases](https://github.com/slovakian/opul/releases). The former guided browser extraction interface is not included; its Rust implementation remains.

## Project structure

- `frontend/`: React interface and official shadcn Base UI components
- `src-tauri/`: native shell, commands, settings, and packaging configuration
- `src/`: Rust state machine, persistence, torrent engine, and media engine
- `tests/ui/`: WebKit interaction and accessibility checks with mocked Tauri IPC
- `Scripts/`: test, package, and launch entrypoints
- `.agents/skills/shadcn/`: official shadcn skill installed for this repository

The interface follows sidebar-15 for workspaces, sidebar-10 for all popover menus, and sidebar-13 for settings. See [the migration decision](docs/adr/0004-tauri-interface.md). The desktop setup takes reference from [tauri-ui](https://github.com/agmmnn/tauri-ui).

Opul is free and open source under the MIT License. [Support development](https://github.com/sponsors/slovakian).
