# Real integration regressions found during issue 69

## Embedded playlist entries

The actual yt-dlp 2026.10.26 Generic extractor returns a playlist with two `HTML5MediaEmbed` entries. Both entries have the same `webpage_url`, but each has its own media `url`. Opul preferred the shared page URL, so Workspace deduplication kept only one entry, and a worker could re-download the entire page instead of its individual entry.

The probe now uses individual HTTP media URLs for Generic/HTML5 entries. Other site extractors keep stable watch-page URLs instead of temporary stream URLs. The accepted collection page remains the referer and source page; the reducer still owns ordered expansion, identity, destination, format and per-Workspace deduplication.

`generic_playlist_keeps_distinct_media_urls_when_entries_share_the_page` failed before the fix with `playlist.html` where `first.mp4` was expected. The fixture was refined with the actual `HTML5MediaEmbed` extractor key after the packaged test exposed it. All 18 media probe/progress tests pass, including the companion stable-page regression.
