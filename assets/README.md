# App icon sources

`AppIcon.png` is the unchanged, full-bleed artwork from before PR #8. Keep it for branding. Its cream background is opaque, even though Finder displayed the old app with transparent corners.

`AppIcon-macOS.png` preserves that app's native appearance on a 1024px transparent canvas. It was exported with AppKit from Finder's rendering of the original icon on macOS 26.2. It includes the system's mask and margins, without cutting into or shrinking the original artwork by hand. This is the source for the shipped ICNS and PNG. Keeping the native master checked in makes resizing reproducible without depending on the build machine's version of macOS or requiring full Xcode.

The distinction matters because Tauri development sets `NSApplication`'s icon directly from the ICNS. It doesn't use Finder's normalization, so the full-bleed artwork appears square there. Removing the cream background by hand changes the silhouette; macOS then shrinks the icon into an additional gray tile. The native master works both directly and through Finder, preserving the pre-PR appearance.

`Scripts/build_icon.sh` regenerates both Tauri assets from the native master. Tauri runs it before development and production builds, including DMG builds. After packaging, `swift Scripts/check_macos_icon.swift [Opul.app]` compares the actual development assets, bundled ICNS, and Finder icon against this reference at several sizes. The packaged smoke test runs this check in CI and saves previews under `test-results/macos-icon/`.

Provenance of the native master: build the ICNS with the pre-PR `Scripts/build_icon.sh` from commit `aaab364`, place it in a temporary app with `CFBundleIconFile` pointing to it, then render `NSWorkspace.shared.icon(forFile: appPath)` into a 1024px RGBA `NSBitmapImageRep`. The original artwork's SHA-256 is `45656fa471420237fbae262b97801b90478720656b97919bfe3cdcd0204fffbf`.
