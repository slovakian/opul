# Tauri interface

Opul now uses a Tauri 2 shell with React, Vite, Tailwind, and shadcn/ui. The existing Rust state machine remains the owner of workspaces, downloads, transfer capabilities, recovery, and persistence. Tauri commands adapt its public API; the frontend renders snapshots and sends user intent. It never writes library state.

The shell uses the existing `Application Support/Opul` directory on macOS, including the core's legacy library importer and exclusive state-directory lock. `OPUL_DATA_DIR` and `OPUL_DOWNLOAD_DIR` permit isolated development and smoke tests.

## Interface contract

- Use sidebar-15 as the layout reference: a quiet left sidebar, collapsible emoji workspaces with saved folders, a compact header, and an optional right inspector.
- Every workspace displays an emoji, defaulting to 📁 for missing or legacy symbols.
- Every menu and dropdown composes a shadcn Sidebar inside a Popover, as sidebar-10.
- All shadcn primitives use Base UI. Radix UI is not used.
- Settings compose a Sidebar inside a Dialog, as sidebar-13.
- Downloads use Item components, or a borderless Table. Both share real core state.
- Search, status filtering, sorting, list/table switching, and details are keyboard accessible. Empty, loading, failed-startup, and recovery states are explicit.
- Follow tauri-ui's desktop defaults: hidden until first paint, native window controls, window dragging, no rubber-band scrolling, and native file dialogs.

## Delivery and verification

1. Add the native shell and serialize the existing public machine types.
2. Build the shadcn interface and connect commands and live snapshots.
3. Replace Swift build and release entrypoints with Tauri, document setup.
4. Run core and shell tests, frontend checks, browser interaction tests, and a packaged native smoke test. Open a PR and launch the app.

References: https://github.com/agmmnn/tauri-ui and https://ui.shadcn.com/blocks/sidebar. The official shadcn skill is installed in `.agents/skills/shadcn` using `npx skills add shadcn/ui --skill shadcn -a codex -y`.
