# Glossary

| Term | Meaning |
| --- | --- |
| Tools | Programs Opul supplies to process video and audio: the built-in tools and optional video tools. Direct files and torrents work without video tools. |
| Built-in tools | FFmpeg and ffprobe in the app's Resources, matching the app's architecture. They merge streams, make MP3 and M4A/AAC, convert/embed subtitles, and make JPG thumbnails. They update with Opul. |
| Video tools | The optional Python runtime and official bare yt-dlp script, downloaded after consent and selected together only after verification and a health check. |
| Python runtime | Opul's signed, numbered package containing Python, curl_cffi and its dependencies, and a sandboxed QuickJS driver. It updates separately from yt-dlp. |
| Setup | The window-filling first-launch offer and shared progress view, also available from Help and Settings ▸ Tools. Opening or dismissing Setup never grants download consent. |
| Not set up | No usable video tools are selected. Opul may offer setup; direct files and torrents work. |
| Turned off | The person declined future offers. Opul suppresses reminders and update checks. Explicit setup re-enables video tools. |
| Setting up | A consented installation is downloading, verifying, or installing Python and yt-dlp. Video Downloads wait; the rest of Opul remains usable. |
| Ready | A verified, health-checked runtime/script pair is selected. Updates preserve that pair until a candidate is safe to activate. |
| Not working | Setup failed or installed tools cannot run. Settings explains the reason and offers recovery and diagnostic details. |
| Download | One durable item owned by one Workspace and directed to one Saved Folder. Its identity and chosen format survive waiting, restart, and tools removal. |
| Workspace | A collection of Downloads and Saved Folders. Playlist deduplication applies within a Workspace. |
| Saved Folder | A persisted destination in a Workspace, chosen before a Download starts. |

The five video tools states are exclusive. A Ready update status (available, updating, failed, or waiting for a newer runtime) is not another state. See [architecture](docs/adr/0026-managed-video-tools.md) and [the module contract](docs/video-tools.md).
