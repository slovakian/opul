# Nightly video tools and compatibility checks

Both Opul channels default to stable yt-dlp. Only Opul Nightly exposes **Use yt-dlp nightly builds** in Settings ▸ Tools. The backend rejects that operation in stable Opul. The preference commits immediately to `Tools/state.json`, outside Settings' Save flow. Changing it is explicit consent to install the selected source, even with automatic updates off. Before setup it only saves the choice; it makes no network request.

The switch uses the existing verified installer. Stable resolves `yt-dlp/yt-dlp/releases/latest`; nightly resolves `yt-dlp/yt-dlp-nightly-builds/releases/latest`. All three assets (bare `yt-dlp`, `SHA2-256SUMS`, `SHA2-256SUMS.sig`) come from that resolved repository/tag. Both use the embedded yt-dlp public key, signature-before-checksum verification and the same installed-runtime health check. Activation waits for active tool users. A failed candidate leaves the working pair available. A channel change cancels obsolete work and clears source-specific availability, compatibility holds and script integrity skips. An interrupted switch resumes on launch; switching back permits an older stable version.

Installed pairs record their yt-dlp channel. Stable scripts retain `yt-dlp/<version>/yt-dlp`; nightly scripts use `yt-dlp/nightly-<version>/yt-dlp`. Retention and runtime rollback preserve channel identity. Notifications include channel identity, so a version shared by both sources cannot suppress a notification. Opul Nightly compares the runtime's minimum Opul version with its base app version, allowing a build such as `0.2.2-nightly.20261108.1` to use a runtime requiring `0.2.2`.

The shell continues to resolve `Opul/Tools` and `Opul Nightly/Tools` under Application Support, with independent preferences and timers. `OPUL_DATA_DIR` is an exact data-directory override; `Tools` lives beneath it. Use different override directories when running both channels together. A directory lock rejects simultaneous ownership, and stable Opul refuses state containing a nightly selection or rollback pair.

## Scheduled checks

[The dedicated workflow](../.github/workflows/tools-compatibility.yml) runs stable checks every three days and nightly checks weekly, for arm64 and x64. A daily UTC tick gates stable work by elapsed days from 2026-01-01, maintaining an exact three-day interval across month boundaries. The Sunday tick selects nightly only. Manual dispatch checks both sources. GitHub schedules become active when the workflow reaches the repository's default branch; development and validation stay on the integration branch.

The runner verifies the current published runtime pointer and its archive with Opul's key, then verifies yt-dlp's signed checksums and bare script with the embedded upstream key. It executes the runtime's interpreter, verifies the requested CPU architecture, runs `--version` and verbose request-handler/JavaScript-engine health checks, then downloads the CC BY 3.0 Big Buck Bunny trailer from W3C. x64 runs through Rosetta on Apple silicon. These network integration checks run only in this dedicated workflow. Normal CI runs the offline contract tests.

A `tools-compatibility` pre-release holds one result JSON per runtime-pointer digest, runtime number, yt-dlp channel/tag and architecture. Existing results skip execution. The workflow serializes runs and matrix entries, scans all issue states for an exact result marker before creating a failure issue, and records both successful and failed tested pairs. Network transport failures leave the pair eligible for retry. Publication always creates a pre-release with `--latest=false`; it never edits an app release or updater manifest.

Failure issues include versions, CPU, pointer digest, diagnostics, a workflow/artifact link and a reproduction command. Artifacts include the signed pointer, `plan.json`, result JSON and verification/health/smoke logs. To reproduce an archived pair without writing GitHub state:

```sh
python3 tools/compatibility/check.py --channel nightly --arch arm64 \
  --plan /path/to/artifact/plan.json --local --output /tmp/opul-compatibility-repro
```

The output directory should be empty. The source checkout needs Python 3.13+, OpenSSL 3, GnuPG (`gpg` and `gpgv`), and the pinned built-in tools for the target architecture (`Scripts/fetch_ffmpeg.sh arm64` or `x64`). Omitting `--plan` resolves the current published pair. Local execution suppresses result publication and failure issues.
