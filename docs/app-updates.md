# Opul updates

Opul updates itself with the Tauri updater, driven from Rust (`src-tauri/src/app_updates.rs`). The webview has no updater permission: it renders `app-updates` snapshots and asks for actions through the `app_updates_*` commands.

## Checking

Each channel reads its own manifest. Stable reads `latest.json` from the latest GitHub release, and Nightly reads `nightly.json` from the rolling `nightly` release. The first scheduled check runs about a minute after launch, then every 24 hours, matching the video-tools cadence. Scheduled checks fail silently. Settings ▸ Updates and the app menu's Check for Updates… run a manual check, which reports its error. Checks are single-flight: a request made while one is running joins it, so no cooldown is needed. A missing manifest (the updater's `ReleaseNotFound`, such as a 404 before a channel's first release) means Opul is up to date.

## Choices

`app-updates.json` in the channel's data folder stores two choices and two facts:

- **Check for updates automatically** is on by default.
- **Install updates automatically** is off by default and takes effect only while checks are on. Setup's _Update Opul automatically_ sets it.
- The time of the last successful check.
- The newest version a notification was claimed for.

With automatic installs off, a found version claims its notification atomically before the toast appears, so each version offers itself once across launches; a newer version offers itself again. Not now only closes the toast. Settings keeps the dot and the Download action.

## Installing

Download verifies the minisign signature against the public key in `tauri.conf.json`, then replaces the `.app` bundle in place. The running process continues, and the new version runs at the next launch. Opul never restarts on its own: Restart (confirmed while downloads are active) checkpoints the download engine, then relaunches. Later leaves the update for the next launch. Automatic installs stay silent until the update is ready.

Development builds (no bundle), translocated copies and apps on a read-only disk image can check but not install, and Settings says why. A non-writable `/Applications` is left to the updater's administrator prompt. Release notes are a _What's new_ link to the GitHub release; there is no channel switching.

## Testing

`app_updates::tests` drives the scheduler, preference gating, notification claims, single-flight checks, failures and install blockers with a fake clock and source. `tests/ui/app-updates.spec.ts` covers Settings, the toasts, Restart confirmation, the menu item and Setup against a mock that mirrors those rules. A real end-to-end update needs two signed releases on the same channel.
