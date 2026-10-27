# Workspace header

The header uses one row for navigation, the current view, a shared search and download input, and display controls. Saved folders show `Workspace › Folder`. The workspace breadcrumb navigates up; the current folder is a label. Narrow headers shorten the identity and hide button text while keeping accessible names. Collapsed or overlay navigation reserves 80px at the left for native macOS window controls. Docked navigation already reserves that space. Empty header areas and the current view label remain draggable.

Typing filters downloads by name or source URL in the active view. Navigation preserves the query, including when switching workspaces. A complete HTTP, HTTPS, or supported BTIH magnet link changes the leading search icon to a link and enables Download. Domains, filenames, incomplete links, and unsupported protocols remain search text. Typing and pasting never submit automatically.

The clear button appears only with text. Escape closes an open popover or dialog first; otherwise it clears the focused shared input. Cmd/Ctrl+F focuses the input. Cmd/Ctrl+N and New download menu actions focus and select its contents, closing overlay navigation when needed. Download options still opens the existing import dialog for video, format, and local torrent choices.

Issue #20 delivers this shell. Contextual and cross-workspace destinations, submission feedback and draft concurrency, split-button options, and the full stepped compression sequence remain in #21 through #24.

`tests/ui/unified-header.spec.ts` covers the shared input, navigation, URL intent, Escape precedence, keyboard/menu focus, overlay navigation, and single-row layout with macOS insets. Existing download and accessibility flows remain in `tests/ui/app.spec.ts`.
