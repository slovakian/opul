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
