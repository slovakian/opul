# Third-party notices

Opul is MIT licensed. It includes or downloads the following software.

## Bundled in Opul.app/Contents/Helpers

- **FFmpeg** (`ffmpeg`, `ffprobe`), licensed under the GNU Lesser General Public License 2.1 or later. Opul's build enables no GPL or non-free parts. The exact source code, the configure arguments, and the checksums of the binaries are published with each `media-tools-<n>` release at https://github.com/slovakian/opul/releases?q=media-tools. Its version is shown in Settings ▸ Media.
- **LAME** (libmp3lame, linked into FFmpeg), licensed under the GNU Lesser General Public License. Source code is published with the same release.
- **QuickJS** by Fabrice Bellard and Charlie Gordon, MIT license, built into Opul's sandboxed driver (`qjs`, source in `MediaTools/qjs-driver.c`). Source code is published with the same release.

You may replace the bundled FFmpeg with your own build in Settings ▸ Media ▸ Bundled Tools ▸ Choose FFmpeg….

## Downloaded after you agree

- **yt-dlp**, released into the public domain under the Unlicense. Opul does not bundle it. It downloads the official `yt-dlp_macos.zip` from https://github.com/yt-dlp/yt-dlp/releases only when you choose to, and checks its signature first.

## Linked into Opul

- **Sparkle**, MIT license.
- Rust crates listed in `Cargo.lock`, under their own licenses (mostly MIT or Apache 2.0), including rPGP (`pgp`) for checking yt-dlp's signature.
