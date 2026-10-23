import AppKit
import Darwin
import Foundation
import OpulClipboard
import OpulCore

/// Main-actor presentation adapter for the process-wide Rust application state machine.
///
/// This type owns no durable application state. `workspaces` is a projection replaced only by
/// committed Rust snapshots; every mutation below is a typed command.
@Observable
final class DownloadLibrary {
    private(set) var workspaces: [Workspace] = []
    private(set) var revision: UInt64 = 0
    private(set) var machineSession: MachineSessionState = .booting

    var preferences: TorrentPreferences {
        didSet {
            persistPreferences()
            applyLimits()
        }
    }
    var engineNotice: String?
    var recoveryNotice: String?
    var mediaToolStatus: MediaToolStatus?
    private var activeExtractions = 0
    var isExtracting: Bool { activeExtractions > 0 }
    private var mediaImportQueue: [MediaImportRequest] = []
    var pendingMediaImport: MediaImportRequest? {
        get { mediaImportQueue.first }
        set {
            if let newValue {
                if !mediaImportQueue.contains(where: { $0.id == newValue.id }) {
                    mediaImportQueue.append(newValue)
                }
            } else if !mediaImportQueue.isEmpty {
                mediaImportQueue.removeFirst()
            }
        }
    }
    var inspectorDownloadID: Download.ID?
    var selection: SidebarSelection? {
        didSet {
            if selection != oldValue {
                pasteNotice = nil
                searchText = ""
                downloadSelection = DownloadSelection()
            }
        }
    }
    var searchText = ""
    /// Downloads selected in the workspace list. Menu bar commands act on these.
    var downloadSelection = DownloadSelection()
    var selectedDownloadIDs: Set<Download.ID> {
        get { downloadSelection.ids }
        set { downloadSelection.ids = newValue }
    }
    var quickLookURL: URL?
    private(set) var quickLookURLs: [URL] = []
    var pasteNotice: PasteNotice?
    var sheet: LibrarySheet?
    var pendingDelete: DeleteRequest?
    var pendingDownloadRemoval: DownloadRemovalRequest?
    var pendingFolderRemoval: FolderRemovalRequest?
    var pendingFolderReplacement: FolderReplacementRequest?
    var isChoosingFolder = false
    var folderChoice: FolderChoice?
    private var activeMoves = 0

    @ObservationIgnored private var machine: OpulStateMachine?
    @ObservationIgnored private var observationTask: Task<Void, Never>?
    @ObservationIgnored private let soundEngine = InteractionSoundEngine()
    @ObservationIgnored private var canPlayTransferSounds = false
    @ObservationIgnored private var platformEffectsInProgress = Set<String>()
    @ObservationIgnored private var rejectedPlatformEffects = Set<String>()
    @ObservationIgnored private var hasShutdown = false

    init() {
        preferences = Self.loadPreferences()
        do {
            try FileManager.default.createDirectory(
                at: Self.applicationSupportFolder,
                withIntermediateDirectories: true
            )
            let opened = try OpulStateMachine.open(config: machineConfig())
            machine = opened
            mediaToolStatus = opened.mediaToolStatus()
            consume(opened.snapshot())
            applyLimits()
            startObservation(of: opened)
        } catch {
            engineNotice = error.localizedDescription
            recoveryNotice = "Opul could not open its application state. No transfer was started."
        }
        canPlayTransferSounds = true
    }

    deinit {
        observationTask?.cancel()
    }

    var selectedWorkspace: Workspace? {
        guard let selection else { return nil }
        return workspaces.first { $0.id == selection.workspaceID }
    }

    func sidebarEntries() -> [SidebarEntry] {
        workspaces.flatMap { workspace in
            let header = SidebarEntry(
                selection: .workspace(workspace.id),
                title: workspace.name,
                symbolName: workspace.icon.symbolName,
                emoji: workspace.icon.emoji,
                isFolder: false,
                isDefault: false,
                finishedCount: workspace.downloads.count(where: { $0.state == .finished }),
                path: nil,
                workspaceID: workspace.id,
                folderID: nil
            )
            return [header] + workspace.savedFolders.map { folder in
                SidebarEntry(
                    selection: .folder(workspace: workspace.id, folder: folder.id),
                    title: folder.name,
                    symbolName: "folder",
                    emoji: nil,
                    isFolder: true,
                    isDefault: folder.isDefault,
                    finishedCount: 0,
                    path: folder.path,
                    workspaceID: workspace.id,
                    folderID: folder.id
                )
            }
        }
    }

    func visibleDownloads(in workspace: Workspace) -> [Download] {
        guard workspace.savedFolders.count > 1,
              selection?.workspaceID == workspace.id,
              let folderID = selection?.folderID else {
            return workspace.downloads
        }
        return workspace.downloads.filter { $0.folderID == folderID }
    }

