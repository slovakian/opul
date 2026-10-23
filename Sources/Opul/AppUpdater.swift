import Combine
import Sparkle
import SwiftUI

final class AppUpdater: ObservableObject {
    @Published private(set) var canCheckForUpdates = false
    private let controller: SPUStandardUpdaterController

    init() {
        // Debug builds never start Sparkle, so a development copy is not replaced by a release.
        #if DEBUG
        let startsUpdater = false
        #else
        let startsUpdater = true
        #endif
        controller = SPUStandardUpdaterController(
            startingUpdater: startsUpdater,
            updaterDelegate: nil,
            userDriverDelegate: nil
        )
        controller.updater.publisher(for: \.canCheckForUpdates).assign(to: &$canCheckForUpdates)
    }

    /// One switch for checking and installing. Off means Opul makes no automatic update checks.
    /// Sparkle saves the choice in its own user defaults, which take precedence over the
    /// Info.plist defaults, so it survives a relaunch.
    var updatesAutomatically: Bool {
        get {
            controller.updater.automaticallyChecksForUpdates
                && controller.updater.automaticallyDownloadsUpdates
        }
        set {
            objectWillChange.send()
            controller.updater.automaticallyChecksForUpdates = newValue
            controller.updater.automaticallyDownloadsUpdates = newValue
        }
    }

    func checkForUpdates() {
        controller.checkForUpdates(nil)
    }
}

struct CheckForUpdatesCommand: Commands {
    @ObservedObject var updater: AppUpdater

    var body: some Commands {
        CommandGroup(after: .appInfo) {
            Button("Check for Updates…", action: updater.checkForUpdates)
                .disabled(!updater.canCheckForUpdates)
        }
    }
}
