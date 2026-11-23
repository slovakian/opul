<!-- This file is generated from docs/features by Scripts/check-features.mjs. Do not edit. -->

# Opul features

This list is generated from [`docs/features/`](features/). Edit the YAML, then run `node Scripts/check-features.mjs --write`.

How a feature moves from an idea to shipped is in [feature workflow](feature-workflow.md).

## Status legend

| Status        | Meaning                                        |
| ------------- | ---------------------------------------------- |
| `idea`        | Not built. Details say what was checked.       |
| `planned`     | Accepted to build, and not started.            |
| `in-progress` | Being built now.                               |
| `partial`     | Some of it works. Details say what is missing. |
| `shipped`     | Available in the app today.                    |
| `wont-do`     | Will not be built. A reason is required.       |

Priority is `p0` through `p3`. A dash means no priority is set.

## Counts

| Status        |  Count |
| ------------- | -----: |
| `shipped`     |     32 |
| `partial`     |     14 |
| `in-progress` |      0 |
| `planned`     |      7 |
| `idea`        |     18 |
| `wont-do`     |      4 |
| **Total**     | **75** |

## Torrent

| ID | Title | Status | Priority | Source | Summary |
| --- | --- | --- | --- | --- | --- |
| `connection-limit` | Connection limit | `shipped` | — | `qbittorrent` | Cap how many torrent connections Opul opens at once. |
| `dht` | DHT | `shipped` | — | `qbittorrent` | Find peers on the DHT, and turn that discovery off. |
| `listen-port` | Listen port | `shipped` | — | `qbittorrent` | Let Opul pick a listen port, or set one yourself. |
| `pause-and-resume` | Pause and resume | `shipped` | — | `qbittorrent` | Pause a torrent and continue it later, including after you quit Opul. |
| `peer-counts` | Peers and seeds | `shipped` | — | `qbittorrent` | See a torrent's peers, seeds, upload speed, and how much it has shared. |
| `remote-torrent-urls` | Remote torrent URLs | `shipped` | — | `qbittorrent` | A link whose path ends in .torrent is fetched and added as a torrent. |
| `seeding-ratio-limits` | Seeding ratio limits | `shipped` | — | `qbittorrent` | Stop seeding a torrent when its share ratio reaches a number you set. |
| `seeding-switch` | Seeding switch | `shipped` | — | `qbittorrent` | Keep sharing a finished torrent, stop sharing, and start again later. |
| `torrent-bandwidth-limits` | Torrent bandwidth limits | `shipped` | — | `qbittorrent` | Limit how fast all torrents download and upload. Zero means unlimited. |
| `torrent-files` | Torrent files | `shipped` | — | `qbittorrent` | Choose a .torrent file and Opul adds that torrent to the selected workspace. |
| `file-priorities` | File priorities | `partial` | — | `qbittorrent` | Mark files in a torrent as high, normal, low, or skipped. |
| `file-selection` | File selection | `partial` | — | `qbittorrent` | Choose which files in a multi-file torrent to download. |
| `magnet-links` | Magnet links | `partial` | — | `qbittorrent` | Paste a magnet link and Opul downloads it into the selected workspace. |
| `move-finished-files` | Move finished files | `partial` | — | `qbittorrent` | Move a finished download into another saved folder. |
| `torrent-trackers` | Tracker list | `partial` | — | `qbittorrent` | See the trackers a torrent announced, and open one. |
| `trash-on-remove` | Trash files on remove | `partial` | — | `qbittorrent` | Delete a download's files when you remove it from the library. |
| `torrent-file-association` | Open .torrent files from Finder | `planned` | p0 | `qbittorrent` | Double-click a .torrent file or drop it on the Dock icon to add it to Opul. |
| `magnet-handler` | Open magnet links from the browser | `planned` | p0 | `qbittorrent` | Click a magnet link anywhere and Opul opens it, shows the files, and lets you pick a workspace. |
| `vpn-interface-binding` | VPN interface binding | `planned` | p1 | `qbittorrent` | Bind torrent traffic to a chosen network interface, such as a VPN, so transfers stop if it drops. |
| `active-torrent-limit` | Active torrent limit | `idea` | — | `qbittorrent` | Queue torrents so only a chosen number download or seed at once. |
| `bandwidth-scheduler` | Bandwidth scheduler | `idea` | — | `qbittorrent` | Use a different speed limit on a schedule, such as at night. |
| `force-recheck` | Force recheck | `idea` | — | `qbittorrent` | Recheck a torrent's files against the pieces on disk whenever you ask. |
| `ip-filtering` | IP filtering | `idea` | — | `qbittorrent` | Block or allow peers by IP address or range. |
| `local-peer-discovery` | Local peer discovery | `idea` | — | `qbittorrent` | Find peers on the local network without a tracker. |
| `peer-exchange` | Peer exchange | `idea` | — | `qbittorrent` | Learn more peers from the peers you are already connected to. |
| `per-torrent-speed-limits` | Per-torrent speed limits | `idea` | — | `qbittorrent` | Set a download or upload cap on a single torrent. |
| `protocol-encryption` | Protocol encryption | `idea` | — | `qbittorrent` | Encrypt BitTorrent peer connections, or require encryption. |
| `rss-feeds` | RSS feeds | `idea` | — | `qbittorrent` | Subscribe to an RSS feed and add matching torrents automatically. |
| `sequential-download` | Sequential download | `idea` | — | `qbittorrent` | Download pieces in order so a video can play before the torrent finishes. |
| `torrent-creation` | Torrent creation | `idea` | — | `qbittorrent` | Build a .torrent file from files already on disk. |
| `upnp-port-forwarding` | UPnP port forwarding | `idea` | — | `qbittorrent` | Ask the router to forward Opul's listen port. |
| `watched-folders` | Watched folders | `idea` | — | `qbittorrent` | Add every .torrent file that appears in a chosen folder. |
| `web-ui` | Web interface | `idea` | — | `qbittorrent` | Control downloads from a browser on another device. |
| `search-plugins` | Search plugins | `wont-do` | — | `qbittorrent` | Search torrent sites from inside Opul. |
| `torrent-categories` | Torrent categories | `wont-do` | — | `qbittorrent` | Group torrents with a separate category, the way qBittorrent does. |
| `torrent-tags` | Torrent tags | `wont-do` | — | `qbittorrent` | Label one torrent with several tags. |
| `background-transfers` | Transfers after quit | `wont-do` | — | `qbittorrent` | Keep downloading and seeding after Opul quits. |

