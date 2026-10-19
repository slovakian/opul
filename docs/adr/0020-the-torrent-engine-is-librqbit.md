# The torrent engine is librqbit

Status: accepted; persistence ownership clarified by ADR 0024

Opul runs one BitTorrent session inside `opul_core`, in the main app process. The session is [librqbit](https://github.com/ikatson/rqbit) 9 (Apache-2.0). A `.torrent` file or a `magnet:` link becomes an ordinary Download in the Workspace that was selected, and its files go to that Workspace's selected or Default saved folder.

Folx 5.34 does the same job with libtorrent 2.0.11 linked into the Folx process (`libtorrent/2.0.11.0`, evidence `ev_714abb7b6bd5c9193190a26e9a8ec089be97a12fa54addc9d4877c849a3159f1` in the existing Folx overview). Opul does not link Folx's binary and does not copy its code. librqbit is the engine that covers the same transfer: metadata fetch, trackers, DHT, peer exchange, file selection, pause, resume, and fast resume across launches.

## What the Folx binary showed

Static Ghidra on the arm64 slice `Folx.arm64` (SHA-256 `8c462249bc44815f1f9cc24debdce40d2d246b8ae1a79a8a2eafccddd7e1d43b`). Nothing was executed.

- `AppDelegate::magnetLinkMetadataFetchedHandler:` (`0x100023b3c`, `ev_6b807ccc5d762edf762463894d5d4dcd4903189337d9802bcd0b89d264b13311`) turns a fetched `.torrent` back into the task that was waiting on that info-hash. It keeps the task's save folder, priority, and comment. If parsing fails it sets an error state (`setStateValue_` 8, then 11).
- `AppDelegate::torrentCompletedHandler:` (`0x100024308`, `ev_7c1ec2f53898280442fdd19279229bff4a0c26fb2b8018cae0cac930775c1cee`) marks the task complete (`setStateValue_` 9) and, unless it was already stopped, `setIsSeeding_` 1. That is Opul's Seeding state. Finished is the later stop, when the ratio is met or the person stops seeding.
- `FolxTorrentInfo::setFilePriorityWithData:withFiles:` (`0x1000f927c`, `ev_c7afcd23c623f14634153709cc4437773fc15f5906c594199691c339240e458e`) walks the file tree and stores one signed byte per file index under `state`. Libtorrent uses 0 for "do not download". Opul's Skip is that bit.
- Preferences strings in the binary, not executed: `EnableDHT`, `TorrentTCPPort`, `TorrentUseRandomPortAtStartup`, `TorrentMaxConnections`, `FolxIsTorrentSeedRatioLimit`, `FolxTorrentSeedRatioLimit`. The preferences window exposes those knobs. The port, DHT, and connection cap apply the next time Opul opens, because librqbit binds them when the session starts. Speed limits change on the live session.

## Decisions

High and low file priority are stored on the Download. librqbit's file API is an include mask (`update_only_files`), not libtorrent's 1/4/7 priority scale, so only Skip changes what is transferred. Skipped files stay out of the selected size, as ADR 0019 requires.

The connected-peer count is the live peer count. The seed count is librqbit's `not_needed` peer count (peers that have nothing further we need). Folx shows seeds and leeches from libtorrent's peer list. This is the closest split the engine exposes.

Opul does not scrape torrent search sites, does not keep a helper running after quit, and does not create a Torrents workspace. Quitting the process stops transfers. librqbit's JSON session and `overwrite` fast-resume bring the same torrents back on the next launch.

UPnP port mapping stays off. A router that does not answer must not stop the session from opening.

## Considered options

Linking libtorrent the way Folx does. Rejected because it pulls a C++ library into the Rust core and because the behavior we need is the transfer, not that binary.

Keeping the simulated `simulate_tick` session. Rejected because a magnet would never resolve and a launch would not resume.
