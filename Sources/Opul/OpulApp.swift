import AppKit
import SwiftUI

@main
struct OpulApp: App {
    @NSApplicationDelegateAdaptor(OpulDelegate.self) private var delegate
    @State private var library: DownloadLibrary
    @StateObject private var updater = AppUpdater()

    init() {
        if CommandLine.arguments.contains("--check-storage") {
            StorageChecks.run()
            exit(0)
        }

        _library = State(initialValue: DownloadLibrary())

        // A SwiftPM bundle does not show its SwiftUI window until the policy is regular.
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        app.activate()
    }

    var body: some Scene {
        WindowGroup {
            LibraryWindow(library: library)
                .environmentObject(updater)
        }
        .defaultSize(width: 1100, height: 720)
        .windowToolbarStyle(.unified(showsTitle: false))
        .commands {
            CheckForUpdatesCommand(updater: updater)
        }
    }
}

final class OpulDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }
}
