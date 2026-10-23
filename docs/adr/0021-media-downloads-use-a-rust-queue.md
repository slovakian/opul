# 0021: Media downloads use a Rust queue

Status: superseded in ownership scope by ADR 0024; tool location superseded by ADR 0025

Opul supports ordinary media pages, direct media URLs, playlists, and user-guided discoveries through one Rust-owned queue. SwiftUI presents the queue, but does not launch or supervise downloader processes.

The Rust core probes supported pages with `yt-dlp`, expands playlists into individual items, persists transfer state, limits concurrency and bandwidth, and supervises pause, resume, cancel, retry, and post-processing. It uses FFmpeg when the chosen format requires merging, audio conversion, subtitle conversion or embedding, or thumbnail conversion. Compatible direct video, audio, image, and subtitle discoveries use a native Rust HTTP path with resumable partial files, scoped cookies and headers, bounded redirects, media-type validation, and collision-safe finalization. Requests that require format selection or post-processing continue through `yt-dlp`.

This boundary keeps transfer state and process lifecycle out of view code. It also lets the app restore interrupted work consistently with its existing Rust torrent engine.

~~`yt-dlp` and FFmpeg may be placed in the app's Resources directory, selected explicitly in Media preferences, installed in standard Homebrew locations, or available on `PATH`. Opul reports the discovered versions.~~ Superseded by ADR 0025: Opul bundles FFmpeg, ffprobe and a sandboxed QuickJS driver, downloads yt-dlp after the person agrees, and uses only paths chosen explicitly in Settings, never Homebrew or `PATH`. Torrent transfers and direct guided files remain available when `yt-dlp` is unavailable.

The queue never executes a shell. Each process argument is passed directly to `Command`, user filename templates cannot escape the chosen destination, and the workspace library, queue, browser cookies, and private header sidecars use owner-only file permissions under Opul's Application Support directory. Per-transfer paths are re-derived from engine-issued UUIDs before reads or deletion rather than trusted from persisted state. Credential sidecars are deleted after a successful transfer.