### Details

- **Connection limit** (`connection-limit`) — Settings, default 200, from 1 to 10,000. Applied when the session starts.
- **DHT** (`dht`) — Settings, on by default. The change applies the next time the torrent session starts.
- **Listen port** (`listen-port`) — Randomize is the default. A fixed port of 0 is rejected. If the chosen port fails, the session retries on an ephemeral port. TCP and uTP are both on, with no separate switch.
- **Pause and resume** (`pause-and-resume`) — Fast resume is on. Quitting stops transfers; the next launch continues them. A complete torrent uses the seeding switch instead of pause.
- **Peers and seeds** (`peer-counts`) — The inspector shows those figures. The seed count is librqbit's count of peers that have nothing further Opul needs.
- **Remote torrent URLs** (`remote-torrent-urls`) — Other http(s) links are files or pages unless Torrent is chosen. The engine stores those as remote metainfo.
- **Seeding ratio limits** (`seeding-ratio-limits`) — The inspector field can be left empty to keep seeding. Starting again after the limit was reached clears it. New torrents have no app-wide default.
- **Seeding switch** (`seeding-switch`) — Finished torrents show Seeding or Not seeding. Checking files appears while Opul reattaches the files. Removing the download is the permanent way out.
- **Torrent bandwidth limits** (`torrent-bandwidth-limits`) — Settings applies KB/s caps to the live session. There is no per-torrent cap and no schedule.
- **Torrent files** (`torrent-files`) — The picker accepts one .torrent file up to 16 MB and stages its metainfo in the app.
- **File priorities** (`file-priorities`) — Skip, Low, Normal, and High are stored. ADR 0020 says only Skip changes the transfer, because librqbit uses an include mask. No priority control is shown.
- **File selection** (`file-selection`) — SetTorrentFiles updates librqbit's include mask, and skipped files stay out of the selected size. The inspector lists names and sizes and does not send a selection.
- **Magnet links** (`magnet-links`) — Pasting a btih magnet into the download field works. The engine fetches metadata while the row is still resolving. Opul is not registered as a magnet: handler (see magnet-handler).
- **Move finished files** (`move-finished-files`) — The state machine can record a move the shell already did. The desktop commands do not offer it. ADR 0027 says moving a finished torrent is not offered yet.
- **Tracker list** (`torrent-trackers`) — The inspector lists tracker URLs. Adding, removing, or editing trackers is not offered.
- **Trash files on remove** (`trash-on-remove`) — The core can trash output it verified. The desktop command always keeps files, and the confirmation says they stay on disk.
- **Open .torrent files from Finder** (`torrent-file-association`) — Needs a fileAssociations entry for .torrent (application/x-bittorrent) and handling of opened file paths at launch and while running. Dropping a .torrent onto the window does not add it. Nothing listens for a file drop, and the only intake is the Open .torrent picker.
- **Open magnet links from the browser** (`magnet-handler`) — Needs tauri-plugin-deep-link with the magnet scheme in the bundle config, single-instance forwarding to the running app, and a first-run or Settings prompt to make Opul the default magnet app. Transmission, qBittorrent, and Folx all support this.
- **VPN interface binding** (`vpn-interface-binding`) — The session leaves librqbit's bind_device_name unset, and Settings has no interface choice. The listen address is unspecified, so traffic follows the default route and does not stop when a VPN drops. qBittorrent, BiglyBT, and Murmur offer this. Transmission does not. Mac torrent users ask for VPN binding in r/torrents, r/Torrenting, and r/ProtonVPN.
- **Active torrent limit** (`active-torrent-limit`) — The concurrent-download setting applies to video pages and direct files, not to a torrent queue.
- **Bandwidth scheduler** (`bandwidth-scheduler`) — Update checks have their own timers. Nothing changes transfer rates by time of day.
- **Force recheck** (`force-recheck`) — Starting seeding again can show Checking files while the engine reattaches. There is no separate Recheck command.
- **IP filtering** (`ip-filtering`) — Session options and Settings have no IP filter list.
- **Local peer discovery** (`local-peer-discovery`) — Session options have no local-service-discovery setting.
- **Peer exchange** (`peer-exchange`) — ADR 0020 says librqbit covers peer exchange. Opul never sets a PEX option, and the engine default was not verified, so this is not marked shipped.
- **Per-torrent speed limits** (`per-torrent-speed-limits`) — Speed limits exist only on the whole session.
- **Protocol encryption** (`protocol-encryption`) — Opul sets no encryption option and does not show one. librqbit's default was not verified, so this is not marked shipped.
- **RSS feeds** (`rss-feeds`) — No RSS reader or auto-downloader is in the app.
- **Sequential download** (`sequential-download`) — No sequential flag in the session options or the interface. librqbit's default piece order was not verified.
- **Torrent creation** (`torrent-creation`) — Opul parses and downloads torrents. It does not create them.
- **UPnP port forwarding** (`upnp-port-forwarding`) — Port mapping is hardcoded off so a silent router cannot block startup (ADR 0020). Choosing a listen port is separate and shipped.
- **Watched folders** (`watched-folders`) — Nothing watches a folder for new torrent files.
- **Web interface** (`web-ui`) — Opul is a desktop app. The librqbit dependency includes an HTTP API client, and this repo does not start an API server.
- **Search plugins** (`search-plugins`) — ADR 0020 also records that Opul does not scrape torrent search sites. Reason: ADR 0019 rejects in-app scrapers because they break often, clutter the interface, and duplicate a web browser.
- **Torrent categories** (`torrent-categories`) — Workspaces are the shipped way to organize downloads. Reason: ADR 0001 puts each download in one workspace, which owns its folder and default format. A second grouping would give those settings two owners.
- **Torrent tags** (`torrent-tags`) — A download cannot be listed in more than one workspace. Reason: ADR 0001 rejects tags because the folder and default format would then have two owners.
- **Transfers after quit** (`background-transfers`) — The next launch resumes the downloads that were running. Reason: ADR 0004 says quitting stops transfers and closing the window quits Opul. A background helper was rejected for this version.

