# Workspace folder appearances

Implements [issue #11](https://github.com/slovakian/opul/issues/11) on the Tauri
interface from PR #8, verified at integration commit `a13055a`.

Rust's bundled `src/application/folder-treatments.json` defines 12 treatments.
Each saved assignment includes the preset identity, version, and resolved hue,
saturation, and brightness. Rendering uses these saved values, so changing the
palette cannot change existing appearances.

Allocation favors less-used preset identities. UUID v5 hashes of the workspace
ID and preset identity break ties consistently across processes. Migration walks
workspace IDs in sorted order and joins the startup checkpoint. Legacy symbols
and missing or blank icons receive folder treatments because the Tauri interface
renders them as folders. Existing IDs, icons, folders, and downloads are preserved.
Snapshots and draft proposals never write state or rebalance assignments.

The Rust draft command returns workspace and folder IDs with a proposed appearance.
The dialog retains them through polling and failed saves. Identical submissions
also retain their request ID, so a retry after a lost response returns the saved
receipt. Selecting another emoji suspends the folder treatment; returning to 📁
restores it. A shared artwork renderer applies filters to the emoji span alone.

Verification on macOS 26.2:

- 95 Rust tests, the TypeScript/production build, and 14 WebKit tests passed.
- State-machine tests cover deterministic migration, balanced allocation, invalid
  parameters, old checkpoint checksums, one migration checkpoint, reads without
  writes, restart, creation, replacement Downloads, rename, reorder, and deletion.
- WebKit flows compare the creation preview, sidebar, heading, and download
  selector, including polling and failures before and after a committed save.
- Inspected all 12 folders at sidebar size in light and dark themes in WebKit
  and a signed debug Tauri bundle using an isolated library. Native checks also
  included 🐼, 👀, 🌈, 👩🏽‍💻, and 👨‍👩‍👧‍👦. A separate WebKit matrix exercised
  all filters on native colored, neutral, eye, skin-tone, and joined artwork.

Run `Scripts/test.sh`, `Scripts/package_app.sh debug`, and
`Scripts/smoke_test.sh`. Appearance tests write `test-results/folders-light.png`,
`folders-dark.png`, and `native-emoji.png`. If port 1420 belongs to another
worktree, run Playwright with a dedicated Vite port and matching base URL.
`OPUL_TEST_PORT=1531 Scripts/test.sh` starts an isolated server on that port.

## Existing workspace icons (#12)

Choose **Change icon…** in either workspace menu. The anchored picker holds a
local draft: search and selection do not write, selecting an emoji keeps it open,
and Cancel, Escape, or an outside click discards it and returns focus to the menu
trigger. Save commits the complete Unicode sequence and appearance together.
Checkpoint failure keeps the draft open. Retries with unchanged input reuse the
request ID, including when the first response was lost after committing.

`SetWorkspaceAppearance` changes only those two fields in the Rust state machine.
Names, folders, format, organization, view settings, and downloads remain current.
The automatic folder assignment is retained separately from the preference:
Automatic, explicit Original, resolved Preset, or resolved Custom parameters.
Automatic applies only to 📁; deliberate treatments follow eligible emoji. The
shared eligibility rule in `frontend/lib/workspace-appearance.ts` suspends filters
and hides color controls for curated exclusions, matching both presentation
selectors without changing stored Unicode. Returning to an eligible emoji restores
the preference. A previously unassigned workspace gets a frozen Rust proposal
when editing begins; it is persisted only when saving a folder, without replacing
an already assigned treatment.

The shared controlled picker composes Frimousse 0.4 with shadcn Base UI popovers,
input groups, buttons, and toggles. Browsing cells always show native colors.
Frimousse's custom rendering API also corrects its zero-based ARIA grid indices
to the required one-based indices. Its internal keyboard scrolling uses those
indices, so the cell renderer keeps the keyboard-active cell visible using its
actual bounds, including after scrolling in either direction. Hover does not
scroll. No dependency fork is needed.

The English dataset is bundled from locked `emojibase-data` 18.0.0. Regenerate it
with `node Scripts/bundle_emoji_data.mjs`; source licenses accompany the data in
`frontend/assets/emoji/`. The custom resolver makes no network or storage requests.
It probes native color glyphs to determine supported versions and country flags,
then filters the bundled dataset before giving it to Frimousse. This is required
because [Frimousse uses custom data without support filtering](https://frimousse.liveblocks.io/#custom-data-sources-and-locales).

Verification adds state-machine checks for atomicity, unrelated settings,
validation, retained automatic assignments, frozen proposals, full Unicode,
checkpoint failure, restart, and idempotent retry. WebKit flows cover both menus,
offline first use, keyboard selection, cancellation and focus, polling, failures,
Original and deliberate treatments, presentation variants, eligibility, and
accessible controls at the 720×520 minimum, including save errors.

Final automated verification: `OPUL_TEST_PORT=1531 Scripts/test.sh` passed 101 Rust
tests (98 core and 3 desktop), the TypeScript/production build, and all 28 WebKit
tests. The appearance-command regression first failed on the missing command;
the keyboard-scrolling regression first reproduced a clipped active cell, then
passed after the renderer correction. Standards and spec reviews have no
remaining findings.

Follow-up tickets retain their scope: creation composition (#13), treatment
suggestions (#14), custom sliders (#15), and skin-tone controls and recents (#16).
The preference model supports deliberate treatments now; their selection controls
arrive in those tickets.

## Color suggestions (#14)

Both Change icon and the workspace dialog use the controlled picker. Six unique
preset previews show the selected emoji through the shared native artwork renderer.
The saved built-in preset is included with its resolved values. Folder suggestions
use a deterministic shuffle seeded by the immutable workspace ID; other eligible
emoji use a fresh random seed once when the picker opens. Switching emoji, search,
selection, polling, and rerenders do not resample either set.

The ghost +6 button reveals the remaining six identities and disappears. The color
section reserves its height so expansion does not resize or scroll the emoji grid.
When its available space is smaller than its content, it scrolls with the official
[shadcn scroll-fade utility](https://ui.shadcn.com/docs/utils/scroll-fade). Keyboard
expansion transfers focus to the first revealed option without scrolling. A check
and pressed state identify deliberate selections; choosing the automatic look as a
preset still records a deliberate preference.

Original, Automatic for folders, Custom, and a saved custom treatment are independent
of the six suggestions. Custom starts a resolved preference from the current preview;
sliders and custom-mode navigation remain in #15. The saved custom choice remains
available after selecting Original or another preset. Excluded emoji hide the entire
color section and suspend the preference without deleting it.

The creation dialog now composes the shared picker so suggestions can be selected
before creating a workspace. Done, outside click, and inner Escape retain the local
appearance draft. Create persists emoji and appearance together, using the frozen
workspace ID and automatic assignment. Canceling the outer dialog discards its draft.

Tests in `tests/ui/color-suggestions.spec.ts` cover unique sampling, saved presets,
per-opening sampling, polling stability, expansion, selection/focus/scroll, deliberate
preferences, eligibility, saved custom restoration, creation/retry, and minimum-window
accessibility. Existing actor tests verify checkpoint durability and process restart;
WebKit reopening tests start a fresh page from the saved snapshot. Screenshots are
written under `test-results/color-suggestions-*.png`.

Final #14 verification on macOS: 101 Rust tests, TypeScript/production build, and
36 WebKit tests passed. Standards and Spec reviews have no remaining findings.
The six-suggestion regression failed before implementation and passes afterward.
`Scripts/package_app.sh debug` and `Scripts/smoke_test.sh` passed with strict bundle
signature and icon checks. Native app inspection used an isolated library and
confirmed all twelve folder and multicolored rainbow previews, preset selection,
creation, saved resolved values, and reopening after process restart.

For machines with limited resources, run the checks sequentially with
`CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace`,
`npm run build`, and `OPUL_TEST_PORT=1541 npm test -- --workers=1`.
