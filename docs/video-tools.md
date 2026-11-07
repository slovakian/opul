# Video tools

Issue #61 supplies initial setup from Settings. Direct files and torrents need no video tools. Python, curl_cffi and sandboxed QuickJS are one Opul runtime; the official bare yt-dlp script is the other part. They become usable together after verification and a health check.

## Public interface

`opul_core::video_tools::VideoTools` owns installation, selection and preferences. The shell opens it with `VideoToolsConfig { root, opul_version, built_in_tools }`. `root` is the channel-resolved Application Support directory plus `Tools`, including any `OPUL_DATA_DIR` override. Stable and Nightly each construct a separate instance. Both currently use stable yt-dlp.

- `snapshot()` reads `VideoToolsSnapshot`; `subscribe()` returns a watch receiver with the current value and pushed changes. `sequence` increases within a module instance. Slow receivers may coalesce progress. Register listeners before reading, and reject older sequence numbers within that instance.
- `execute(operation)` accepts `SetUp`, `Retry`, `CancelSetup`, `TurnOff`, `SetKeepUpToDate { enabled }`, and `RefreshPublishedSize`. It returns the current snapshot. Setup runs in the background. Duplicate setup commands during setup or Ready do nothing. Preferences commit before the command returns. Retry and Setup both express consent, including from Turned off.
- `invocation()` returns a `VideoToolsInvocation` only in Ready. It supplies the interpreter, bare script, QuickJS and built-in tools directory. MediaEngine reads this dynamically before probes and when constructing a transfer worker. Each worker retains its selected command for that transfer. Paths never cross the frontend bridge.
- `diagnostics()` includes the state, detailed error, installed versions, chip and Opul version for Copy details.

The snapshot has exactly one `state`: `not_set_up`, `turned_off`, `setting_up`, `ready`, or `not_working`. It also carries `keep_up_to_date` and nullable `published_size`. Setting up has Python and yt-dlp steps, with `pending`, `downloading`, `verifying`, `installing`, `done`, or `failed`, downloaded bytes and nullable total bytes. A failed initial setup becomes Not working with a download, verification or health-check reason. Missing or unlaunchable installed files become Not working with `stopped_working` at restart.

Ready carries the runtime number, Python version, yt-dlp version, nullable `last_checked` Unix timestamp and exactly one update flag: `none`, `available { version }`, `updating { version, progress }`, `failed { version }`, or `waiting_for_runtime { version }`. This slice activates with `none`; it defines and renders the other flags for #65. An update must preserve Ready and its selected pair until another candidate passes verification and health checks.

Tauri exposes `tools_snapshot` with `{ video, built_in: { ffmpeg, ffprobe } }`, `video_tools_execute`, `video_tools_details`, and pushed `video-tools` events. Settings subscribes before its initial read. Tools controls commit immediately and do not participate in the other Settings sections' draft or Save flow. Settings requests published metadata when Not set up; this fetches the signed pointer, the release tag and signed checksum metadata, then HEADs the script for its size. It never fetches executable payloads or grants consent. Failed metadata lookup leaves the `about 23 MB` fallback. Opening the module itself performs no network request.

## Verification and pairing

1. Fetch the fixed Opul pointer and its raw Ed25519 signature. Verify exact JSON bytes before parsing. Retry a mismatched pointer/signature pair once, since publication replaces two assets.
2. Enforce schema 1, the minimum Opul version and the host architecture. Runtime URLs must belong to that runtime's immutable numbered release. Runtime 1 requires Opul 0.2.2; older development builds reject setup with an update explanation.
3. Resolve yt-dlp's HTTPS `releases/latest` redirect once. Fetch `SHA2-256SUMS`, its detached OpenPGP signature, and the bare script from that resolved tag. Verify the manifest with the embedded yt-dlp key before reading its script checksum. Never fetch a key beside the payload.
4. Stage the runtime archive under a unique `.install-<uuid>` directory. Verify its Opul signature before SHA-256 and exact size, then unpack with bounded expanded size and path/link containment checks. Verify the script checksum against the authenticated manifest. A signature/checksum failure discards that payload and retries it once; a second failure never executes or selects it.
5. Run the candidate interpreter with `-I`, its script, `--ignore-config`, `--no-plugin-dirs`, `--no-cache-dir`, `--no-update`, explicit sandboxed QuickJS, and explicit bundled FFmpeg/ffprobe directory. `--version` must report the exact resolved tag. A separate network-free `--verbose --list-impersonate-targets` invocation must report that version, a Python version, supported curl_cffi and QuickJS, and no Python deprecation warning. Both subprocesses have timeouts and are killed if setup is cancelled. `--version` alone exits before initializing the handlers, so it cannot establish compatibility.
6. Move both verified parts into their version directories, then atomically replace `state.json`. Only the committed selection becomes Ready. A cancelled operation cannot commit after a newer command.

