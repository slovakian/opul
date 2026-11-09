# ADR 0027: The desktop pushes the library to the interface as patches

Status: Accepted

The interface no longer polls. `subscribe_library` opens a Tauri channel per webview. The desktop sends the whole library once, then, whenever the application actor publishes a new projection, a patch holding only what changed since the previous message: changed scalar fields, every workspace outline (its settings and ordered download ids) when any workspace or the downloads it holds changed, and each download whose projection changed. A download that no outline lists has been removed. When nothing changes, nothing is sent. A subscription from the same webview replaces the previous one, so a reloaded page never receives two feeds.

The feed reuses the actor's projection publisher (ADR 0024): a single thread per subscriber waits on its sequence and diffs consecutive `MachineSnapshot`s. Adapter telemetry arrives as one batch per runner pass, so a pass publishes at most one telemetry projection. The `snapshot` command remains for smoke tests and other one-off reads.

The interface keeps the library in an external store outside React state. Applying a message keeps every object that did not change, and splits each workspace from the downloads it holds, so workspace settings keep their identity while transfers advance. Components subscribe with `useLibrarySelector` to exactly the slice they render: the shell reads workspaces and counts, the sidebar reads counts and its footer reads the combined rate, the list reads downloads, and memoized rows render again only when their own download changes. Commands, preferences and `busy` stay in the `useLibrary` context, which no longer changes with the library.

A command settles when the feed has delivered its receipt's revision, so controls never act on state from before the command. If the feed cannot open, the interface tries again after one second and keeps any library it already shows. A message that fails validation reconnects, because later patches build on it. A message older than the library, by boot and sequence, changes nothing.

## Considered options

Pushing the whole snapshot on every change was simpler, but its size and parsing cost grow with the library while a transfer advances several times a second. Tauri events were rejected in favour of channels, which are ordered, scoped to the subscribing webview, and intended for streaming. A general JSON Patch was rejected because entity-level replacement maps directly onto how the interface shares unchanged objects.
