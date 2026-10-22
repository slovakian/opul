import AppKit
import SwiftUI

/// The actions for one or more Downloads. The right-click menu and the Download menu in the
/// menu bar share it, so an action has the same name and shortcut wherever it appears.
///
/// A context menu hides what does not apply. The menu bar keeps every item in place and
/// disables what does not apply, so its layout stays the same as the selection changes.
struct DownloadActionMenu: View, Equatable {
    enum Placement: Equatable {
        case contextMenu
        case menuBar
    }

    let library: DownloadLibrary
    let downloads: [Download]
    let workspaceID: Workspace.ID?
    let savedFolders: [SavedFolder]
    let isMovingDownloads: Bool
    let hasQuickLook: Bool
    let placement: Placement

    init(
        library: DownloadLibrary,
        downloads: [Download],
        workspaceID: Workspace.ID? = nil,
        savedFolders: [SavedFolder]? = nil,
        isMovingDownloads: Bool? = nil,
        hasQuickLook: Bool? = nil,
        placement: Placement
    ) {
        self.library = library
        self.downloads = downloads
        let currentWorkspace = (workspaceID == nil || savedFolders == nil) ? library.selectedWorkspace : nil
        self.workspaceID = workspaceID ?? currentWorkspace?.id
        self.savedFolders = savedFolders ?? currentWorkspace?.savedFolders ?? []
        self.isMovingDownloads = isMovingDownloads ?? library.isMovingDownloads
        self.hasQuickLook = hasQuickLook ?? (library.quickLookURL != nil)
        self.placement = placement
    }

    static func == (lhs: DownloadActionMenu, rhs: DownloadActionMenu) -> Bool {
        lhs.placement == rhs.placement
            && lhs.workspaceID == rhs.workspaceID
            && lhs.savedFolders == rhs.savedFolders
            && lhs.isMovingDownloads == rhs.isMovingDownloads
            && lhs.hasQuickLook == rhs.hasQuickLook
            && lhs.downloads.elementsEqual(rhs.downloads, by: actionEquals)
    }

    private static func actionEquals(_ lhs: Download, _ rhs: Download) -> Bool {
        lhs.id == rhs.id
            && lhs.name == rhs.name
            && lhs.state == rhs.state
            && lhs.source == rhs.source
            && lhs.capabilities == rhs.capabilities
            && lhs.folderID == rhs.folderID
            && lhs.seedRatioLimit == rhs.seedRatioLimit
            && lhs.openableURL == rhs.openableURL
            && lhs.webPageURL == rhs.webPageURL
            && lhs.shareableLink == rhs.shareableLink
            && lhs.distinctPageLink == rhs.distinctPageLink
            && lhs.errorMessage == rhs.errorMessage
            && lhs.magnetLink == rhs.magnetLink
    }

    private var ids: [Download.ID] { downloads.map(\.id) }
    private var single: Download? { downloads.count == 1 ? downloads.first : nil }
    private var isMenuBar: Bool { placement == .menuBar }

    private enum Group: CaseIterable {
        case transfer, open, copy, organize, remove
    }

    var body: some View {
        // A separator goes only between groups that show something, so a context menu
        // never shows two in a row or one at either end.
        let groups = isMenuBar ? Group.allCases : Group.allCases.filter(hasItems)
        ForEach(Array(groups.enumerated()), id: \.offset) { index, group in
            if index > 0 {
                Divider()
            }
            switch group {
            case .transfer: transferSection
            case .open: openSection
            case .copy: copySection
            case .organize: organizeSection
            case .remove: removeSection
            }
        }
    }

    private func hasItems(_ group: Group) -> Bool {
        downloads.contains { download in
            let capabilities = download.capabilities
            return switch group {
            case .transfer:
                capabilities.canResume || capabilities.canRetry || capabilities.canPause
                    || capabilities.canCancel || capabilities.canStopSeeding
            case .open:
                download.openableURL != nil || download.source == .torrent || download.webPageURL != nil
            case .copy:
                download.shareableLink != nil || download.distinctPageLink != nil
                    || download.openableURL != nil || download.errorMessage?.isEmpty == false
            case .organize:
                capabilities.canMove || download.acceptsSeedRatio
            case .remove:
                capabilities.canRemove
            }
        }
    }

    // MARK: Transfer

    @ViewBuilder
    private var transferSection: some View {
        let canResume = downloads.contains { $0.capabilities.canResume }
        let canRetry = downloads.contains { $0.capabilities.canRetry }
        item(
            canResume || !canRetry ? "Resume" : "Retry",
            systemImage: canResume || !canRetry ? "play.fill" : "arrow.clockwise",
            shortcut: KeyboardShortcut("r", modifiers: .command),
            isAvailable: canResume || canRetry
        ) {
            library.resumeOrRetryDownloads(ids)
        }
        item(
            "Pause",
            systemImage: "pause.fill",
            shortcut: KeyboardShortcut("p", modifiers: .command),
            isAvailable: downloads.contains { $0.capabilities.canPause }
        ) {
            library.pauseDownloads(ids)
        }
        item(
            "Cancel",
            systemImage: "xmark.circle",
            shortcut: KeyboardShortcut(".", modifiers: .command),
            isAvailable: downloads.contains { $0.capabilities.canCancel }
        ) {
            library.cancelDownloads(ids)
        }
        item(
            "Stop Seeding",
            systemImage: "arrow.up.circle",
            isAvailable: downloads.contains { $0.capabilities.canStopSeeding }
        ) {
            library.stopSeeding(ids)
        }
    }