The Opul raw Ed25519 trust anchor is embedded in `install.rs` and corresponds to `tools/python-runtime/signing-public.pem`. The yt-dlp OpenPGP trust anchor is embedded from `src/video_tools/keys/yt-dlp.asc`, retrieved from [yt-dlp's public key](https://github.com/yt-dlp/yt-dlp/blob/master/public.key). [yt-dlp's installation documentation](https://github.com/yt-dlp/yt-dlp#release-files) specifies OpenPGP checksum signatures. Its key format differs from Opul's raw detached signatures. Fixture tests generate independent test keys for both projects.

## Storage and lifetime

`Tools/state.json` stores `installed`, `keep_up_to_date` and `turned_off`. Installed files live at `python-runtime/<number>/` and `yt-dlp/<tag>/yt-dlp`. The shell's channel resolver chooses `Application Support/Opul` or `Application Support/Opul Nightly`, or the exact `OPUL_DATA_DIR` override. A file lock prevents multiple module instances from writing one Tools root. Setup excludes Tools from Time Machine with the system `tmutil` before downloading. Failed or cancelled staging directories are removed; stale staging directories are also removed when opening the module.

A failed candidate leaves the committed selection unchanged. Turn off is the initial suppression choice, not removal of an active Ready pair. Removal, including stopping transfers and optionally turning off, belongs to #66. `OPUL_YT_DLP` is the sole hidden developer override. It bypasses managed command selection, never appears in Settings, and is rejected by the managed-tools smoke harness. Homebrew, PATH discovery, path preferences and the legacy tool-status API are gone.

## Verification

`cargo test -p opul_core video_tools` drives public commands against local HTTP fixtures with generated Ed25519/OpenPGP keys and a fixture interpreter. It checks consent and restart preferences, tampered metadata/payloads before execution, minimum app version, accepted/rejected health checks, cancellation, committed selection, restart validation and the arguments actually supplied to an already-open MediaEngine. `vp run test tests/ui/video-tools.spec.ts tests/ui/built-in-tools.spec.ts` exercises the mocked desktop bridge, pushed snapshots, immediate preferences, all initial states, Ready flags and axe audits.

For a real packaged check, build an app with version at least 0.2.2, then run `Scripts/smoke_video_tools.sh <app-path>`. It launches that app with isolated data/download directories, expresses consent through the module, verifies published assets, probes and downloads the permitted W3C sample through the real application state machine and MediaEngine, records versions and output size, and checks Time Machine exclusion. This test uses the network and is separate from normal CI.

## Handoff

- #62 adds range resume, reachability and retry scheduling, more startup cleanup/pruning, and broader recovery cases. Initial verification retry, cancellation, startup health checking and copied diagnostics are present here.
- #63 consumes module snapshots to represent waiting Downloads in the application reducer and projection, and supplies consent at video submission. This slice reports unavailable tools as a normal dependency error.
- #64 adds first-launch Setup, dismissal memory, the Help menu and sidebar progress. Reuse the public operations, snapshot and `VideoToolsProgress`; do not create a second setup owner.
- #65 adds manual update operations, idle-transfer coordination, health-check pairing against newer runtimes, retained versions, rollback and timestamps. #67 adds scheduled checks and update notifications. The exclusive Ready flag and immediate preference are established here; no updater runs in this slice.
- #68 adds Nightly's yt-dlp source switch and compatibility workflow. Stable/Nightly data and preferences are already isolated.
- #66 adds Remove and in-flight transfer handling. No Remove button is shown until that command exists.

These are the boundaries of #61; they do not mark the full parent #58 complete.
