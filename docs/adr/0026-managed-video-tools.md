# ADR 0026: Opul verifies and selects its own video tools

Status: Accepted

The video-tools module owns optional video setup. Settings provides consent, cancellation, retry, offer suppression and an immediate update preference. The application shell resolves separate Stable and Nightly data directories and gives the module its Tools root. Direct files and torrents remain independent.

FFmpeg and ffprobe are built-in LGPL executables shipped in Resources, one architecture per installer, and update with the app. The optional video tools are the independently updated Python runtime and bare yt-dlp script. The signed fixed `python-runtime` pointer selects an immutable numbered runtime release with architecture, exact compressed size, checksum, signature URL and minimum Opul version. Neither optional component belongs in the installer.

A Python runtime and yt-dlp script form a selected pair. Opul verifies its signed runtime pointer before parsing, enforces the minimum application version, and verifies the runtime archive's signature before its checksum and extraction. It resolves one yt-dlp release tag and verifies that project's OpenPGP checksum signature with an embedded key. These are separate trust anchors and formats. A candidate must answer the expected version and initialize supported curl_cffi and sandboxed QuickJS without a Python deprecation warning before activation.

Installation stages in Tools, then selects both versions with an atomic state.json replacement. Pushed snapshots expose five exclusive states and one exclusive Ready update flag. MediaEngine obtains the interpreter/script command dynamically from Ready, with explicit built-in FFmpeg/ffprobe and QuickJS paths. No Homebrew or PATH discovery or tool-path preferences remain. Only the hidden OPUL_YT_DLP developer override bypasses selection.

Tools live in channel-specific Application Support, excluded from Time Machine. Startup removes abandoned staging and unreferenced versions. Updates wait for active media users; the previous script lasts until another script update, and the previous runtime/full pair lasts until the new runtime finishes a video Download. Failure before that success restores the retained pair. Removal cancels installation, stops video writers before pruning, and retains partial and finished output.

This preserves [ADR 0024](0024-rust-application-state-machine-owns-app-state.md): the application reducer owns waiting Downloads and their capabilities; the tools module owns tool selection. A waiting playlist stays one Download until probing can expand it under [ADR 0006](0006-a-playlist-becomes-many-downloads.md). [The Tauri shell decision](0004-tauri-interface.md) remains in force; the shell exposes commands/events and the frontend renders snapshots without owning installation or Download state.

This supersedes ADR 0021's remaining yt-dlp discovery paragraph and completes the discovery replacement begun in ADR 0025. The public contract, pairing rules, disk layout, tests and remaining sibling-ticket responsibilities are documented in [video tools](../video-tools.md).
