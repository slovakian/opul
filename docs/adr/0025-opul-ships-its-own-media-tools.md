# 0025: Opul ships its own media tools

Status: accepted

Supersedes the tool-location paragraph of ADR 0021. Decided in issue #5.

Video downloads used to work only for people who had installed yt-dlp and FFmpeg with Homebrew. Opul now supplies its tools itself, and keeps the disk image small.

## What is bundled and what is downloaded

`Opul.app/Contents/Helpers` carries the tools that are small and change rarely:

- **FFmpeg and ffprobe**, our own minimal LGPL build: remuxing, MP3 (libmp3lame) and AAC audio, subtitle conversion and embedding, and thumbnail conversion. No video re-encoding and no GPL or non-free parts. yt-dlp needs ffprobe next to ffmpeg for audio extraction and thumbnails.
- **A sandboxed QuickJS driver**, installed as `qjs`, for the JavaScript challenges YouTube sets.

yt-dlp is not bundled. It changes far more often than Opul is released, and its macOS folder build is about 54 MB. Opul downloads it **only after the person agrees**, from the welcome sheet, the "Video downloads need yt-dlp" prompt, or Settings. Closing the welcome counts as Not Now: video features stay visible but locked until yt-dlp is downloaded.

The `media-tools.yml` workflow builds the bundled tools from pinned sources (`MediaTools/sources.env`) and publishes them, with their exact source tarballs, as a `media-tools-<n>` release. `Scripts/package_app.sh` downloads that release and refuses any file whose checksum differs from `MediaTools/release.env`.

## yt-dlp is verified before it runs

Opul takes the stable channel's `yt-dlp_macos.zip` only. It resolves the latest tag from the redirect of `releases/latest` (not the rate-limited API) and fetches every file from that one tag. `SHA2-256SUMS.sig` must verify against yt-dlp's release key, which is stored in Opul's code, and the zip must match its line in `SHA2-256SUMS`. The zip is extracted with `/usr/bin/ditto`, never a shell, and the extracted copy must answer `--version` before a `current` link is switched to it atomically.

A release that fails `--version` is recorded and never tried again; the previous copy stays in use. A checksum or signature failure is shown and never retried silently. Network errors are retried with backoff, 30 seconds, 2 minutes, 10 minutes and then hourly, but only once the person has agreed. Copies live in `~/Library/Application Support/Opul/Tools/yt-dlp/<version>/`, readable only by the person. Opul keeps the previous version for rollback and never deletes a version a running process uses.

Automatic updates are on by default and can be turned off in the welcome or Settings. Off means no automatic checks at all, the same as Opul's own update switch. Checks happen at launch and every 24 hours, and an update installs only while no video download is running. A manual check works either way.

## No detection, only explicit choices

Opul never searches for installed copies: no Homebrew paths, no `PATH`, not even during setup. A person who wants their own yt-dlp (for example a nightly build) or FFmpeg picks the file with Choose…, and Opul runs it once before saving the path. Opul doesn't update a yt-dlp the person provides, and if that copy stops working Opul says so and offers to download its own copy instead of switching silently.

## QuickJS instead of Deno

yt-dlp's default JavaScript runtime is Deno, which Cloudflare acquired in October 2026 with one more year of releases promised. Opul uses Bellard's QuickJS instead, built from source. Stock `qjs` always registers its `std` and `os` modules, which reach the file system and can start processes, so Opul ships its own small driver (`MediaTools/qjs-driver.c`). It evaluates the script yt-dlp writes as a classic script with only `print`, `console.log`, `performance.now` and `scriptArgs`, no module loader, a 512 MB memory limit and a stack limit, and it writes to stderr only on failure because yt-dlp treats any stderr output as an error. `Scripts/test_media_tools.sh` proves that `std`, `os`, `__loadScript` and `import()` are unreachable.

Every yt-dlp process gets `--no-js-runtimes --js-runtimes quickjs:<Helpers>/qjs`, so a Deno or Node that happens to be installed is never used.

## Tool changes apply to the next process

The Rust core resolves the tool paths each time it starts a yt-dlp process, not once at launch. Changing a path, downloading yt-dlp or installing an update takes effect for the next download without a relaunch, and downloads already running keep the version they started with. While yt-dlp is being downloaded, a pasted video link becomes an item that waits as "Getting video tools…" and starts on its own.

## Consequences

- The disk image grows by the compressed size of the three helpers and stays under the 25 MB target; the release reports its size and warns above it.
- Opul talks to GitHub for yt-dlp, and only after consent. The privacy page says so.
- Bumping FFmpeg, LAME or QuickJS means changing `MediaTools/sources.env`, running the workflow, and committing the new `MediaTools/release.env`.
- The format matrix in `src/media.rs` checks every download format against the bundled FFmpeg whenever the tools or the format arguments change.