## Media download

| ID | Title | Status | Priority | Source | Summary |
| --- | --- | --- | --- | --- | --- |
| `audio-extraction` | Audio extraction | `shipped` | — | `yt-dlp` | Save a video page as an MP3 or M4A audio file. |
| `concurrent-media-downloads` | Concurrent media downloads | `shipped` | — | `opul-original` | Download several videos or files at once, up to a limit you choose. |
| `media-rate-limit` | Media rate limit | `shipped` | — | `yt-dlp` | Cap how fast page downloads transfer. Zero means unlimited. |
| `media-controls` | Pause, resume, cancel, and retry | `shipped` | — | `opul-original` | Pause a page download, continue it, cancel it, or try a failed one again. |
| `playlists` | Playlists | `shipped` | — | `yt-dlp` | A playlist link becomes one download per entry, in order, in that workspace. |
| `cookies-browser-auth` | Cookies and browser sign-in | `partial` | — | `yt-dlp` | Download a page that only works while you are signed in. |
| `filename-templates` | Filename templates | `partial` | — | `yt-dlp` | Name a media file from the page title. |
| `format-quality-selection` | Format and quality selection | `partial` | — | `yt-dlp` | Pick a video quality before a page download starts. |
| `subtitles` | Subtitles | `partial` | — | `yt-dlp` | Download subtitles, convert them to SRT, and embed them in the file. |
| `thumbnails` | Thumbnails | `partial` | — | `yt-dlp` | Save a thumbnail next to the download, as a JPEG when FFmpeg is available. |
| `chapters` | Chapters | `idea` | — | `yt-dlp` | Keep chapter markers, or split a video into chapter files. |
| `live-streams` | Live streams | `idea` | — | `yt-dlp` | Record a live stream from the start or from when you press download. |
| `metadata-embedding` | Metadata embedding | `idea` | — | `yt-dlp` | Write the title and other tags into the media file itself. |
| `sponsorblock` | SponsorBlock | `idea` | — | `yt-dlp` | Skip or mark sponsor segments in a video. |