    // MARK: Open

    @ViewBuilder
    private var openSection: some View {
        let hasFiles = downloads.contains { $0.openableURL != nil }
        let finishedFiles = downloads.contains { $0.state == .finished && $0.openableURL != nil }
        item(
            "Open",
            systemImage: "arrow.up.forward.app",
            shortcut: KeyboardShortcut(.downArrow, modifiers: .command),
            isAvailable: finishedFiles
        ) {
            library.openOutput(ids.filter { id in downloads.first { $0.id == id }?.state == .finished })
        }
        item(
            quickLookTitle,
            systemImage: "eye",
            shortcut: KeyboardShortcut("y", modifiers: .command),
            isAvailable: finishedFiles || hasQuickLook
        ) {
            library.toggleQuickLook(ids.filter { id in downloads.first { $0.id == id }?.state == .finished })
        }
        item(
            "Show in Finder",
            systemImage: "folder",
            shortcut: KeyboardShortcut("r", modifiers: [.command, .option]),
            isAvailable: hasFiles
        ) {
            library.revealInFinder(ids)
        }
        item(
            "Torrent Info",
            systemImage: "info.circle",
            shortcut: KeyboardShortcut("i", modifiers: .command),
            isAvailable: single?.source == .torrent
        ) {
            if let single { library.inspectorDownloadID = single.id }
        }
        item(
            "Open Page in Browser",
            systemImage: "safari",
            shortcut: KeyboardShortcut("o", modifiers: [.command, .shift]),
            isAvailable: downloads.contains { $0.webPageURL != nil }
        ) {
            library.openPageInBrowser(ids)
        }
    }

    private var quickLookTitle: String {
        if hasQuickLook, isMenuBar { return "Close Quick Look" }
        if let single { return "Quick Look “\(single.name)”" }
        return "Quick Look"
    }

    // MARK: Copy

    @ViewBuilder
    private var copySection: some View {
        let links = downloads.compactMap(\.shareableLink)
        item(
            copyLinkTitle(count: links.count),
            systemImage: "link",
            shortcut: KeyboardShortcut("c", modifiers: [.command, .shift]),
            isAvailable: !links.isEmpty
        ) {
            library.copyLinks(ids)
        }
        item(
            "Copy Page Link",
            systemImage: "doc.on.clipboard",
            isAvailable: downloads.contains { $0.distinctPageLink != nil }
        ) {
            library.copyPageLinks(ids)
        }
        item(
            downloads.count > 1 ? "Copy File Paths" : "Copy File Path",
            systemImage: "doc.on.doc",
            shortcut: KeyboardShortcut("c", modifiers: [.command, .option]),
            isAvailable: downloads.contains { $0.openableURL != nil }
        ) {
            library.copyFilePaths(ids)
        }
        item(
            "Copy Error Message",
            systemImage: "exclamationmark.bubble",
            isAvailable: downloads.contains { $0.state == .failed && !($0.errorMessage ?? "").isEmpty }
        ) {
            library.copyErrorMessages(ids)
        }
    }

    private func copyLinkTitle(count: Int) -> String {
        if downloads.allSatisfy({ $0.source == .torrent }), downloads.contains(where: { $0.magnetLink != nil }) {
            return count > 1 ? "Copy Magnet Links" : "Copy Magnet Link"
        }
        return count > 1 ? "Copy Links" : "Copy Link"
    }

    // MARK: Organize

    @ViewBuilder
    private var organizeSection: some View {
        let seedable = downloads.filter(\.acceptsSeedRatio)
        if isMenuBar || downloads.contains(where: { $0.capabilities.canMove }) {
            moveMenu
        }
        if isMenuBar || !seedable.isEmpty {
            seedRatioMenu(for: seedable)
        }
    }

    @ViewBuilder
    private var moveMenu: some View {
        let movable = downloads.filter(\.capabilities.canMove)
        if movable.isEmpty || workspaceID == nil || savedFolders.isEmpty || isMovingDownloads {
            unavailableSubmenu("Move to", systemImage: "folder.badge.gearshape")
        } else {
            moveMenu(movable)
        }
    }

