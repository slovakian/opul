# ADR 0027: Seeding is a switch you can undo

Status: Accepted

A torrent whose selected files are all on disk is complete. Rust records that on the Download (`TorrentDownload.complete`) from engine reports only: seeding proves every selected file is there, and fetching outside a recheck proves something is missing, either because more files were selected or because a recheck found a gap. The frontend never works completion out from progress, which is wrong after a file-selection change and during a recheck.

A complete torrent is never paused, resumed or cancelled. It seeds or it doesn't, and Stop seeding and Start seeding switch between the two. Finished and "reached the ratio limit" both mean not seeding, and either can seed again. Removing it from the library is the only permanent way out. The snapshot names three states for it: `Seeding`, `NotSeeding`, and `CheckingFiles` while it reattaches to its files on the way to seeding again. `CheckingFiles` is also what a seeding torrent shows while Opul relaunches.

Start seeding moves the Download to Starting and ensures its transfer. The engine clears its stopped flag and unpauses the torrent where it left its files. Moving a finished torrent's files is not offered yet; when it is, Start seeding must add the torrent again at the new location so it rechecks the files there. Starting again after the ratio limit stopped it clears that limit, so the torrent seeds until stopped or given a new one. Starting again after stopping it by hand keeps the limit.

While a complete torrent checks its files, stale engine reports that it stopped, and the recheck itself, change nothing. A late "seeding" report after Stop seeding is ignored too. Checkpoints from before this decision load seeding and finished torrents as complete, and a seeding torrent someone paused as not seeding.

This refines [ADR 0024](0024-rust-application-state-machine-owns-app-state.md): the reducer owns the switch and its capabilities, and [ADR 0020](0020-the-torrent-engine-is-librqbit.md)'s engine only carries it out.