    /// The workspace list as shown: the folder filter and then the search text applied.
    func displayedDownloads(in workspace: Workspace) -> [Download] {
        let visible = visibleDownloads(in: workspace)
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return visible }
        return visible.filter { download in
            download.name.localizedStandardContains(query)
                || download.address.localizedStandardContains(query)
        }
    }

    /// Selected downloads that are still shown. A search can hide part of a selection,
    /// and hidden downloads are never acted on.
    var selectedDownloads: [Download] {
        guard !selectedDownloadIDs.isEmpty, let workspace = selectedWorkspace else { return [] }
        return displayedDownloads(in: workspace).filter { selectedDownloadIDs.contains($0.id) }
    }

    func selectDownload(
        _ id: Download.ID,
        extending: Bool,
        toggling: Bool,
        in order: [Download.ID]
    ) {
        downloadSelection.click(id, extending: extending, toggling: toggling, in: order)
    }

    func clearDownloadSelection() {
        downloadSelection = DownloadSelection()
    }

    func pasteClipboard() {
        guard selectedWorkspace != nil else {
            warn("Choose a Workspace before pasting a download.")
            return
        }
        let downloads = clipboardDownloads(in: pasteboardItems(on: .general))
        guard !downloads.isEmpty else {
            warn("Paste a link or a torrent file.")
            return
        }
        searchText = ""
        for download in downloads {
            switch download {
            case .address(let address): paste(address: address)
            case .torrentFile(let path): addTorrentFile(at: URL(fileURLWithPath: path))
            }
        }
    }

    func paste(address: String) {
        let addresses = address
            .components(separatedBy: .newlines)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        guard !addresses.isEmpty else {
            warn("Paste a link.")
            return
        }
        for raw in addresses {
            if raw.lowercased().hasPrefix("magnet:") || isTorrentFile(raw) {
                addTorrentAddress(raw, fileData: nil)
            } else {
                Task { await addMediaAddress(raw) }
            }
        }
    }

    func addGuidedDownload(
        candidate: GuidedCandidate,
        session: GuidedExtractionSession,
        cookies: String,
        userAgent: String?
    ) -> String? {
        guard let machine,
              let workspace = selectedWorkspace,
              let folderID = folderIDForPaste(in: workspace) else {
            let message = "Choose a Workspace and download folder before adding guided media."
            warn(message)
            return message
        }
        let format: Format = candidate.directDownload ? .original : workspace.defaultFormat
        let item = MachineMediaItem(
            url: candidate.url,
            title: candidate.title,
            pageUrl: candidate.referer,
            thumbnailUrl: nil,
            extractor: "User-guided extraction",
            playlistIndex: nil
        )
        let request = MachineMediaRequest(
            requestId: UUID().uuidString,
            expectedRevision: nil,
            workspaceId: workspace.id.uuidString,
            folderId: folderID.uuidString,
            format: format.machine,
            policy: mediaPolicy(userAgent: userAgent, direct: candidate.directDownload),
            items: [item]
        )
        do {
            _ = try machine.addGuidedMedia(
                request: request,
                session: session,
                selection: GuidedDownloadSelection(
                    candidateId: candidate.id,
                    userAgent: userAgent,
                    cookies: cookies
                )
            )
            refreshTransfers()
            pasteNotice = nil
            playDownloadAddedSound()
            return nil
        } catch {
            let message = error.localizedDescription
            warn(message)
            return message
        }
    }

    func clearGuidedLoginData() {
        do {
            try machine?.clearGuidedLoginData()
            pasteNotice = PasteNotice(
                text: "Browser and queued guided login data were cleared. Active transfers may need retrying.",
                isWarning: false
            )
        } catch {
            warn(error.localizedDescription)
        }
    }

    func addTorrentFile(at url: URL) {
        let accessed = url.startAccessingSecurityScopedResource()
        defer { if accessed { url.stopAccessingSecurityScopedResource() } }
        do {
            addTorrentAddress(
                url.path(percentEncoded: false),
                fileData: try Data(contentsOf: url)
            )
        } catch {
            warn(error.localizedDescription)
        }
    }

    func open(_ url: URL) {
        if url.isFileURL {
            addTorrentFile(at: url)
        } else if url.scheme?.lowercased() == "opul" {
            let components = URLComponents(url: url, resolvingAgainstBaseURL: false)
            if let address = components?.queryItems?.first(where: { $0.name == "url" })?.value {
                paste(address: address)
            }
        } else {
            paste(address: url.absoluteString)
        }
    }

    func findDownload(_ id: Download.ID) -> Download? {
        workspaces.lazy.flatMap(\.downloads).first { $0.id == id }
    }

    func previewSound(_ category: InteractionSoundCategory) {
        soundEngine.playRandom(category, preferences: preferences.interactionSounds)
    }

    func folderName(for folderID: SavedFolder.ID) -> String {
        folder(for: folderID)?.name ?? "Folder"
    }

    func pauseDownload(_ id: Download.ID) {
        if performDownloadCommand(id, machineMethod: { try $0.pauseDownload(request: $1) }) {
            soundEngine.play(.pause, preferences: preferences.interactionSounds)
        }
    }

    func resumeDownload(_ id: Download.ID) {
        if performDownloadCommand(id, machineMethod: { try $0.resumeDownload(request: $1) }) {
            soundEngine.play(.resume, preferences: preferences.interactionSounds)
        }
    }

    func cancelDownload(_ id: Download.ID) {
        _ = performDownloadCommand(id, machineMethod: { try $0.cancelDownload(request: $1) })
    }

    func retryDownload(_ id: Download.ID) {
        _ = performDownloadCommand(id, machineMethod: { try $0.retryDownload(request: $1) })
    }

    func stopSeeding(_ id: Download.ID) {
        if performDownloadCommand(id, machineMethod: { try $0.stopSeeding(request: $1) }) {
            soundEngine.play(.pause, preferences: preferences.interactionSounds)
        }
    }

    func askToRemoveDownload(_ id: Download.ID) {
        askToRemoveDownloads([id])
    }

    func askToRemoveDownloads(_ ids: Set<Download.ID>) {
        let downloads = workspaces.flatMap(\.downloads).filter { ids.contains($0.id) }
        guard !downloads.isEmpty else { return }
        pendingDownloadRemoval = DownloadRemovalRequest(downloadIDs: downloads.map(\.id))
    }

    func removePendingDownloads(moveFilesToTrash: Bool) {
        guard let request = pendingDownloadRemoval else { return }
        pendingDownloadRemoval = nil
        if retireDownloads(request.downloadIDs, moveFilesToTrash: moveFilesToTrash) {
            soundEngine.playRandom(.success, preferences: preferences.interactionSounds)
        }
    }

    func clearFinished(in workspaceID: Workspace.ID) {
        let ids = workspaces
            .first(where: { $0.id == workspaceID })?
            .downloads
            .filter { $0.state == .finished || $0.state == .cancelled }
            .map(\.id) ?? []
        if retireDownloads(ids, moveFilesToTrash: false) {
            soundEngine.playRandom(.success, preferences: preferences.interactionSounds)
        }
    }

    func selectAllFiles(_ id: Download.ID, included: Bool) {
        guard var download = findDownload(id) else { return }
        for index in download.files.indices {
            download.files[index].isIncluded = included
            download.files[index].priority = included ? .normal : .skip
        }
        setTorrentFiles(download)
    }

    func toggleFileInclusion(downloadID: Download.ID, fileIndex: Int) {
        guard var download = findDownload(downloadID),
              let index = download.files.firstIndex(where: { $0.fileIndex == fileIndex }) else {
            return
        }
        download.files[index].isIncluded.toggle()
        download.files[index].priority = download.files[index].isIncluded ? .normal : .skip
        setTorrentFiles(download)
    }

    func setFilePriority(
        downloadID: Download.ID,
        fileIndex: Int,
        priority: TorrentFilePriority
    ) {
        guard var download = findDownload(downloadID),
              let index = download.files.firstIndex(where: { $0.fileIndex == fileIndex }) else {
            return
        }
        download.files[index].priority = priority
        download.files[index].isIncluded = priority != .skip
        setTorrentFiles(download)
    }

    func setSeedRatioLimit(_ id: Download.ID, limit: Double?) {
        guard let machine else { return }
        do {
            _ = try machine.setSeedRatio(request: downloadCommand(id), limit: limit)
            refreshTransfers()
        } catch {
            warn(error.localizedDescription)
        }
    }

    func copyInfoHash(_ id: Download.ID) {
        guard let hash = findDownload(id)?.infoHash else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(hash, forType: .string)
    }

    func copyMagnetURI(_ id: Download.ID) {
        guard let magnet = findDownload(id)?.magnetLink else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(magnet, forType: .string)
    }

    func forceRecheck(_ id: Download.ID) {
        pasteNotice = PasteNotice(
            text: "Opul checks pieces when a torrent is added or resumed.",
            isWarning: false
        )
        _ = id
    }

    func revealInFinder(_ id: Download.ID) {
        revealInFinder([id])
    }

    func revealInFinder(_ ids: [Download.ID]) {
        let urls = ids.compactMap(findDownload).compactMap { download in
            (download.outputPath ?? download.contentFolder ?? folder(for: download.folderID)?.path)
                .map { URL(fileURLWithPath: $0) }
        }
        guard !urls.isEmpty else { return }
        NSWorkspace.shared.activateFileViewerSelecting(urls)
    }

    // MARK: Download actions
    //
    // Each action takes the downloads it targets and applies itself to the ones it fits,
    // so a mixed selection never fails as a whole.

    func pauseDownloads(_ ids: [Download.ID]) {
        for id in ids where findDownload(id)?.capabilities.canPause == true {
            pauseDownload(id)
        }
    }

    /// Resumes paused downloads and retries failed ones: both mean "get this going again".
    func resumeOrRetryDownloads(_ ids: [Download.ID]) {
        for id in ids {
            guard let capabilities = findDownload(id)?.capabilities else { continue }
            if capabilities.canResume {
                resumeDownload(id)
            } else if capabilities.canRetry {
                retryDownload(id)
            }
        }
    }

    func cancelDownloads(_ ids: [Download.ID]) {
        for id in ids where findDownload(id)?.capabilities.canCancel == true {
            cancelDownload(id)
        }
    }

    func stopSeeding(_ ids: [Download.ID]) {
        for id in ids where findDownload(id)?.capabilities.canStopSeeding == true {
            stopSeeding(id)
        }
    }

    func setSeedRatioLimit(_ ids: [Download.ID], limit: Double?) {
        for id in ids where findDownload(id)?.source == .torrent {
            setSeedRatioLimit(id, limit: limit)
        }
    }

    func openOutput(_ ids: [Download.ID]) {
        for url in ids.compactMap(findDownload).compactMap(\.openableURL) {
            NSWorkspace.shared.open(url)
        }
    }

    /// Opens Quick Look on the downloads' files. Asking again for the same files closes it,
    /// the way the space bar does in Finder.
    func toggleQuickLook(_ ids: [Download.ID]) {
        let urls = ids.compactMap(findDownload).compactMap(\.openableURL)
        if quickLookURL != nil, urls.isEmpty || urls == quickLookURLs {
            quickLookURL = nil
            return
        }
        guard let first = urls.first else { return }
        quickLookURLs = urls
        quickLookURL = first
    }

    func openPageInBrowser(_ ids: [Download.ID]) {
        for url in ids.compactMap(findDownload).compactMap(\.webPageURL) {
            NSWorkspace.shared.open(url)
        }
    }

    func copyLinks(_ ids: [Download.ID]) {
        copyLines(ids.compactMap(findDownload).compactMap(\.shareableLink))
    }

    func copyPageLinks(_ ids: [Download.ID]) {
        copyLines(ids.compactMap(findDownload).compactMap(\.distinctPageLink))
    }

    func copyFilePaths(_ ids: [Download.ID]) {
        copyLines(ids.compactMap(findDownload).compactMap(\.openableURL).map {
            $0.path(percentEncoded: false)
        })
    }

    func copyErrorMessages(_ ids: [Download.ID]) {
        copyLines(ids.compactMap(findDownload).compactMap { download in
            guard let message = download.errorMessage, !message.isEmpty else { return nil }
            return ids.count == 1 ? message : "\(download.name): \(message)"
        })
    }

    /// Removes without asking. Files go to the Trash, where the person can still recover them.
    func removeDownloadsMovingFilesToTrash(_ ids: [Download.ID]) {
        let removable = ids.filter { findDownload($0)?.capabilities.canRemove == true }
        if retireDownloads(removable, moveFilesToTrash: true) {
            soundEngine.playRandom(.success, preferences: preferences.interactionSounds)
        }
    }

    /// Moves finished downloads, files and all, into another saved folder of their Workspace.
    /// Files move first; the state machine records the new location only once they have.
    func moveDownloads(_ ids: [Download.ID], to folderID: SavedFolder.ID) {
        guard let target = folder(for: folderID) else { return }
        let candidates = ids.compactMap(findDownload).filter { $0.folderID != folderID }
        let movable = candidates.filter(\.capabilities.canMove)
        let skipped = candidates.count - movable.count
        guard !movable.isEmpty else {
            if skipped > 0 {
                warn("Only finished downloads can move to another folder.")
            }
            return
        }
        activeMoves += 1
        Task {
            defer { activeMoves -= 1 }
            var failures: [String] = []
            for download in movable {
                do {
                    try await move(download, to: target)
                } catch {
                    failures.append(error.localizedDescription)
                }
            }
            refreshTransfers()
            if let failure = failures.first {
                warn(failure)
            } else if skipped > 0 {
                pasteNotice = PasteNotice(
                    text: "Moved \(movable.count) to \(target.name). Unfinished downloads stay where they are.",
                    isWarning: false
                )
            } else {
                soundEngine.playRandom(.success, preferences: preferences.interactionSounds)
            }
        }
    }

    func beginMoveToNewFolder(_ ids: [Download.ID], in workspaceID: Workspace.ID) {
        folderChoice = .addAndMove(workspace: workspaceID, downloads: ids)
        isChoosingFolder = true
    }

    var isMovingDownloads: Bool { activeMoves > 0 }

    func updateTrackers(_ id: Download.ID) {
        refreshTransfers()
        _ = id
    }

    /// Compatibility entry point for views. This reads one immutable projection; engines are
    /// observed by the Rust runner and never polled by Swift.
    func refreshTransfers() {
        guard let machine else { return }
        consume(machine.snapshot())
    }

    func createWorkspace(name: String, icon: WorkspaceIcon, folderPath: String? = nil) {
        guard let machine else { return }
        let workspaceID = UUID()
        let folderID = UUID()
        let path = folderPath ?? MacFolder.path(for: .downloadsDirectory)
        let folderName = displayName(forFolderPath: path, fallback: name)
        do {
            _ = try machine.createWorkspace(request: MachineWorkspaceRequest(
                requestId: UUID().uuidString,
                expectedRevision: nil,
                workspaceId: workspaceID.uuidString,
                name: name,
                icon: icon.machine,
                folderId: folderID.uuidString,
                folderName: folderName,
                folderPath: path,
                defaultFormat: MachineFormat.best,
                fileOrganization: preferences.fileOrganization.machine
            ))
            consume(machine.snapshot())
            selection = .workspace(workspaceID)
        } catch {
            warn(error.localizedDescription)
        }
    }

    func updateWorkspaceSettings(
        _ id: Workspace.ID,
        to name: String,
        icon: WorkspaceIcon,
        fileOrganizationOverride: DownloadFileOrganization?
    ) {
        guard let machine,
              let workspace = workspaces.first(where: { $0.id == id }),
              let folder = workspace.savedFolders.first(where: \.isDefault)
                ?? workspace.savedFolders.first else { return }
        do {
            _ = try machine.updateWorkspace(request: MachineWorkspaceRequest(
                requestId: UUID().uuidString,
                expectedRevision: nil,
                workspaceId: id.uuidString,
                name: name,
                icon: icon.machine,
                folderId: folder.id.uuidString,
                folderName: folder.name,
                folderPath: folder.path,
                defaultFormat: workspace.defaultFormat.machine,
                fileOrganization: (fileOrganizationOverride ?? preferences.fileOrganization).machine
            ))
            refreshTransfers()
        } catch {
            warn(error.localizedDescription)
        }
    }

    func askToDelete(_ id: Workspace.ID) {
        guard let workspace = workspaces.first(where: { $0.id == id }) else { return }
        let hasDownloads = !workspace.downloads.isEmpty
        var message = workspaces.count == 1
            ? "A Downloads Workspace will be created afterward, empty of Downloads."
            : "\(workspace.name) and its Downloads will leave Opul."
        if hasDownloads {
            message += " Active transfers stop. Files stay where they are unless you choose to move them, including partially downloaded files, to the Trash."
        }
        pendingDelete = DeleteRequest(
            workspaceID: id,
            title: "Delete \(workspace.name)?",
            message: message,
            hasDownloads: hasDownloads
        )
    }

    func deletePending(moveFilesToTrash: Bool) {
        guard let machine, let request = pendingDelete else { return }
        pendingDelete = nil
        let replacement = workspaces.count == 1 ? freshWorkspaceRequest() : nil
        do {
            _ = try machine.deleteWorkspace(
                requestId: UUID().uuidString,
                expectedRevision: nil,
                workspaceIdValue: request.workspaceID.uuidString,
                replacement: replacement,
                moveFilesToTrash: moveFilesToTrash
            )
            refreshTransfers()
            if let first = workspaces.first {
                selection = .workspace(first.id)
            }
        } catch {
            warn(error.localizedDescription)
        }
    }

    func setView(_ settings: WorkspaceViewSettings, on id: Workspace.ID) {
        guard let machine else { return }
        do {
            _ = try machine.setWorkspaceView(
                requestId: UUID().uuidString,
                expectedRevision: nil,
                workspaceIdValue: id.uuidString,
                layout: settings.layout.machine,
                sort: settings.sort.machine,
                grouping: settings.grouping.machine
            )
            refreshTransfers()
        } catch {
            warn(error.localizedDescription)
        }
    }

    func setDefaultFormat(_ format: Format, on id: Workspace.ID) {
        guard let machine else { return }
        do {
            _ = try machine.setDefaultFormat(
                requestId: UUID().uuidString,
                expectedRevision: nil,
                workspaceIdValue: id.uuidString,
                format: format.machine
            )
            refreshTransfers()
        } catch {
            warn(error.localizedDescription)
        }
    }

    func makeDefault(_ folderID: SavedFolder.ID, in workspaceID: Workspace.ID) {
        guard let machine, let folder = folder(for: folderID) else { return }
        do {
            _ = try machine.setDefaultFolder(request: folderRequest(
                workspaceID: workspaceID,
                folderID: folderID,
                name: folder.name,
                path: folder.path
            ))
            refreshTransfers()
        } catch {
            warn(error.localizedDescription)
        }
    }

    func downloadCount(in folderID: SavedFolder.ID, workspaceID: Workspace.ID) -> Int {
        workspaces.first(where: { $0.id == workspaceID })?
            .downloads.count(where: { $0.folderID == folderID }) ?? 0
    }

    func revealFolder(_ folderID: SavedFolder.ID) {
        guard let folder = folder(for: folderID) else { return }
        NSWorkspace.shared.activateFileViewerSelecting([
            URL(fileURLWithPath: folder.path, isDirectory: true),
        ])
    }

    func askToRemoveFolder(_ folderID: SavedFolder.ID, in workspaceID: Workspace.ID) {
        guard let workspace = workspaces.first(where: { $0.id == workspaceID }),
              workspace.savedFolders.count > 1,
              let folder = workspace.savedFolders.first(where: { $0.id == folderID }) else {
            return
        }
        let downloads = workspace.downloads.filter { $0.folderID == folderID }
        guard !downloads.isEmpty else {
            removeFolder(folderID, from: workspaceID)
            return
        }
        pendingFolderRemoval = FolderRemovalRequest(
            workspaceID: workspaceID,
            folderID: folderID,
            folderName: folder.name,
            downloadIDs: downloads.map(\.id)
        )
    }

    func removePendingFolder(moveFilesToTrash: Bool) {
        guard let machine, let request = pendingFolderRemoval else { return }
        pendingFolderRemoval = nil
        do {
            _ = try machine.removeFolder(
                request: folderRequest(
                    workspaceID: request.workspaceID,
                    folderID: request.folderID,
                    name: request.folderName,
                    path: folder(for: request.folderID)?.path ?? ""
                ),
                moveFilesToTrash: moveFilesToTrash
            )
            refreshTransfers()
            if selection?.folderID == request.folderID {
                selection = .workspace(request.workspaceID)
            }
        } catch {
            warn(error.localizedDescription)
        }
    }

    func beginAddFolder(in workspaceID: Workspace.ID) {
        folderChoice = .add(workspaceID)
        isChoosingFolder = true
    }

    func beginChooseDefaultFolder(in workspaceID: Workspace.ID) {
        folderChoice = .addAsDefault(workspaceID)
        isChoosingFolder = true
    }

    func beginReplaceFolder(_ folderID: SavedFolder.ID, in workspaceID: Workspace.ID) {
        folderChoice = .replace(workspace: workspaceID, folder: folderID)
        isChoosingFolder = true
    }

    func useChosenFolder(_ result: Result<[URL], Error>) {
        let choice = folderChoice
        folderChoice = nil
        guard let choice, case .success(let urls) = result, let url = urls.first else { return }
        let accessed = url.startAccessingSecurityScopedResource()
        defer { if accessed { url.stopAccessingSecurityScopedResource() } }
        let path = url.path(percentEncoded: false)
        let name = url.lastPathComponent.isEmpty ? "Folder" : url.lastPathComponent
        switch choice {
        case .add(let workspaceID):
            addFolder(name: name, path: path, workspaceID: workspaceID, makeDefault: false)
        case .addAsDefault(let workspaceID):
            addFolder(name: name, path: path, workspaceID: workspaceID, makeDefault: true)
        case .addAndMove(let workspaceID, let downloadIDs):
            if let folderID = addFolder(name: name, path: path, workspaceID: workspaceID, makeDefault: false) {
                moveDownloads(downloadIDs, to: folderID)
            }
        case .replace(let workspaceID, let folderID):
            guard let workspace = workspaces.first(where: { $0.id == workspaceID }),
                  let folder = workspace.savedFolders.first(where: { $0.id == folderID }),
                  folder.path != path else { return }
            let downloads = workspace.downloads.filter { $0.folderID == folderID }
            if downloads.isEmpty {
                relinkFolder(folderID, in: workspaceID, name: name, path: path)
            } else {
                pendingFolderReplacement = FolderReplacementRequest(
                    workspaceID: workspaceID,
                    folderID: folderID,
                    folderName: folder.name,
                    replacementName: name,
                    replacementPath: path,
                    downloadCount: downloads.count,
                    canRelink: downloads.allSatisfy { $0.state == .finished }
                )
            }
        }
    }

    func resolvePendingFolderReplacement(relink: Bool) {
        guard let request = pendingFolderReplacement else { return }
        pendingFolderReplacement = nil
        if relink, request.canRelink {
            relinkFolder(
                request.folderID,
                in: request.workspaceID,
                name: request.replacementName,
                path: request.replacementPath
            )
        } else {
            addFolder(
                name: request.replacementName,
                path: request.replacementPath,
                workspaceID: request.workspaceID,
                makeDefault: true
            )
        }
    }

    func acknowledgeRecovery() {
        guard let machine else { return }
        do {
            _ = try machine.acknowledgeRecovery()
            refreshTransfers()
        } catch {
            engineNotice = error.localizedDescription
        }
    }

    func shutdown() {
        guard !hasShutdown, let machine else { return }
        hasShutdown = true
        observationTask?.cancel()
        do {
            _ = try machine.beginShutdown()
            consume(machine.snapshot())
        } catch {
            engineNotice = error.localizedDescription
        }
    }

    private func addTorrentAddress(_ address: String, fileData: Data?) {
        guard let machine,
              let workspace = selectedWorkspace,
              let folderID = folderIDForPaste(in: workspace) else {
            warn("Choose a Workspace and download folder before adding a torrent.")
            return
        }
        do {
            _ = try machine.addTorrent(request: MachineTorrentRequest(
                requestId: UUID().uuidString,
                expectedRevision: nil,
                workspaceId: workspace.id.uuidString,
                folderId: folderID.uuidString,
                source: address,
                metainfo: fileData,
                seedRatioLimit: defaultSeedLimit()
            ))
            refreshTransfers()
            pasteNotice = nil
            playDownloadAddedSound()
        } catch {
            warn(error.localizedDescription)
        }
    }

    private func addMediaAddress(_ address: String) async {
        guard let machine,
              let workspace = selectedWorkspace,
              let folderID = folderIDForPaste(in: workspace) else {
            warn("Choose a Workspace and download folder before adding media.")
            return
        }
        activeExtractions += 1
        defer { activeExtractions -= 1 }
        do {
            let probe = try await Task.detached(priority: .userInitiated) {
                try machine.probeMedia(url: address)
            }.value
            pendingMediaImport = MediaImportRequest(
                workspaceID: workspace.id,
                folderID: folderID,
                probe: probe
            )
        } catch {
            warn(error.localizedDescription)
        }
    }

    func queueMediaItems(
        _ request: MediaImportRequest,
        itemIndexes: Set<Int>,
        format: Format,
        folderID: SavedFolder.ID
    ) {
        guard let machine,
              let workspace = workspaces.first(where: { $0.id == request.workspaceID }),
              workspace.savedFolders.contains(where: { $0.id == folderID }) else {
            warn("Choose an available download folder before adding media.")
            return
        }
        let items = request.probe.items.indices
            .filter(itemIndexes.contains)
            .map { request.probe.items[$0] }
            .map { item in
                MachineMediaItem(
                    url: item.webpageUrl,
                    title: item.title,
                    pageUrl: request.probe.webpageUrl,
                    thumbnailUrl: item.thumbnailUrl ?? request.probe.thumbnailUrl,
                    extractor: item.extractor ?? request.probe.extractor,
                    playlistIndex: item.playlistIndex
                )
            }
        guard !items.isEmpty else {
            warn("No downloads were selected.")
            return
        }
        do {
            let receipt = try machine.addMedia(request: MachineMediaRequest(
                requestId: UUID().uuidString,
                expectedRevision: nil,
                workspaceId: request.workspaceID.uuidString,
                folderId: folderID.uuidString,
                format: format.machine,
                policy: mediaPolicy(userAgent: nil, direct: false),
                items: items
            ))
            refreshTransfers()
            if receipt.createdDownloadIds.isEmpty {
                warn("Those downloads are already in this Workspace.")
            } else {
                pasteNotice = nil
                playDownloadAddedSound()
            }
        } catch {
            warn(error.localizedDescription)
        }
    }

    private func performDownloadCommand(
        _ id: Download.ID,
        machineMethod: (OpulStateMachine, MachineDownloadCommand) throws -> MachineCommandReceipt
    ) -> Bool {
        guard let machine else { return false }
        do {
            _ = try machineMethod(machine, downloadCommand(id))
            refreshTransfers()
            return true
        } catch {
            warn(error.localizedDescription)
            return false
        }
    }

    private func setTorrentFiles(_ download: Download) {
        guard let machine else { return }
        do {
            _ = try machine.setTorrentFiles(
                request: downloadCommand(download.id),
                files: download.files.sorted { $0.fileIndex < $1.fileIndex }.map { file in
                    MachineTorrentFile(
                        index: UInt32(clamping: file.fileIndex),
                        path: file.path,
                        size: UInt64(clamping: file.size),
                        priority: file.priority.rawValue
                    )
                }
            )
            refreshTransfers()
        } catch {
            warn(error.localizedDescription)
        }
    }

    private func retireDownloads(_ ids: [Download.ID], moveFilesToTrash: Bool) -> Bool {
        guard !ids.isEmpty, let machine else { return false }
        do {
            _ = try machine.removeDownloads(
                requestId: UUID().uuidString,
                expectedRevision: nil,
                downloadIds: ids.map(\.uuidString),
                moveFilesToTrash: moveFilesToTrash
            )
            refreshTransfers()
            return true
        } catch {
            warn(error.localizedDescription)
            return false
        }
    }

    private func removeFolder(_ folderID: SavedFolder.ID, from workspaceID: Workspace.ID) {
        guard let machine, let folder = folder(for: folderID) else { return }
        do {
            _ = try machine.removeFolder(
                request: folderRequest(
                    workspaceID: workspaceID,
                    folderID: folderID,
                    name: folder.name,
                    path: folder.path
                ),
                moveFilesToTrash: false
            )
            refreshTransfers()
        } catch {
            warn(error.localizedDescription)
        }
    }

    @discardableResult
    private func addFolder(
        name: String,
        path: String,
        workspaceID: Workspace.ID,
        makeDefault: Bool
    ) -> SavedFolder.ID? {
        guard let machine,
              let workspace = workspaces.first(where: { $0.id == workspaceID }) else { return nil }
        if let existing = workspace.savedFolders.first(where: { $0.path == path }) {
            if makeDefault { self.makeDefault(existing.id, in: workspaceID) }
            selection = .folder(workspace: workspaceID, folder: existing.id)
            return existing.id
        }
        let folderID = UUID()
        do {
            _ = try machine.addFolder(
                request: folderRequest(
                    workspaceID: workspaceID,
                    folderID: folderID,
                    name: name,
                    path: path
                ),
                makeDefault: makeDefault
            )
            refreshTransfers()
            selection = .folder(workspace: workspaceID, folder: folderID)
            return folderID
        } catch {
            warn(error.localizedDescription)
            return nil
        }
    }

    private func move(_ download: Download, to target: SavedFolder) async throws {
        guard let machine, let outputPath = download.outputPath else { return }
        // Move the outermost artifact: the download's own folder when it has one, else its file.
        let source = URL(fileURLWithPath: download.contentFolder ?? outputPath).standardizedFileURL
        let destination = URL(fileURLWithPath: target.path, isDirectory: true)
            .appending(path: source.lastPathComponent)
            .standardizedFileURL
        guard let movedOutput = DownloadMove.rebasedPath(outputPath, from: source, to: destination) else {
            throw DownloadMove.Failure.outsideOwnFolder(download.name)
        }
        guard !FileManager.default.fileExists(atPath: destination.path(percentEncoded: false)) else {
            throw DownloadMove.Failure.nameTaken(source.lastPathComponent, folder: target.name)
        }
        // A move across volumes copies every byte, so keep it off the main actor.
        try await Task.detached(priority: .userInitiated) {
            try FileManager.default.moveItem(at: source, to: destination)
        }.value
        do {
            _ = try machine.moveDownload(
                request: downloadCommand(download.id),
                folderIdValue: target.id.uuidString,
                outputPath: movedOutput,
                contentFolder: download.contentFolder == nil ? nil : destination.path(percentEncoded: false)
            )
        } catch {
            // The record still points at the old place, so put the files back there.
            try? await Task.detached(priority: .userInitiated) {
                try FileManager.default.moveItem(at: destination, to: source)
            }.value
            throw error
        }
    }

    private func copyLines(_ lines: [String]) {
        guard !lines.isEmpty else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(lines.joined(separator: "\n"), forType: .string)
    }

    private func relinkFolder(
        _ folderID: SavedFolder.ID,
        in workspaceID: Workspace.ID,
        name: String,
        path: String
    ) {
        guard let machine else { return }
        do {
            _ = try machine.relinkFolder(request: folderRequest(
                workspaceID: workspaceID,
                folderID: folderID,
                name: name,
                path: path
            ))
            refreshTransfers()
        } catch {
            warn(error.localizedDescription)
        }
    }

    private func startObservation(of machine: OpulStateMachine) {
        let initial = machine.snapshot()
        observationTask = Task.detached(priority: .utility) { [weak self, machine] in
            var boot = initial.bootId
            var sequence = initial.sequence
            while !Task.isCancelled {
                let snapshot = machine.waitForSnapshot(
                    afterBootId: boot,
                    afterSequence: sequence,
                    timeoutMilliseconds: 30_000
                )
                guard !Task.isCancelled else { return }
                boot = snapshot.bootId
                sequence = snapshot.sequence
                await self?.consume(snapshot)
            }
        }
    }

    private func consume(_ snapshot: MachineSnapshot) {
        let previousStates = Dictionary(
            uniqueKeysWithValues: workspaces.flatMap(\.downloads).map { ($0.id, $0.state) }
        )
        revision = snapshot.revision
        machineSession = snapshot.session
        workspaces = snapshot.workspaces.compactMap(workspace(from:))
        if !selectedDownloadIDs.isEmpty {
            let remaining = selectedDownloadIDs.filter { findDownload($0) != nil }
            if remaining != selectedDownloadIDs {
                selectedDownloadIDs = remaining
            }
        }
        let notices = snapshot.recoveryNotices
        if !notices.isEmpty {
            recoveryNotice = notices.joined(separator: " ")
        } else if rejectedPlatformEffects.isEmpty {
            recoveryNotice = nil
        }
        if snapshot.session == .readOnly {
            engineNotice = "Opul entered read-only mode because its durable state could not be saved."
        }
        if selection == nil, let first = workspaces.first {
            selection = .workspace(first.id)
        } else if let selection, !contains(selection), let first = workspaces.first {
            self.selection = .workspace(first.id)
        }
        for download in workspaces.flatMap(\.downloads) {
            playSoundForTransition(from: previousStates[download.id], to: download.state)
        }
        processPlatformEffects()
    }

    private func workspace(from value: MachineWorkspace) -> Workspace? {
        guard let id = UUID(uuidString: value.id) else { return nil }
        var workspace = Workspace(
            id: id,
            name: value.name,
            icon: value.icon.presentation,
            savedFolders: value.folders.compactMap { folder in
                guard let id = UUID(uuidString: folder.id) else { return nil }
                return SavedFolder(
                    id: id,
                    name: folder.name,
                    path: folder.path,
                    isDefault: folder.isDefault
                )
            },
            defaultFormat: value.defaultFormat.presentation,
            view: WorkspaceViewSettings(
                layout: value.layout.presentation,
                sort: value.sort.presentation,
                grouping: value.grouping.presentation
            ),
            downloads: value.downloads.compactMap(download(from:))
        )
        workspace.fileOrganizationOverride = value.fileOrganization.presentation
        return workspace
    }

    private func download(from value: MachineDownload) -> Download? {
        guard let id = UUID(uuidString: value.id),
              let folderID = UUID(uuidString: value.folderId) else { return nil }
        let format = value.format?.presentation
        let files = value.files.map { file in
            TorrentFileItem(
                fileIndex: Int(file.index),
                path: file.path,
                name: URL(fileURLWithPath: file.path).lastPathComponent,
                size: Int64(clamping: file.size),
                isIncluded: file.priority != TorrentFilePriority.skip.rawValue,
                priority: TorrentFilePriority(rawValue: file.priority) ?? .normal
            )
        }
        let kind: Kind? = if value.source == .torrent {
            kind(forLargest: files)
        } else if let format {
            kind(for: value.sourceUrl, format: format)
        } else {
            nil
        }
        return Download(
            id: id,
            name: value.name,
            address: value.sourceUrl,
            pageURL: value.pageUrl,
            state: value.state.presentation,
            source: value.source.presentation,
            kind: kind,
            format: format,
            folderID: folderID,
            infoHash: value.infoHash,
            totalSize: Int64(clamping: value.totalSize),
            bytesDownloaded: Int64(clamping: value.bytesDownloaded),
            bytesUploaded: Int64(clamping: value.bytesUploaded),
            downloadRate: Int64(clamping: value.downloadRate),
            uploadRate: Int64(clamping: value.uploadRate),
            peersCount: Int(value.peers),
            seedsCount: Int(value.seeds),
            seedRatio: value.seedRatio,
            seedRatioLimit: value.seedRatioLimit,
            etaSeconds: value.etaSeconds.map(Int.init),
            pieceLength: Int64(clamping: value.pieceLength),
            pieceCount: Int(value.pieceCount),
            comment: value.comment,
            trackers: value.trackers,
            files: files,
            isVerifying: value.state == .verifying,
            verificationProgress: Double(value.progressMillionths) / 1_000_000,
            progress: Double(value.progressMillionths) / 1_000_000,
            outputPath: value.outputPath,
            contentFolder: value.contentFolder,
            thumbnailURL: value.thumbnailUrl,
            extractor: value.extractor,
            errorMessage: value.errorMessage,
            playlistIndex: value.playlistIndex.map(Int.init),
            capabilities: value.capabilities.presentation
        )
    }

    private func processPlatformEffects() {
        guard let machine else { return }
        for effect in machine.platformEffects()
        where !platformEffectsInProgress.contains(effect.effectId)
            && !rejectedPlatformEffects.contains(effect.effectId) {
            let present = effect.targets
                .map { ($0, URL(fileURLWithPath: $0.path).standardizedFileURL) }
                .filter { FileManager.default.fileExists(atPath: $0.1.path) }
            let urls = present.filter { fileIdentityMatches($0.0, at: $0.1) }.map(\.1)
            let changed = present.filter { !urls.contains($0.1) }.map(\.1)
            if !changed.isEmpty {
                rejectedPlatformEffects.insert(effect.effectId)
                recoveryNotice = changed.count == 1
                    ? "A downloaded file changed after removal was requested, so Opul left it in place: \(changed[0].lastPathComponent)."
                    : "\(changed.count) downloaded files changed after removal was requested, so Opul left them in place."
            }
            guard !urls.isEmpty else {
                // The snapshot refresh that follows picks up the finished removal; consuming
                // here would re-enter this loop while it still walks the same effects.
                completePlatformEffect(effect.effectId, succeeded: changed.isEmpty, refresh: false)
                continue
            }
            platformEffectsInProgress.insert(effect.effectId)
            NSWorkspace.shared.recycle(urls) { [weak self] _, error in
                Task { @MainActor in
                    guard let self else { return }
                    self.platformEffectsInProgress.remove(effect.effectId)
                    if let error {
                        self.rejectedPlatformEffects.insert(effect.effectId)
                        self.recoveryNotice = "A file could not be moved to the Trash: \(error.localizedDescription)"
                    }
                    self.completePlatformEffect(effect.effectId, succeeded: error == nil && changed.isEmpty)
                }
            }
        }
    }

    private func completePlatformEffect(_ effectID: String, succeeded: Bool, refresh: Bool = true) {
        guard let machine else { return }
        do {
            try machine.completePlatformEffect(effectIdValue: effectID, succeeded: succeeded)
            if refresh { consume(machine.snapshot()) }
        } catch {
            engineNotice = error.localizedDescription
        }
    }

    /// The file must still be the one Opul recorded. Its device and inode prove that on their
    /// own, and survive Finder adding a `.DS_Store` or an app editing the file in place; size
    /// and modification time only stand in when the inode was never recorded.
    private func fileIdentityMatches(_ target: MachineTrashTarget, at url: URL) -> Bool {
        var value = stat()
        guard lstat(url.path, &value) == 0 else { return false }
        if let expected = target.expectedFileIdentifier {
            return "\(value.st_dev):\(value.st_ino)" == expected
        }
        if let expected = target.expectedByteLength, UInt64(value.st_size) != expected {
            return false
        }
        if let expected = target.expectedModifiedAtMillis {
            let actual = UInt64(value.st_mtimespec.tv_sec) * 1_000
                + UInt64(value.st_mtimespec.tv_nsec) / 1_000_000
            if actual != expected { return false }
        }
        return true
    }

    private func machineConfig() -> OpulMachineConfig {
        OpulMachineConfig(
            applicationSupportFolder: Self.applicationSupportFolder.path(percentEncoded: false),
            downloadsFolder: MacFolder.path(for: .downloadsDirectory),
            torrent: EngineConfig(
                sessionFolder: Self.applicationSupportFolder
                    .appending(path: "torrent-session", directoryHint: .isDirectory)
                    .path(percentEncoded: false),
                listenPort: UInt16(clamping: preferences.listenPort),
                randomizePort: preferences.randomizePort,
                enableDht: preferences.enableDHT,
                maxConnections: UInt32(clamping: max(preferences.maxConnections, 0)),
                maxDownloadBps: bps(preferences.maxDownloadRateKBps),
                maxUploadBps: bps(preferences.maxUploadRateKBps)
            ),
            media: MediaEngineConfig(
                sessionFolder: Self.applicationSupportFolder
                    .appending(path: "media-session", directoryHint: .isDirectory)
                    .path(percentEncoded: false),
                resourcesFolder: Bundle.main.resourceURL?.path(percentEncoded: false),
                ytDlpPath: preferences.ytDlpPath.nilIfBlank,
                ffmpegPath: preferences.ffmpegPath.nilIfBlank,
                maxConcurrentDownloads: UInt32(clamping: max(preferences.maxConcurrentMediaDownloads, 1)),
                maxDownloadBps: bps(preferences.maxDownloadRateKBps)
            )
        )
    }

    private func applyLimits() {
        machine?.setSpeedLimits(
            downloadBps: bps(preferences.maxDownloadRateKBps),
            uploadBps: bps(preferences.maxUploadRateKBps)
        )
        machine?.setMaxConcurrentMediaDownloads(
            maximum: UInt32(clamping: max(preferences.maxConcurrentMediaDownloads, 1))
        )
    }

    private func mediaPolicy(userAgent: String?, direct: Bool) -> MachineMediaPolicy {
        MachineMediaPolicy(
            downloadSubtitles: direct ? false : preferences.downloadSubtitles,
            subtitleLanguages: direct ? [] : preferences.subtitleLanguages
                .split(separator: ",")
                .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                .filter { !$0.isEmpty },
            embedSubtitles: direct ? false : preferences.embedSubtitles,
            writeThumbnail: direct ? false : preferences.writeThumbnail,
            writeMetadata: direct ? false : preferences.writeMetadata,
            filenameTemplate: preferences.filenameTemplate,
            userAgent: userAgent
        )
    }

    private func folderRequest(
        workspaceID: Workspace.ID,
        folderID: SavedFolder.ID,
        name: String,
        path: String
    ) -> MachineFolderRequest {
        MachineFolderRequest(
            requestId: UUID().uuidString,
            expectedRevision: nil,
            workspaceId: workspaceID.uuidString,
            folderId: folderID.uuidString,
            name: name,
            path: path
        )
    }

    private func downloadCommand(_ id: Download.ID) -> MachineDownloadCommand {
        MachineDownloadCommand(
            requestId: UUID().uuidString,
            expectedRevision: nil,
            downloadId: id.uuidString
        )
    }

    private func freshWorkspaceRequest() -> MachineWorkspaceRequest {
        let workspaceID = UUID()
        let folderID = UUID()
        let path = MacFolder.path(for: .downloadsDirectory)
        return MachineWorkspaceRequest(
            requestId: UUID().uuidString,
            expectedRevision: nil,
            workspaceId: workspaceID.uuidString,
            name: "Downloads",
            icon: MachineWorkspaceIcon(kind: .symbol, value: "arrow.down.circle"),
            folderId: folderID.uuidString,
            folderName: "Downloads",
            folderPath: path,
            defaultFormat: .best,
            fileOrganization: preferences.fileOrganization.machine
        )
    }

    private func defaultSeedLimit() -> Double? {
        preferences.autoStopSeeding && preferences.defaultSeedRatio > 0
            ? preferences.defaultSeedRatio
            : nil
    }

    private func bps(_ kilobytes: Int) -> UInt64 {
        kilobytes > 0 ? UInt64(kilobytes) * 1_024 : 0
    }

    private func folder(for id: SavedFolder.ID) -> SavedFolder? {
        workspaces.lazy.flatMap(\.savedFolders).first { $0.id == id }
    }

    private func folderIDForPaste(in workspace: Workspace) -> SavedFolder.ID? {
        if let folderID = selection?.folderID,
           selection?.workspaceID == workspace.id,
           workspace.savedFolders.contains(where: { $0.id == folderID }) {
            return folderID
        }
        return workspace.savedFolders.first(where: \.isDefault)?.id
            ?? workspace.savedFolders.first?.id
    }

    private func contains(_ selection: SidebarSelection) -> Bool {
        guard let workspace = workspaces.first(where: { $0.id == selection.workspaceID }) else {
            return false
        }
        guard let folderID = selection.folderID else { return true }
        return workspace.savedFolders.contains { $0.id == folderID }
    }

    private func kind(forLargest files: [TorrentFileItem]) -> Kind? {
        guard let largest = files.max(by: { $0.size < $1.size }) else { return nil }
        return kind(forExtension: URL(fileURLWithPath: largest.name).pathExtension.lowercased())
    }

    private func kind(for address: String, format: Format) -> Kind {
        if format.isAudioOnly { return .audio }
        return kind(forExtension: URL(string: address)?.pathExtension.lowercased() ?? "")
    }

    private func kind(forExtension ext: String) -> Kind {
        switch ext {
        case "mp4", "mov", "mkv", "webm", "m4v": .video
        case "mp3", "m4a", "aac", "wav", "flac", "aiff": .audio
        default: .file
        }
    }

    private func isTorrentFile(_ address: String) -> Bool {
        (URL(string: address)?.path.lowercased() ?? address.lowercased()).hasSuffix(".torrent")
    }

    private func displayName(forFolderPath path: String, fallback: String) -> String {
        if path == MacFolder.path(for: .downloadsDirectory) { return "Downloads" }
        let name = URL(fileURLWithPath: path).lastPathComponent
        return name.isEmpty ? fallback : name
    }

    private func warn(_ message: String) {
        pasteNotice = PasteNotice(text: message, isWarning: true)
        soundEngine.playRandom(.warning, preferences: preferences.interactionSounds)
    }

    private func playDownloadAddedSound() {
        soundEngine.playRandom(.tap, preferences: preferences.interactionSounds)
    }

    private func playSoundForTransition(from previousState: DownloadState?, to state: DownloadState) {
        guard canPlayTransferSounds,
              let category = InteractionSoundCategory.forDownloadTransition(
                from: previousState,
                to: state
              ) else { return }
        soundEngine.playRandom(category, preferences: preferences.interactionSounds)
    }

    private func persistPreferences() {
        guard let data = try? JSONEncoder().encode(preferences) else { return }
        UserDefaults.standard.set(data, forKey: Self.preferencesKey)
    }

    private static func loadPreferences() -> TorrentPreferences {
        guard let data = UserDefaults.standard.data(forKey: preferencesKey),
              let stored = try? JSONDecoder().decode(TorrentPreferences.self, from: data) else {
            return TorrentPreferences()
        }
        return stored
    }

    private static let preferencesKey = "opul.torrent-preferences"

    static var applicationSupportFolder: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
        return base.appending(path: "Opul", directoryHint: .isDirectory)
    }

    static var libraryURL: URL {
        applicationSupportFolder.appending(path: "library.json", directoryHint: .notDirectory)
    }

    static var libraryBackupURL: URL {
        applicationSupportFolder.appending(path: "library.json.bak", directoryHint: .notDirectory)
    }
}

