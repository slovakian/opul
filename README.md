# Opul

Opul is a modern, fast download manager built with **Tauri v2** + **shadcn/ui** and a high-performance **Rust** download engine.

It organizes downloads in Workspaces, supports multiple saved folders and custom views (List & Table), and handles video/stream pages (via yt-dlp/ffmpeg), direct URLs, and BitTorrent downloads (powered by librqbit).

Opul is free and open source under the MIT License.

---

## Features

- **Tauri v2 + shadcn/ui Desktop Shell**: Fast, responsive, native desktop experience without startup flash, with external link routing and native dark/light theme support.
- **Workspaces & Folders**: Group downloads logically by topic, workspace, or destination directory.
- **BitTorrent Client**: Full BitTorrent support with piece tracking, DHT, peer/seed stats, tracker management, and customizable seed ratios.
- **Media Stream Extraction**: Probe and download video pages, streams, and audio with configurable formats (1080p, 720p, MP4, MP3, etc.).
- **List & Table Views**: Switch seamlessly between rich visual download cards and compact data table views.
- **Sorting & Grouping**: Group downloads by Status, Folder, or Source; sort by Added Order, Name, Status, or Source.
- **Audio Feedback**: Tactile interaction sounds for completions, failures, and actions.
- **Safe Trashing**: Remove items from Opul's library while keeping disk files, or send them safely to the macOS Trash.

---

## Tech Stack

- **Desktop Shell**: [Tauri v2](https://tauri.app) + [tauri-ui](https://github.com/agmmnn/tauri-ui)
- **Frontend UI**: React 19, TypeScript, Tailwind CSS v4, [shadcn/ui](https://ui.shadcn.com), Lucide icons, Sonner
- **Core Engine (Rust)**:
  - `crates/opul-core`: State machine, persistence, checkpointing, and media extraction
  - `librqbit`: Embedded BitTorrent engine
  - `reqwest` & `tokio`: Async HTTP & I/O
- **Native Host**: `src-tauri` with Tauri commands & capabilities

---

## Development & Build

### Prerequisites

- [Bun](https://bun.sh) (or Node.js 20+)
- [Rust](https://rustup.rs) (stable toolchain)
- macOS Command Line Tools

### Run in Development

```bash
bun install
bun run tauri dev
```

### Run Tests & Validation

```bash
# Run all unit tests, typechecks, and builds
Scripts/test.sh
```

### Build Production Application & DMG

```bash
bun run tauri build
```

This compiles optimized binaries and creates:
- `target/release/bundle/macos/Opul.app`
- `target/release/bundle/dmg/Opul_0.3.0_aarch64.dmg`

Or launch locally using:

```bash
Scripts/compile_and_run.sh
```

---

## Project Structure

```
├── crates/
│   └── opul-core/         # Rust download engine, state machine, and BitTorrent
├── src/                   # React 19 + shadcn/ui frontend
│   ├── components/        # Sidebar, Header, DownloadCard, DownloadTable, Modals
│   ├── hooks/             # useOpul reactive state hook
│   ├── lib/               # Tauri IPC client, audio feedback, formatters
│   └── types/             # TypeScript definitions
├── src-tauri/             # Tauri v2 native desktop host & IPC handlers
├── public/                # Sound effects, fallback artwork, and static assets
└── Scripts/               # Build, test, and run automation scripts
```

---

## License

MIT License. Bug reports and pull requests are welcome!
