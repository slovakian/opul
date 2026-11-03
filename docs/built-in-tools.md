# Built-in FFmpeg and ffprobe

`tools/ffmpeg/sources.env` pins the FFmpeg and LAME sources and tools release. `tools/ffmpeg/release.env` pins each published binary archive by SHA-256. Tauri's build and development hooks run `Scripts/fetch_ffmpeg.sh`, selecting `TAURI_ENV_TARGET_TRIPLE` or the host for development. A single architecture is staged under `src-tauri/resources/media-tools/` and mapped into app Resources. Archive downloads are cached in `.build/ffmpeg-cache/`; every use verifies the archive checksum, executable checksums, architecture, deployment target and system-only dynamic dependencies. Packaging checks the installed copies again. Separate working directories are required for simultaneous packaging targets.

To update:

1. Change the source pins or `tools/ffmpeg/build.sh`, and increment `FFMPEG_RELEASE` in `sources.env`. A recipe change always needs a new release.
2. Push the source pins. `.github/workflows/ffmpeg.yml` builds only for pin changes (or an explicit workflow dispatch), and skips an existing release. It cross-compiles both architectures on Apple silicon and requires Intel execution through Rosetta. Pull requests build and test without publishing.
3. Copy the two archive hashes from the workflow's `SHA256SUMS` into `release.env`. Never overwrite a published release. Committing new binary hashes does not rebuild the tools.
4. Run `Scripts/fetch_ffmpeg.sh arm64`, then `Scripts/test_ffmpeg.sh src-tauri/resources/media-tools`. Repeat for `x64`. Run normal repository checks and package both single-architecture installers.

For an isolated build from source, run `tools/ffmpeg/build.sh arm64 .build/ffmpeg-release` or use `x64`. This is a maintainer operation, not a packaging dependency. Each output includes LGPL texts, an exact-source pointer, configure output, deployment metadata and executable checksums. Binaries are ad-hoc signed. When `APPLE_SIGNING_IDENTITY` is configured, the fetch hook first verifies the published bytes, then signs both staged executables with that identity, a timestamp and hardened runtime before Tauri seals the app. It records the upstream archive hash in `ARCHIVE.txt` and regenerates the staged executable checksums, since signing changes their bytes. The application retains its existing optional notarization flow.

The smoke suite uses only generated CC0 fixtures. It checks remuxed H.264 and AAC, MP3, AAC/M4A, VTT to SRT, SRT embedding into Matroska and MP4, and PNG/WebP to JPG. ffprobe verifies every resulting codec. Full video re-encoding is not included. The deployment target is checked as 13.0; testing on a macOS 13 machine is a separate compatibility check from execution on the current runner.

FFmpeg's [license guidance](https://ffmpeg.org/legal.html) and LAME's [license notes](https://lame.sourceforge.io/license.txt) explain the upstream licenses. The release hosts the exact unmodified sources beside the binaries. The recipe excludes the GPL LAME decoder and FFmpeg GPL/nonfree components. FFmpeg is a trademark of Fabrice Bellard.
