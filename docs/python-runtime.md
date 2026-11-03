# Python runtime releases

The Python runtime is optional video tooling downloaded by Opul. It is never built or bundled by normal app CI, nightly builds, or app release builds. `.github/workflows/python-runtime.yml` builds only when `tools/python-runtime/runtime.lock.json` changes, on a runtime pull request, or on manual dispatch. Existing numbered releases are skipped. Any change to the inputs or recipe needs a new runtime number. Pull requests build and test without publishing.

The runtime contains Python from [python-build-standalone](https://github.com/astral-sh/python-build-standalone/releases/tag/20261026), wheel dependencies pinned to [yt-dlp 2026.10.26](https://github.com/yt-dlp/yt-dlp/blob/2026.10.26/pyproject.toml), and the Opul driver built against [Bellard QuickJS](https://bellard.org/quickjs/). The lock records exact URLs and SHA-256 hashes for each architecture, wheel, source archive, upstream dependency pins and Python license inventory. The Intel Python archive is upstream's cross build. The Intel QuickJS driver is cross-compiled on Apple silicon with `-arch x86_64`. No CPython build or Intel runner is needed.

The shipped layout is relative to the extracted runtime root:

```text
bin/python3 -> python3.13
bin/python3.13
bin/qjs
lib/python313.zip                    # pure standard library, pyc only
lib/python3.13/lib-dynload/           # retained even when empty
lib/python3.13/site-packages/         # wheel extensions, pyc, data, license metadata
licenses/                           # Python, bundled dependencies, QuickJS
runtime.lock.json
SOURCE.txt
BUILD-INFO.json                     # recipe commit, compiler and SDK versions
```

The archive extracts directly into `Tools/python-runtime/<number>/`; it has no extra top-level directory. Both `arm64` and `x64` are single-architecture archives, compatible with macOS 13+. Verification inspects **every** Mach-O file, including wheel extensions, rejects extra slices and deployment targets above 13, checks load dependencies and code signatures, and executes the result. Execution on the build host and through Rosetta does not substitute for a test on a physical macOS 13 machine.

There is no pip, Tk, IDLE, test suite, mutagen, requests or urllib3. The one additional wheel, pycparser, is a cffi dependency pinned by yt-dlp. Wheels are extracted without running install scripts, `.pth` files or entry points. Python's default TLS context uses `/etc/ssl/cert.pem`; curl_cffi explicitly uses the included certifi CA bundle. Standard-library bytecode is compiled by the exact target Python, with stable filenames, hash invalidation, and normalized zip metadata. The empty extension directory prevents Python's platform-dependent-libraries warning.

The QuickJS driver links only the engine sources. It does not link `quickjs-libc`, register `std` or `os`, or install a module loader. Its only host globals are `print`, `console.log`, `performance.now`, and an empty `scriptArgs`. The host reads exactly the file passed by yt-dlp's `qjs --script <file>` command. JavaScript cannot read other files, start processes, fetch URLs or load modules. Promise jobs run with error propagation. A 512 MiB allocation cap and 30-second interrupt bound script execution. `test_sandbox.py` exercises the actual driver, including successful promise jobs, attempted host access and static/dynamic imports.

## Build and verify

On an Apple silicon Mac with Xcode command-line tools, Rosetta, Python 3.13 or later for the build scripts, and OpenSSL 3 on `PATH`:

```sh
vp install
export PATH="$(brew --prefix openssl@3)/bin:$PATH"
python3 tools/python-runtime/test_release.py
python3 tools/python-runtime/build.py arm64 .build/python-runtime/out
tools/python-runtime/sign.sh .build/python-runtime/out/arm64
python3 tools/python-runtime/verify.py .build/python-runtime/out/arm64 arm64
python3 tools/python-runtime/build.py x64 .build/python-runtime/out
tools/python-runtime/sign.sh .build/python-runtime/out/x64
python3 tools/python-runtime/verify.py .build/python-runtime/out/x64 x64
python3 tools/python-runtime/release.py .build/python-runtime/out --key /secure/path/runtime-key.pem
```

The final command packs both stages, extracts each into a temporary location, repeats the native-code and sandbox checks, makes a real download with the pinned bare yt-dlp script, then signs the actual archives, checksum file and pointer. Existing archives in the output folder are reverified rather than silently replaced. Use a fresh output directory after changing pins. Archive creation normalizes ordering, timestamps, owner, modes and gzip headers. Reproduction requires the same SDK/compiler and signing mode; Apple timestamped Developer ID signatures are deliberately not byte-identical. The bytecode and archives omit local build paths.

The smoke test downloads the [Big Buck Bunny trailer](https://media.w3.org/2010/05/bunny/trailer.mp4), copyright Blender Foundation, under [Creative Commons Attribution 3.0](https://peach.blender.org/about/). It forces curl_cffi impersonation, explicitly selects the bundled QuickJS driver, disables plugins and user configuration, and checks yt-dlp's diagnostics for the pinned version, supported curl_cffi and QuickJS. The bare script already contains yt-dlp-ejs. yt-dlp is a smoke-test input and is **not** inside the runtime archive. The recipe downloads inputs only by their recorded hashes; it never resolves a dependency version during a build.

## Signing and publication

Every artifact requires an Opul Ed25519 signature, independently of Apple signing. The GitHub Actions secret `OPUL_RUNTIME_SIGNING_KEY` holds a dedicated private PKCS#8 PEM key. It must match [signing-public.pem](../tools/python-runtime/signing-public.pem), or publication fails. That public PEM is the trust anchor to embed in the video-tools installer. Do not bootstrap trust from the public-key asset downloaded alongside an archive.

`.sig` files are raw 64-byte Ed25519 signatures over the **exact file bytes**, without hashing the message first. JSON is UTF-8 with a trailing newline; verify those downloaded bytes before parsing. SHA-256 is a separate integrity check after signature verification. Rust consumers can decode the PEM's SubjectPublicKeyInfo to its 32-byte Ed25519 public key and use `ed25519-dalek` strict verification. This is separate from yt-dlp's own signature format and key.

Verify from a checkout whose public key you trust:

```sh
openssl pkeyutl -verify -rawin -pubin \
  -inkey tools/python-runtime/signing-public.pem \
  -in python-runtime.json -sigfile python-runtime.json.sig
openssl pkeyutl -verify -rawin -pubin \
  -inkey tools/python-runtime/signing-public.pem \
  -in python-runtime-1-arm64.tar.gz -sigfile python-runtime-1-arm64.tar.gz.sig
shasum -a 256 -c SHA256SUMS
```

Without an Apple account, every Mach-O file is ad-hoc signed. With `DEVELOPER_ID_CERTIFICATE_P12` and `DEVELOPER_ID_CERTIFICATE_PASSWORD`, the workflow imports a temporary keychain and signs all code with that Developer ID, timestamp and hardened runtime. This mode also requires `NOTARY_APPLE_ID`, `NOTARY_PASSWORD` and `NOTARY_TEAM_ID` and waits for successful notarization before packaging. The raw executable archive cannot be stapled as an app bundle; Apple's online ticket lookup applies. No unsigned or failed-notarization Developer ID build is published. The keychain is removed afterwards.

To publish, push the lock change to `integration/58-opul-managed-tools` or manually dispatch the Python runtime workflow on that branch. Maintainers can also add `--publish` to the final local command above after committing the recipe. This creates an immutable `python-runtime-<number>` pre-release containing both archives, their `.sig` files, `SHA256SUMS`, `SHA256SUMS.sig`, the signed pointer snapshot, public key and verification logs. An existing numbered release, including a draft, is never overwritten. A publication failure leaves the fixed pointer unchanged until both remote archives have been downloaded and their sizes, hashes and signatures verified.

The fixed `python-runtime` pre-release holds `python-runtime.json` and `python-runtime.json.sig`. These are the only mutable discovery assets. Both numbered and pointer releases explicitly use `--prerelease --latest=false`; the helper rejects non-runtime tags. Tools releases must never take Latest from the app's `latest.json`. Replacement of the two pointer files is serialized by the workflow concurrency group. The files are separate GitHub assets, so consumers should retry a mismatched pair during replacement. Recover a partial publication by verifying the existing numbered assets and restoring its signed pointer snapshot; never overwrite a numbered archive.

## Installer handoff

Fetch these fixed URLs directly over HTTPS, without using the GitHub API:

- [python-runtime.json](https://github.com/slovakian/opul/releases/download/python-runtime/python-runtime.json)
- [python-runtime.json.sig](https://github.com/slovakian/opul/releases/download/python-runtime/python-runtime.json.sig)

The signed pointer schema is:

```json
{
  "schema_version": 1,
  "runtime": 1,
  "minimum_opul_version": "0.2.2",
  "architectures": {
    "arm64": {
      "url": "https://github.com/slovakian/opul/releases/download/python-runtime-1/python-runtime-1-arm64.tar.gz",
      "sha256": "<64 lowercase hex characters>",
      "size": 0,
      "signature_url": "https://github.com/slovakian/opul/releases/download/python-runtime-1/python-runtime-1-arm64.tar.gz.sig"
    },
    "x64": {
      "url": "https://github.com/slovakian/opul/releases/download/python-runtime-1/python-runtime-1-x64.tar.gz",
      "sha256": "<64 lowercase hex characters>",
      "size": 0,
      "signature_url": "https://github.com/slovakian/opul/releases/download/python-runtime-1/python-runtime-1-x64.tar.gz.sig"
    }
  }
}
```

The example uses zero only as a placeholder; published `size` is the exact compressed archive byte count. Select the architecture before adding the separately downloaded yt-dlp script's size for Setup. Verify signature, checksum and size before unpacking, then health-check the interpreter/script pair before switching `state.json`. The public key, schema and executable paths are the contract for #61. The previous runtime stays available for rollback. License notices travel with the runtime; certifi includes its unmodified MPL-2.0 notices and upstream source link.

## Runtime 1 verification

Published on 2026-11-03 from integration commit `841d4aae2219ee123e3d616ca579797e04f9c3f8`. The [publication workflow](https://github.com/slovakian/opul/actions/runs/41598244426) passed both builds, relocated archive checks, real downloads, Opul signatures and direct-URL verification. [Runtime 1](https://github.com/slovakian/opul/releases/tag/python-runtime-1) and the [fixed pointer](https://github.com/slovakian/opul/releases/tag/python-runtime) are pre-releases. The app's Latest release remained `v0.2.1` after publication.

| Architecture | Archive bytes | SHA-256 |
| --- | --: | --- |
| arm64 | 17143107 | `8d10c2129fed28c8a728d309b69ea34b62c1729167355e50e1c2bb810655b522` |
| x64 | 17654743 | `4bef61f406eabb108ea50f303353a406876b2b60ed2f25f5ca2689bac2afd244` |

The raw 32-byte Ed25519 verification key, hex encoded, is `065404daa6f716a3d221b6c5e13a44d5e31ac9d0f9cdb9590eb4682f3d4a1b55`. It is the same key as `signing-public.pem`. Signatures are raw bytes, not ASCII armor or base64. The minimum Opul version is `0.2.2`.

An independent post-publication check fetched the fixed JSON and signature without the GitHub API, verified against the checked-in key, downloaded both archives by their advertised URLs, verified their signatures, hashes and sizes, and only then extracted and executed them. Both published runtimes downloaded the 11,053,871-byte permitted trailer. The arm64 archive contains 47 native files; x64 contains 49. All passed code-signature, single-architecture and deployment-target checks. Both ran seven sandbox tests and the Python/package/CA checks. Immutable evidence logs are attached as [verification-arm64.log](https://github.com/slovakian/opul/releases/download/python-runtime-1/verification-arm64.log) and [verification-x64.log](https://github.com/slovakian/opul/releases/download/python-runtime-1/verification-x64.log).

Local repository validation passed `vp check`, TypeScript checking, 37 unit tests, 124 Rust tests and packaging checks. The full UI run passed 193 tests; its two failures both passed a focused single-worker rerun. The four release-contract tests passed, including signature tampering, wrong-key rejection and deterministic archive metadata. A second clean local arm64 build produced an identical archive before publication. Separate standards and spec reviews were completed; the notarization acceptance gate and synchronous/asynchronous curl_cffi certificate selection were corrected and reverified.

[App CI for the implementation commit](https://github.com/slovakian/opul/actions/runs/41598244390) also passed all seven jobs: checks, Rust tests, both UI shards, both single-architecture DMGs and the packaged-app launch smoke test.

Runtime 1 uses ad-hoc Apple signatures and mandatory Opul Ed25519 signatures. Developer ID signing and notarization are implemented but were not exercised because no Apple credentials are configured. Native execution was tested on current macOS and through Rosetta; a physical macOS 13 machine was not available. The compatibility evidence is each native file's inspected deployment target plus successful current-host execution.