### Details

- **Audio extraction** (`audio-extraction`) — The engine passes extract-audio and the matching audio format. Other codecs are not offered. This needs the built-in FFmpeg tools.
- **Concurrent media downloads** (`concurrent-media-downloads`) — Settings offers 1, 2, 3, 4, 5, or 8 at a time. The default is 3. This does not queue torrents.
- **Media rate limit** (`media-rate-limit`) — The Settings download cap is passed to yt-dlp as a rate limit. The upload cap applies to torrents, not to page downloads.
- **Pause, resume, cancel, and retry** (`media-controls`) — Opul's queue supervises the yt-dlp process. Pausing before a page has been probed does not need a running process.
- **Playlists** (`playlists`) — An entry already in the workspace is skipped. Each download then uses no-playlist. Until video tools can probe, the playlist stays one waiting row.
- **Cookies and browser sign-in** (`cookies-browser-auth`) — The engine can pass a cookie file to yt-dlp and scoped cookies on a direct request. The Tauri app has no cookies-from-browser setting and no sign-in browser. Guided sign-in exists only in the core.
- **Filename templates** (`filename-templates`) — The default template is the title plus the extension. The engine can take another template and refuses one that escapes the destination. The desktop always sends the default, with no field to change it.
- **Format and quality selection** (`format-quality-selection`) — Presets are Best available, Best MP4, 1080p, 720p, and Original file, plus a workspace default. There is no site format list and no custom yt-dlp format string.
- **Subtitles** (`subtitles`) — The engine can request subtitles, automatic captions, a language list, SRT conversion, and embedding when FFmpeg is present. The desktop always turns subtitles off, and there is no control.
- **Thumbnails** (`thumbnails`) — The engine can write and convert a thumbnail. The desktop always leaves this off, and there is no control. This is a sidecar file, not a picture embedded in the video.
- **Chapters** (`chapters`) — No chapter arguments are passed to yt-dlp.
- **Live streams** (`live-streams`) — There is no live recording mode. A link yt-dlp can already save as a normal file still downloads as a normal file.
- **Metadata embedding** (`metadata-embedding`) — The stored metadata flag writes an info JSON sidecar, not embedded tags or an embedded thumbnail. The desktop leaves that flag off, with no control.
- **SponsorBlock** (`sponsorblock`) — No SponsorBlock arguments or settings.

## Direct download