private extension WorkspaceIcon {
    var machine: MachineWorkspaceIcon {
        switch self {
        case .symbol(let value): MachineWorkspaceIcon(kind: .symbol, value: value)
        case .emoji(let value): MachineWorkspaceIcon(kind: .emoji, value: value)
        }
    }
}

private extension MachineWorkspaceIcon {
    var presentation: WorkspaceIcon {
        switch kind {
        case .symbol: .symbol(value)
        case .emoji: .emoji(value)
        }
    }
}

private extension Format {
    var machine: MachineFormat {
        switch self {
        case .best: .best
        case .bestMp4: .bestMp4
        case .p1080: .p1080
        case .p720: .p720
        case .original: .original
        case .audioMp3: .audioMp3
        case .audioM4a: .audioM4a
        }
    }
}

private extension MachineFormat {
    var presentation: Format {
        switch self {
        case .best: .best
        case .bestMp4: .bestMp4
        case .p1080: .p1080
        case .p720: .p720
        case .original: .original
        case .audioMp3: .audioMp3
        case .audioM4a: .audioM4a
        }
    }
}

private extension DownloadFileOrganization {
    var machine: MachineFileOrganization {
        switch self {
        case .automatic: .automatic
        case .alwaysCreateFolder: .alwaysCreateFolder
        case .keepInSaveLocation: .keepInSaveLocation
        }
    }
}

