# Releasing Opul

Opul packages with Tauri and [updates itself](app-updates.md) from the signed updater manifests these workflows publish. Video tools have their own consent and update lifecycle.

## Local packages

Install Xcode command-line tools, Rust, Vite+, Python 3, and the WebKit test browser. Fetch dependencies and validate before packaging:

```bash
vp install --frozen-lockfile
pnpm exec playwright install webkit
vp check
Scripts/test.sh
Scripts/test_packaging.sh
```

Build each architecture separately. Universal packages are rejected because each installer contains only its matching built-in FFmpeg and ffprobe. Use a version compatible with the signed runtime pointer (runtime 1 requires Opul 0.2.2):

```bash
MARKETING_VERSION=0.2.2 Scripts/package_dmg.sh release stable arm64
MARKETING_VERSION=0.2.2 Scripts/package_dmg.sh release stable x64
```

Install the matching Rust target with `rustup target add aarch64-apple-darwin x86_64-apple-darwin`. Intel is cross-compiled on Apple silicon. Rosetta can execute it locally; record that separately from a physical Intel test. `Scripts/package_app.sh debug stable arm64` builds just an app. Both scripts accept `nightly`, `--target` triples, and `MARKETING_VERSION`. Git release tags determine release versions; do not bump package manifests to publish.

The outputs are `Opul-arm64.dmg` and `Opul-x64.dmg`, or `Opul-Nightly-arm64.dmg` and `Opul-Nightly-x64.dmg`. The friendly `Opul.dmg`/`Opul-Nightly.dmg` alias refers to the last architecture built. Copy each app to a separate directory before another build replaces the root app.

Packaging verifies built-in checksums, licenses, signatures and architecture. `tools/verify_package.py` also inspects every native file's architecture and deployment target, verifies the app signature, excludes optional video executables, and emits exact app-file and DMG byte counts and hashes. Deployment metadata is evidence of the build target; it does not prove execution on macOS 13.

```bash
python3 tools/verify_package.py Opul.app arm64 --dmg Opul-arm64.dmg
Scripts/smoke_test.sh Opul.app
Scripts/smoke_video_tools.sh Opul.app
```

The last command saves a video before consent, sets up verified tools, downloads the CC BY 3.0 Blender trailer, removes/reinstalls tools while retaining output, then quits normally and relaunches the same temporary library. It records versions/output size, checks Time Machine exclusion, and verifies a Stopped checkpoint after each window-close shutdown. It uses the network and is separate from ordinary CI. It unsets `OPUL_YT_DLP` to require managed tools. Preserve the printed reports and diagnostic artifacts with the commit being tested.

## Tools and source releases

FFmpeg and ffprobe are built in; users install no external programs and configure no tool paths. Normal app builds fetch the matching immutable archive pinned by `tools/ffmpeg/release.env` and verify its checksum. `.github/workflows/ffmpeg.yml` rebuilds only changed source/recipe pins. Every app carries LGPL texts and the exact source/recipe pointer. See [built-in tools](built-in-tools.md) and [third-party notices](../THIRD_PARTY_NOTICES.md).

The optional Python runtime and bare yt-dlp script are downloaded only after consent in Setup or Settings ▸ Tools. Tool changes take effect immediately. The hidden `OPUL_YT_DLP` override exists only for development; it bypasses managed selection and is unsuitable for acceptance testing. See [contributor guidance](../CONTRIBUTING.md).

The [Python runtime workflow](python-runtime.md) produces immutable `python-runtime-<n>` pre-releases and the signed fixed `python-runtime` pointer. Opul signatures are mandatory; Apple Developer ID is optional. Runtime and compatibility-result releases must explicitly remain pre-releases with `--latest=false`, preserving the app updater's `latest.json`. Never overwrite numbered tools assets. Changing recipes or dependencies requires a new numbered release and updated license inventory, followed by signature, relocation, sandbox and permitted-content smoke checks for both architectures.

The [compatibility workflow](video-tools-compatibility.md) checks current stable yt-dlp every three days and nightly weekly. Its cron requires the workflow on the default branch. Work on `integration/58-opul-managed-tools` does not authorize merging it into main or claiming cron has run.

## Acceptance record

For each release candidate, record the exact pushed commit, app version/channel, installer/runtime sizes, SHA-256, signatures, architecture slices, deployment metadata, OS/hardware, and artifact links. Run the full checks against that commit, including `vp check`, `vp test`, `vp run test`, Rust workspace tests, offline tools contracts and packaging/native smoke checks.

Verify fresh consent, Not now/Esc, Turned off and restart; waiting video and playlist identity/destination/format; real permitted-content completion; manual and automatic updates; runtime rollback before its first successful Download; removal with partial output retained and re-setup. Check direct files and torrents throughout. The [packaged acceptance driver](packaged-video-journey.md) exercises these paths with production signed tools and real HTTP/BitTorrent fixtures. Distinguish mocked bridge and signed fixture evidence from a real packaged journey. Execute both packages on macOS 13 before closing the final distribution gate; current macOS and Rosetta alone cannot satisfy that requirement. [Issue 69 evidence](evidence/video-tools-69.md) tracks remaining gaps.

Audit app-owned copy, release notes and screenshots for “video sites”/“video pages” wording without site brands. Preserve upstream yt-dlp diagnostics. Examples and fixtures must be public domain or Creative Commons with attribution and source links.

## GitHub release

`.github/workflows/release.yml` supports a `v*.*.*` tag or manual dispatch with a patch/minor/major bump. The matrix publishes the two architecture DMGs and the friendly alias. It generates signed updater manifests with `Scripts/generate-updater-manifest.mjs` and verifies public asset checksums. Nightly uses an isolated app identity/data folder and defaults to stable yt-dlp, with an optional nightly script switch.

Optional Developer ID signing uses `DEVELOPER_ID_CERTIFICATE_P12` and `DEVELOPER_ID_CERTIFICATE_PASSWORD`. Notarization uses `NOTARY_APPLE_ID`, `NOTARY_TEAM_ID`, and `NOTARY_PASSWORD`. Tauri updater signing uses `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Both workflows fail without them, and the manifest generator rejects missing or placeholder signatures, so a release can never advertise an update that won't verify. Do not describe ad-hoc signed packages as notarized.

### Rotating the updater key

The public key in `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`) must match the private key in the secrets. Installed copies trust only the key they shipped with, so a rotation strands every earlier build: those users must download the next release by hand. Rotate only when the private key is lost or exposed:

1. Run `pnpm exec tauri signer generate -w <path outside the repo>` with a strong password.
2. Store the private key and password in the maintainers' password manager.
3. Set them with `gh secret set TAURI_SIGNING_PRIVATE_KEY < <key file>` and `gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, then delete the local copies.
4. Commit only the new `.pub` contents to `tauri.conf.json`, and say in the release notes that this release must be installed manually.

The current key id is `FD050D5EBFEE2E4E`. It replaced `D5A4676F417CDAA6`, whose private key was lost, so no Tauri build signed before the rotation can update itself.