| ID | Title | Status | Priority | Source | Summary |
| --- | --- | --- | --- | --- | --- |
| `direct-http-downloads` | Direct HTTP downloads | `shipped` | — | `opul-original` | Paste a direct file link and Opul saves that file without video tools. |
| `link-kind-detection` | Link kind detection | `shipped` | — | `opul-original` | Opul treats magnets and .torrent links as torrents, common file suffixes as files, and asks when a link is ambiguous. |
| `browser-capture` | Browser capture | `planned` | p1 | `competitor:folx` | Send links and downloads from Safari, Chrome, or Firefox to Opul with a browser extension. |
| `segmented-downloads` | Segmented downloads | `planned` | p1 | `competitor:motrix` | Speed up large direct downloads by fetching a file over several connections at once. |

### Details

- **Direct HTTP downloads** (`direct-http-downloads`) — Compatible video, audio, image, and subtitle responses use a Rust HTTP path with resumable partials, bounded redirects, a media-type check, and a collision-safe name. Format selection and post-processing stay on yt-dlp.
- **Link kind detection** (`link-kind-detection`) — The prompt offers File, Video or playlist, or Torrent. Video is unavailable until video tools are turned on.
- **Browser capture** (`browser-capture`) — No browser extension or messaging host sends links into Opul. Magnet links and .torrent files are separate entries (magnet-handler, torrent-file-association). Folx, Free Download Manager, Motrix (browser extension 0.1.25 with download handoff, Nov 2026), Downie, Grabbit, Convoy, and WDM ship browser capture.
- **Segmented downloads** (`segmented-downloads`) — A direct file uses one HTTP connection. A partial file can resume with a Range request, and several files can run at once. Nothing fetches one file over several connections. Folx, Free Download Manager, Motrix (aria2), Progressive Downloader, and Convoy (16 connections) do. It is a top ask in r/macapps best download manager threads.

## Organization

| ID | Title | Status | Priority | Source | Summary |
| --- | --- | --- | --- | --- | --- |
| `aggregate-views` | All, in progress, and completed | `shipped` | — | `opul-original` | See every workspace together as all downloads, in progress, or completed. |
| `saved-folders` | Saved folders | `shipped` | — | `opul-original` | A workspace remembers one or more folders, and one of them is the default. |
| `workspace-icons` | Workspace icons and color | `shipped` | — | `opul-original` | Give a workspace an emoji and, for the folder emoji, a color treatment. |
| `workspaces` | Workspaces | `shipped` | — | `opul-original` | Keep downloads in named workspaces. Each one has its own folders and default video format. |
| `duplicate-downloads` | Duplicate downloads | `partial` | — | `opul-original` | Opul avoids adding the same torrent twice, and skips playlist entries already in the workspace. |
| `file-organization` | File organization | `partial` | — | `opul-original` | Put a download's files in their own folder, or leave them in the save location. |

### Details

- **All, in progress, and completed** (`aggregate-views`) — Each of those views remembers its own layout, sort, and grouping. Adding from them can ask which workspace, or use a default workspace you chose.
- **Saved folders** (`saved-folders`) — A download always uses a saved folder in its workspace. A new download uses the default until you pick another.
- **Workspace icons and color** (`workspace-icons`) — Change icon and the creation dialog share one picker. Twelve folder treatments and a custom treatment are saved with the emoji. Older SF Symbol icons still load and are shown as folders.
- **Workspaces** (`workspaces`) — You can create, rename, and delete them. The sidebar always has at least one. Deleting the last one creates an empty Downloads workspace. There is no permanent Torrents workspace.
- **Duplicate downloads** (`duplicate-downloads`) — A second active torrent with the same info hash is rejected. Playlist expansion skips URLs already in that workspace. Pasting the same page again has no separate warning dialog in the interface.
- **File organization** (`file-organization`) — Automatic is the default and uses a subfolder when the download is naturally a bundle. Always create a folder and Keep in the save location exist on the workspace and are applied by the engine. The workspace dialog does not offer a control.

## Interface