private extension MachineFileOrganization {
    var presentation: DownloadFileOrganization {
        switch self {
        case .automatic: .automatic
        case .alwaysCreateFolder: .alwaysCreateFolder
        case .keepInSaveLocation: .keepInSaveLocation
        }
    }
}

private extension DownloadLayout {
    var machine: MachineLayout {
        switch self {
        case .list: .list
        case .table: .table
        }
    }
}

private extension MachineLayout {
    var presentation: DownloadLayout {
        switch self {
        case .list: .list
        case .table: .table
        }
    }
}

private extension DownloadSort {
    var machine: MachineSort {
        switch self {
        case .order: .order
        case .name: .name
        case .state: .state
        case .source: .source
        }
    }
}

private extension MachineSort {
    var presentation: DownloadSort {
        switch self {
        case .order: .order
        case .name: .name
        case .state: .state
        case .source: .source
        }
    }
}

private extension DownloadGrouping {
    var machine: MachineGrouping {
        switch self {
        case .none: .none
        case .state: .state
        case .folder: .folder
        case .kind: .kind
        case .source: .source
        }
    }
}

private extension MachineGrouping {
    var presentation: DownloadGrouping {
        switch self {
        case .none: .none
        case .state: .state
        case .folder: .folder
        case .kind: .kind
        case .source: .source
        }
    }
}

