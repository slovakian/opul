# Runtime 2 publication for issue 69

[Runtime 2](https://github.com/slovakian/opul/releases/tag/python-runtime-2) was published by [the successful two-architecture workflow](https://github.com/slovakian/opul/actions/runs/42437895292) from `fd4fc86cfddff201fcbc9648256c63cd2aeb1c7b`. The fixed signed `python-runtime` pointer selects 2. Numbered runtime 1 remains unchanged. [App CI for the change](https://github.com/slovakian/opul/actions/runs/42437895225) passed all seven jobs.

The executable and wheel pins match runtime 1. The new archive adds the full curl-impersonate notice inventory and exact native-source provenance: ten upstream notice files, BoringSSL fiat-crypto LICENSE and AUTHORS, the pinned curl-impersonate CMake recipe, and the pinned curl_cffi binding recipe. All inputs and copied notice files have recorded SHA-256 values in `tools/python-runtime/runtime.lock.json`. Build and verification require all twelve notice/author files; missing, modified or symlinked notices fail validation.

Independent review compared the notices from both architecture-specific upstream bundles and verified the source recipes against the actual native dependencies: `libcurl/8.23.0-IMPERSONATE BoringSSL zlib/1.3.1 brotli/1.2.1 zstd/1.5.7 c-ares/1.34.9 AppleIDN nghttp2/1.63.0 ngtcp2/1.20.0 nghttp3/1.15.0 curl-impersonate/2.3.1`. BoringSSL's recorded source commit is `156c7b75ae9b8c3b3f847acf264f17594c3859fb`. [Third-party notices](../../THIRD_PARTY_NOTICES.md) describe the inventory and exact FFmpeg/LAME source information.

| Runtime 2 | Archive bytes | Sum of regular-file bytes after installation | Signed native files | Archive SHA-256 |
| --- | --: | --: | --: | --- |
| arm64 | 17,158,771 | 35,459,508 | 47 | `4300ae61f973623db21996f67343a6b37d441248ae34d60967598b77a3a9e0c1` |
| x64 | 17,670,641 | 35,177,335 | 49 | `1295279def62d2ed8bc174d4b742ca1cf231e89b2857ffd9f248023a467833b2` |

The signed release manifest and SHA256SUMS report these archive sizes/hashes. Installed byte counts were measured from the real packaged acceptance libraries; they exclude symlink storage, filesystem allocation overhead, the separately installed yt-dlp script and retained rollback versions.

Both published archives passed architecture, dependency, deployment-target, signature, relocation/import, certificate, QuickJS sandbox and actual permitted-content checks. The release logs are retained here: [arm64](video-tools-69-runtime-2-arm64.log), [x64](video-tools-69-runtime-2-x64.log). Each downloaded the 11,053,871-byte CC BY 3.0 Blender trailer. This validates current-host execution; deployment metadata of 13.0 does not establish execution on macOS 13. Signatures are mandatory Opul Ed25519 signatures and ad-hoc Apple signatures; Developer ID and notarization were not exercised.

The historical runtime-1 archive remains immutable and lacks this complete notice inventory. The current verifier rejects it explicitly. Its original recipe revision can reproduce the historical structural audit, which does not establish complete notices. Update acceptance reconstructs its authentic interpreter/script pair from the immutable signed release, then exercises the application's real production update to runtime 2. No source URL or trust override is used.
