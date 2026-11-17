# Rune migration behavior baseline

Recorded for [#100](https://github.com/slovakian/opul/issues/100), before production migration. The source baseline is Opul [`3229d7f4d081621626948384c2bdc4f4ef7c6a58`](https://github.com/slovakian/opul/tree/3229d7f4d081621626948384c2bdc4f4ef7c6a58). The integration branch initially pointed to exactly that commit, with no prerequisite implementation commits. Later integration work must preserve these product behaviors while replacing framework adapters.

## Evidence and scope

Read the [feature inventory](../FEATURES.md) and its six [YAML sources](../features/), [glossary](../../GLOSSARY.md), current frontend, desktop commands, behavioral tests, assets, and product ADRs together. The inventory contains 70 entries: 32 shipped, 14 partial, 2 planned, 18 ideas, and 4 rejected. A partial entry does not require exposing every API the core already implements.

The retained [light inspector screenshot](evidence/baseline-inspector.png) and [dark screenshot](evidence/baseline-dark.png) come from `tests/ui/app.spec.ts` using the baseline's WebKit fixture, at 1220 × 800 with reduced motion. They show the current interface, not a native Rune port or a real download. Validation commands, actual counts, and platform limitations are recorded in [preview verification](preview.md).

Reinspect the exact source independently of subsequent integration changes:

```sh
git show 3229d7f:docs/FEATURES.md
git show 3229d7f:GLOSSARY.md
git ls-tree -r 3229d7f -- docs/features docs/adr frontend assets tests/ui
```

## Product contracts to preserve

| Area | Current exposed behavior | Implementation and behavior evidence |
| --- | --- | --- |
| Ownership | Every Download belongs to one ordinary Workspace and one of its Saved Folders. Rust owns identity, capabilities, lifecycle, recovery, receipts, and persistence. Deleting the last Workspace creates an empty Downloads Workspace. | ADRs 0001, 0002, 0005, 0024; `src/application/`; `workspace-removal.spec.ts` |
| Sidebar | All, In progress, Completed aggregate views; one-folder Workspace is simple, multiple folders form a branch. Disclosure, Workspace navigation, folder navigation, and context actions are separate. Selection, breadcrumbs and contextual destination agree. | `workspace-sidebar.tsx`; `workspace-sidebar.spec.ts`, `workspace-branches.spec.ts`, `unified-header.spec.ts`, `destination.spec.ts` |
| Expansion | Live updates, rename, reorder, responsive remounts, and navigation retain independent branch choices. Remember previous state and Start collapsed are startup preferences. Failed reads/writes retain local choices and offer Retry. | `workspace-expansion.spec.ts`, `workspace-expansion-cleanup.spec.ts`, `workspace-preference-lifecycle.spec.ts`; desktop preference tests; [prior verification](../workspace-sidebar-verification.md) |
| Saved Folders | Native picker adds a catalog entry; adding a second folder expands and navigates correctly. Removal normalizes multiple-to-one and missing selection. Temporary destination does not change the breadcrumb location or default. | `saved-folder-additions.spec.ts`, `saved-folder-confirmation.spec.ts`, `workspace-removal.spec.ts`, `destination.spec.ts` |
| Workspace editing | Create, rename, delete, default folder and default format; commands commit atomically. Picker cancellation leaves state unchanged. Failed or lost-response submissions retain drafts and reuse receipts. | `workspace-dialog.tsx`, `workspace-header.tsx`; `workspace-creation.spec.ts`, `submission.spec.ts`; actor tests |
| Appearance | Full Unicode emoji, supported skin tones, offline search, keyboard grid, latest 24 successfully saved recents, twelve folder treatments, six initial color suggestions and +6, Original/Automatic/Custom, exclusions. Preview drafts stay local; Save/Create commits atomically. | [appearance contract](../workspace-appearances.md); `appearance.spec.ts`, `emoji-picker.spec.ts`, `icon-picker.spec.ts`, `color-suggestions.spec.ts`, `custom-treatment.spec.ts`, `skin-tones-recents.spec.ts`; actor persistence tests |
| Library | Search and status filters; list/table; sort by order/name/state/source; group by nothing/state/folder/kind/source. Per-view display preferences; card/minimal list preference; narrow-window navigation. | `downloads.tsx`, `download-toolbar.tsx`; `app.spec.ts`, `display-preferences.spec.ts`, `list-style.spec.ts`, `responsive-header.spec.ts` |
| Submission | Direct HTTP files, pasted btih magnets, native .torrent picker, remote .torrent URLs, supported media pages. Ambiguous links ask File/Video or playlist/Torrent. Aggregate views use chosen/default Workspace and destination choices. | `download-input.tsx`, `download-options.tsx`; `download-options.spec.ts`, `submission.spec.ts`, `destination.spec.ts`; Rust media/torrent tests |
| Media | Best available, Best MP4, 1080p, 720p, Original file and Workspace default; MP3/M4A extraction. Playlists expand into ordered per-Workspace deduplicated Downloads. Concurrency choices 1/2/3/4/5/8, default 3; media download cap. | `model.ts`, desktop command adapters, ADRs 0006, 0021; `src/media/tests.rs`, `src/application/waiting_tests.rs` |
| Transfer controls | Pause/resume/cancel/retry/removal according to Rust capabilities; pending phases remain truthful. Complete torrents use Start seeding/Stop seeding, including Checking files and restart after ratio limit. Removal keeps output files. | `download-actions.tsx`, `download-menu.tsx`, `row-view.ts`; ADR 0027 seeding; `torrent-row-states.spec.ts`, `context-menu.spec.ts`; core tests |
| Torrent settings | Session bandwidth limits, DHT, listen-port random/fixed fallback, connection limit default 200. Inspector shows peers/seeds/upload/rate/ratio, seed ratio limit, files and tracker links. | `settings-dialog.tsx`, `download-inspector.tsx`; `trackers-trace.spec.ts`, `torrent-row-states.spec.ts`; ADR 0020 |
| Details | Selecting a Download opens the persistent right inspector with current information, source copy, ratio editing and Show in Finder when supported. | `download-inspector.tsx`, `inspector-panel.tsx`; `app.spec.ts`, `trackers-trace.spec.ts`; screenshot above |
| Desktop | Light/dark/system appearance; Cmd-N begins submission, Cmd-comma opens Settings, Cmd-F focuses search; native menus, dialogs and Finder reveal. Explicit loading, empty, startup failure and recovery states. Acknowledge recovery fences transfer execution. | `use-app-keyboard.ts`, `src-tauri/src/desktop.rs`; `app.spec.ts`, `built-in-tools.spec.ts`; ADR 0024 |
| Video tools | Opening Setup never grants consent. Five exclusive states: Not set up, Turned off, Setting up, Ready, Not working. Verified Python/yt-dlp pair; progress/cancel/retry/reminders, updates, diagnostics, removal, rollback and Stable/Nightly separation. Waiting media retains identity/destination and starts when ready; files/torrents stay usable. | ADR 0026; [video tools](../video-tools.md); `video-setup.spec.ts`, `video-tools.spec.ts`, `video-updates.spec.ts`, `waiting-video.spec.ts`; core tools tests |
| Built-in tools | Architecture-matched FFmpeg/ffprobe in bundle Resources; no user path setting. Missing tools block post-processing, not direct files/torrents. | ADR 0025; `built-in-tools.spec.ts`; `Scripts/test_ffmpeg.sh`; [packaged journey](../packaged-video-journey.md) |
| App updates | Signed Stable/Nightly feeds; automatic checks on, automatic install off by default; manual checks/download; user-controlled restart; development builds cannot install. | [updates](../app-updates.md); `app-updates.spec.ts`; `src-tauri/src/app_updates/` |
| Quit/reopen | Closing the last window quits. Quiesce transfers and owned children, commit Stopped before termination; reopening retains running intent and deliberate holds. Recovery remains explicit. | ADR 0004 quitting and ADR 0024; application actor/transfer tests; native packaged journeys |

Test filenames in the table are under `tests/ui/`; frontend filenames are under `frontend/components/`, `frontend/lib/`, or `frontend/hooks/`. Browser fixtures prove interface behavior, not native OS services, real output files, signed update delivery, or process cleanup. Preserve the existing Rust and packaged-app seams for those checks.

## Partial features: preserve only the exposed part

| Inventory ID | Exposed at baseline | Dormant or missing behavior outside migration scope |
| --- | --- | --- |
| `file-priorities` | Stored priorities exist; no UI control. | High/normal/low scheduling; only Skip affects the include mask. |
| `file-selection` | Inspector lists file names and sizes. | No selection control or preflight file chooser; core `SetTorrentFiles` exists. |
| `magnet-links` | Paste btih magnet in the download field. | OS registration and incoming-link routing. |
| `move-finished-files` | No desktop move action. | Core can record a shell-performed move; not a functioning desktop move workflow. |
| `torrent-trackers` | Read and open tracker URLs. | Add/edit/remove tracker controls. |
| `trash-on-remove` | Remove from library keeps files and says so. | Core's verified-output trash capability is not exposed. |
| `cookies-browser-auth` | Normal page submission; no authentication settings. | Core cookie-file/scoped-cookie support and guided sign-in are not exposed. |
| `filename-templates` | Title plus extension default. | Custom template field; core supports bounded templates. |
| `format-quality-selection` | Existing presets and Workspace default. | Site format browser and custom yt-dlp strings. |
| `subtitles` | Desktop submits subtitles off. | Core language/automatic captions/SRT/embedding controls. |
| `thumbnails` | Desktop submits thumbnails off. | Core sidecar thumbnail writing/conversion controls. |
| `duplicate-downloads` | Reject same active torrent info hash; deduplicate playlist URLs within Workspace. | Dedicated warning for repeated ordinary page submissions. |
| `file-organization` | Automatic default; existing Workspace engine setting remains meaningful. | Workspace dialog has no organization control. |
| `guided-browser` | No desktop guided-browser window or action. | Core observations/candidates/scoped cookies and ADR 0022's proposed WebKit flow. |

The two planned OS-open features, 18 ideas, and four rejected features remain outside parity scope. In particular, do not infer shipped PEX, protocol encryption, sequential download or UPnP from engine defaults. There is no torrent queue limit, scheduler, RSS, watched folders, web interface, search scraper, tagging, or background transfer helper to port. Refer to the immutable baseline inventory for each entry's rationale.

## Motion and assets

`frontend/lib/row-view.ts`, `download-row-status.tsx`, `download-row-description.tsx`, `download-row-parts.tsx`, `ui/progress.tsx`, `ui/circular-progress.tsx`, and `frontend/styles.css` jointly define effects. Preserve state mapping as well as appearance:

- Horizontal indeterminate sheen moves left to right in 2.6s. Determinate sheen moves right to left toward the fill in 6s and is clipped to the empty track. The fill never shimmers.
- Resuming preserves the value and pulses in 2.4s; looking for peers uses a slower 3.2s breathing treatment. Circular indeterminate sheen crosses the ring stroke in 2.6s.
- Upload/seeding text shimmer moves bottom to top in 4s. Getting file info, Checking files and Processing have their current quiet text treatment.
- Row layers crossfade in place: 200ms enter, 150ms exit, with blur/scale for normal motion. Only a witnessed transition from a working state into Finished/Seeding/NotSeeding celebrates completion: 300ms fill settle, then a check held for 2700ms. Initial completed items and reattached Checking files do not replay it.
- Reduced motion removes sheen, pulses, blur and spatial scaling, retaining 150ms opacity state changes. A still screenshot cannot establish shimmer direction or timing. `download-row-stability.spec.ts` and `torrent-row-states.spec.ts` retain existing behavioral assertions.

Retain `frontend/assets/opul-logo.png`, `opul-icon.png`, `download-folder.png`, `assets/AppIcon*.png` and `assets/FallbackArtwork/Masters/`. Emoji data is the locked emojibase 18 dataset in `frontend/assets/emoji/data.json` with adjacent emojibase/frimousse licenses; preserve complete Unicode sequences and native artwork behavior. Folder treatment identities and resolved values live in `src/application/folder-treatments.json`; do not replace them with ad hoc theme colors. The frontend uses variable Inter. Rune's embedded Inter is a preview convenience, not approval of final type metrics.

## Intentional migration changes and historical ADRs

Issue #99 authorizes the visual refresh and replacing the persistent inspector with a modal right sheet that overlays the unchanged list width, blurs and blocks its background, consumes outside dismissal, contains focus, handles nested popups, closes on Escape and restores focus. This is a future change, not baseline behavior and not implemented by this minimal preview.

Product ownership and lifetime rules in ADRs 0001/0002/0004 quitting/0005/0006/0019/0020/0024/0025/0026/0027 seeding remain relevant. Historical SwiftUI/WebKit/Tauri wording does not imply those frameworks must survive. ADR 0003's SF Symbols is superseded by the implemented emoji contract; ADR 0004 Tauri and ADR 0027 patch transport describe replaceable adapters. No ADR is globally rewritten by #100. Production Rune persistence and shutdown still need their own implementation and verification; support for importing old installations is optional under #99.