| ID | Title | Status | Priority | Source | Summary |
| --- | --- | --- | --- | --- | --- |
| `appearance` | Appearance | `shipped` | — | `opul-original` | Use a light theme, a dark theme, or follow the system, and pick a card or minimal list. |
| `download-inspector` | Download inspector | `shipped` | — | `opul-original` | Open a download to see its progress, source, format, and torrent details. |
| `keyboard-and-menus` | Keyboard and app menu | `shipped` | — | `opul-original` | Start a download, open Settings, and check for updates from the keyboard or the app menu. |
| `library-recovery` | Library recovery | `shipped` | — | `opul-original` | If the library has to be restored from a backup, Opul waits until you acknowledge it. |
| `library-layout` | List, table, sort, and group | `shipped` | — | `opul-original` | Show downloads as a list or a table, then sort or group them. |
| `library-search-and-filters` | Search and status filters | `shipped` | — | `opul-original` | Search the library and narrow it to all, active, finished, paused, or failed downloads. |
| `show-in-finder` | Show in Finder | `shipped` | — | `opul-original` | Reveal a finished download's file in Finder. |
| `guided-browser` | Guided browser | `partial` | — | `competitor:downie` | Open a page, sign in, and pick a media file Opul saw while you browsed. |
| `completion-notifications` | Completion notifications | `planned` | p2 | `competitor:transmission` | Get a macOS notification when a download finishes or fails, and see overall progress on the Dock icon. |

### Details

- **Appearance** (`appearance`) — Both choices are in Settings and apply to the whole app.
- **Download inspector** (`download-inspector`) — Torrent details include the ratio control, files, and trackers. Copying the source link is in the inspector.
- **Keyboard and app menu** (`keyboard-and-menus`) — Command-N starts a download, Command-comma opens Settings, and Command-F focuses search. The menu also sends those actions.
- **Library recovery** (`library-recovery`) — Recovery notices stay up until Acknowledge recovery. Transfers stay fenced until then.
- **List, table, sort, and group** (`library-layout`) — Sort by order, name, state, or source. Group by nothing, state, folder, kind, or source. Cards and a quieter minimal list are a separate appearance setting.
- **Search and status filters** (`library-search-and-filters`) — Command-F focuses the search field.
- **Show in Finder** (`show-in-finder`) — The action is offered when the download has output Opul can reveal.
- **Guided browser** (`guided-browser`) — The Rust core can classify observations and download a chosen candidate with scoped cookies. ADR 0022 describes a WebKit browser. The Tauri app has no guided-browser window and does not call that API.
- **Completion notifications** (`completion-notifications`) — An in-app toast announces a download when it is added. Nothing posts a macOS notification when a download finishes or fails, and the Dock icon shows no transfer progress. Transmission, Folx, Downie, and Murmur offer this.

## System

| ID | Title | Status | Priority | Source | Summary |
| --- | --- | --- | --- | --- | --- |
| `app-updates` | App updates | `shipped` | — | `opul-original` | Opul can check for a signed update, download it, and restart when you ask. |
| `built-in-ffmpeg` | Built-in FFmpeg | `shipped` | — | `opul-original` | FFmpeg and ffprobe ship inside Opul and merge or convert media on the Mac. |
| `quit-stops-transfers` | Quit stops transfers | `shipped` | — | `opul-original` | Quitting Opul stops transfers. Opening it again continues them. |
| `video-tools` | Video tools | `shipped` | — | `yt-dlp` | Optional, verified Python and the official yt-dlp script download video pages. |
| `prevent-sleep` | Prevent sleep | `planned` | p2 | `competitor:transmission` | Keep the Mac awake while downloads are active. |

### Details

- **App updates** (`app-updates`) — Stable and Nightly use separate manifests. Automatic checks are on by default and automatic install is off. Opul does not restart itself while you are using it. Development builds can check but not install.
- **Built-in FFmpeg** (`built-in-ffmpeg`) — One architecture per installer, for macOS 13 and later. There is no path setting. If the tools are missing, post-processing stops and direct files and torrents still work.
- **Quit stops transfers** (`quit-stops-transfers`) — Closing the window quits the app. It does not ask when transfers are still running, and it does not keep a helper running.
- **Video tools** (`video-tools`) — Setup asks first. Tools can stay updated, and Nightly can opt into yt-dlp nightly builds. Removing them keeps partial and finished files. Torrents and direct files do not need them. A page added before tools are ready waits, then starts.
- **Prevent sleep** (`prevent-sleep`) — Nothing keeps the Mac from idle sleep while transfers run. This covers idle sleep only while Opul is open. ADR 0004 still applies: quitting stops transfers, and closing the window quits Opul. Transmission, Folx, Motrix, and Free Download Manager offer this.