    private func moveMenu(_ movable: [Download]) -> some View {
        Menu {
            if let workspaceID {
                ForEach(savedFolders) { folder in
                    let isCurrent = movable.allSatisfy { $0.folderID == folder.id }
                    Button {
                        library.moveDownloads(movable.map(\.id), to: folder.id)
                    } label: {
                        if isCurrent {
                            Label(folder.name, systemImage: "checkmark")
                        } else {
                            Text(folder.name)
                        }
                    }
                    .disabled(isCurrent)
                    .help(folder.path)
                }
                Divider()
                Button("New Saved Folder…") {
                    library.beginMoveToNewFolder(movable.map(\.id), in: workspaceID)
                }
            }
        } label: {
            Label("Move to", systemImage: "folder.badge.gearshape")
        }
    }

    /// A submenu with nothing to offer. A menu bar shows it disabled, which a `Menu` cannot be.
    private func unavailableSubmenu(_ title: String, systemImage: String) -> some View {
        Button {} label: {
            Label(title, systemImage: systemImage)
        }
        .disabled(true)
    }

    @ViewBuilder
    private func seedRatioMenu(for torrents: [Download]) -> some View {
        if torrents.isEmpty {
            unavailableSubmenu("Stop Seeding at Ratio", systemImage: "gauge.with.dots.needle.67percent")
        } else {
            seedRatioPicker(for: torrents)
        }
    }

    private func seedRatioPicker(for torrents: [Download]) -> some View {
        let limits = Set(torrents.map(\.seedRatioLimit))
        let current: Double?? = limits.count == 1 ? limits.first : nil
        return Menu {
            Picker(
                "Stop Seeding at Ratio",
                selection: Binding<Double??>(
                    get: { current },
                    set: { choice in
                        if let choice { library.setSeedRatioLimit(torrents.map(\.id), limit: choice) }
                    }
                )
            ) {
                ForEach(Self.seedRatios, id: \.self) { ratio in
                    Text(ratio.formatted(.number.precision(.fractionLength(1)))).tag(Double??.some(ratio))
                }
                Divider()
                Text("Never").tag(Double??.some(nil))
            }
            .pickerStyle(.inline)
            .labelsHidden()
        } label: {
            Label("Stop Seeding at Ratio", systemImage: "gauge.with.dots.needle.67percent")
        }
    }

    private static let seedRatios: [Double] = [0.5, 1, 1.5, 2, 3, 5]

    // MARK: Remove

    @ViewBuilder
    private var removeSection: some View {
        let removable = downloads.contains { $0.capabilities.canRemove }
        item(
            downloads.count > 1 ? "Remove \(downloads.count) Downloads…" : "Remove…",
            systemImage: "minus.circle",
            shortcut: KeyboardShortcut(.delete, modifiers: .command),
            role: .destructive,
            isAvailable: removable
        ) {
            library.askToRemoveDownloads(Set(ids))
        }
        item(
            "Remove and Move to Trash",
            systemImage: "trash",
            shortcut: KeyboardShortcut(.delete, modifiers: [.command, .option]),
            role: .destructive,
            isAvailable: removable
        ) {
            library.removeDownloadsMovingFilesToTrash(ids)
        }
    }

    // MARK: Building blocks

    @ViewBuilder
    private func item(
        _ title: String,
        systemImage: String,
        shortcut: KeyboardShortcut? = nil,
        role: ButtonRole? = nil,
        isAvailable: Bool,
        action: @escaping () -> Void
    ) -> some View {
        if isMenuBar || isAvailable {
            Button(role: role) {
                if isMenuBar {
                    DownloadShortcutGate.perform(action)
                } else {
                    action()
                }
            } label: {
                Label(title, systemImage: systemImage)
            }
            .keyboardShortcut(shortcut)
            .disabled(!isAvailable)
        }
    }
}

private extension Download {
    /// Torrents still transferring or seeding. A finished torrent is past its ratio.
    var acceptsSeedRatio: Bool {
        source == .torrent && [.queued, .fetching, .processing, .paused, .seeding].contains(state)
    }
}

/// Menu bar shortcuts are application-wide, so they would otherwise fire while the person
/// types in a text field or works in a sheet. In those cases the key goes back to the
/// focused text (⌘⌫ still deletes to the start of the line) and the action does not run.
/// A menu item chosen with the mouse always runs.
enum DownloadShortcutGate {
    static func perform(_ action: () -> Void) {
        guard let event = NSApp.currentEvent, event.type == .keyDown,
              let window = NSApp.keyWindow else {
            action()
            return
        }
        if let text = window.firstResponder as? NSText {
            text.keyDown(with: event)
            return
        }
        if window.sheetParent != nil || window.attachedSheet != nil {
            return
        }
        action()
    }
}

/// The Download menu in the menu bar. It acts on the downloads selected in the list.
struct DownloadCommands: Commands {
    let library: DownloadLibrary

    var body: some Commands {
        CommandMenu("Download") {
            DownloadActionMenu(
                library: library,
                downloads: library.selectedDownloads,
                workspaceID: library.selectedWorkspace?.id,
                savedFolders: library.selectedWorkspace?.savedFolders ?? [],
                isMovingDownloads: library.isMovingDownloads,
                hasQuickLook: library.quickLookURL != nil,
                placement: .menuBar
            )
            .equatable()
        }
    }
}