private extension MachineDownloadState {
    var presentation: DownloadState {
        switch self {
        case .queued, .starting: .queued
        case .fetching: .fetching
        case .processing, .verifying: .processing
        case .seeding: .seeding
        case .pausing, .paused: .paused
        case .finished: .finished
        case .failed: .failed
        case .cancelled: .cancelled
        }
    }
}

private extension MachineSource {
    var presentation: Source {
        switch self {
        case .torrent: .torrent
        case .videoPage: .videoPage
        case .direct: .fileURL
        case .guided: .guided
        }
    }
}

private extension MachineCapabilities {
    var presentation: DownloadCapabilities {
        DownloadCapabilities(
            canPause: canPause,
            canResume: canResume,
            canRetry: canRetry,
            canCancel: canCancel,
            canStopSeeding: canStopSeeding,
            canChangeFileSelection: canChangeFileSelection,
            canRemove: canRemove,
            canRevealOutput: canRevealOutput,
            canMove: canMove
        )
    }
}

enum MacFolder {
    static func path(
        for directory: FileManager.SearchPathDirectory,
        appending component: String? = nil
    ) -> String {
        let base = FileManager.default.urls(for: directory, in: .userDomainMask).first
            ?? URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
        guard let component else { return base.path(percentEncoded: false) }
        let cleaned = component.replacingOccurrences(of: "/", with: "-")
        return base.appending(path: cleaned).path(percentEncoded: false)
    }
}

private extension String {
    var nilIfBlank: String? {
        let value = trimmingCharacters(in: .whitespacesAndNewlines)
        return value.isEmpty ? nil : value
    }
}
