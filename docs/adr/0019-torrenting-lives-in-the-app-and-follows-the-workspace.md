# Torrenting lives in the app and follows the Workspace

A torrent is an ordinary Download in one Workspace. Its files download to the Workspace's selected or Default saved folder. Opul handles `.torrent` files and `magnet:` links directly in the main app process through `opul_core`. While a magnet link fetches its metadata, the Download stays in Fetching with no Kind until its files are known. When all selected files finish, the Download transitions to Seeding until its ratio target is met or the person stops seeding.

Multi-file torrents allow selecting which files to download. Skipping files excludes their bytes from the Download's total size and transfer plan. Quitting Opul stops torrent transfers immediately, and relaunching resumes them.

## Considered options

A separate background daemon or helper process (such as FolxAgent or FolxScheduleHelper) that continues torrent transfers while Opul is closed. Rejected because ADR 0004 requires quitting to stop transfers and closing the window to quit Opul without background processes running.

In-app scrapers of public torrent search websites. Rejected because site scrapers break frequently, clutter the interface, and duplicate web browsers.

A permanent global Torrents workspace. Rejected because ADR 0001 and ADR 0002 establish that every Workspace is ordinary and holds any kind of Download.
