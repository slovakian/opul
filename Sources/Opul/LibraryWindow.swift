import AppKit
import QuickLook
import SwiftUI
import UniformTypeIdentifiers

struct LibraryWindow: View {
    @Bindable var library: DownloadLibrary
    @State private var showPreferences = false
    @State private var isChoosingTorrent = false

    var body: some View {
        NavigationSplitView {
            WorkspaceSidebar(library: library)
        } detail: {
            WorkspaceArea(
                library: library,
                onOpenTorrent: { isChoosingTorrent = true },
                onOpenPreferences: { showPreferences = true }
            )
        }
        .navigationTitle("Opul")
        .sheet(item: $library.sheet) { sheet in
            switch sheet {
            case .guided:
                GuidedExtractionSheet(library: library)
            case .create, .settings:
                CreateWorkspaceSheet(library: library, sheet: sheet)
            }
        }
        .sheet(isPresented: $showPreferences) {
            PreferencesSheet(library: library)
        }
        .sheet(isPresented: $library.isShowingWelcome, onDismiss: library.welcomeDismissed) {
            WelcomeSheet(library: library)
        }
        .alert(
            videoToolsPromptTitle,
            isPresented: videoToolsPromptPresented,
            presenting: library.videoToolsPrompt
        ) { prompt in
            Button(videoToolsDownloadTitle(prompt)) {
                library.resolveVideoToolsPrompt(download: true)
            }
            .keyboardShortcut(.defaultAction)
            Button("Cancel", role: .cancel) {
                library.resolveVideoToolsPrompt(download: false)
            }
        } message: { prompt in
            if prompt.brokenCustomPath != nil {
                Text("Opul won't switch tools on its own. Download Opul's copy of yt-dlp to use it from now on, and this link starts when it's ready.")
            } else {
                Text("yt-dlp is a free tool that reads video pages. Opul downloads it from GitHub, and this link starts when it's ready.")
            }
        }
        .environment(\.videoTools, videoToolsContext)
        .sheet(item: $library.pendingMediaImport) { request in
            MediaImportSheet(library: library, request: request)
        }
        .sheet(item: inspectorBinding) { request in
            TorrentInspectorSheet(library: library, downloadID: request.downloadID)
        }
        .quickLookPreview($library.quickLookURL, in: library.quickLookURLs)
        .fileImporter(
            isPresented: $library.isChoosingFolder,
            allowedContentTypes: [.folder],
            allowsMultipleSelection: false,
            onCompletion: library.useChosenFolder
        )
        .fileImporter(
            isPresented: $isChoosingTorrent,
            allowedContentTypes: [UTType(filenameExtension: "torrent") ?? .data],
            allowsMultipleSelection: false
        ) { result in
            if case .success(let urls) = result, let url = urls.first {
                library.addTorrentFile(at: url)
            }
        }
        .confirmationDialog(
            library.pendingDelete?.title ?? "Delete Workspace?",
            isPresented: deletePresented,
            titleVisibility: .visible
        ) {
            if library.pendingDelete?.hasDownloads == true {
                Button("Delete Workspace and Downloads", role: .destructive) {
                    library.deletePending(moveFilesToTrash: false)
                }
                Button("Delete and Move Files to Trash", role: .destructive) {
                    library.deletePending(moveFilesToTrash: true)
                }
            } else {
                Button("Delete", role: .destructive) {
                    library.deletePending(moveFilesToTrash: false)
                }
            }
            Button("Cancel", role: .cancel) {
                library.pendingDelete = nil
            }
        } message: {
            Text(library.pendingDelete?.message ?? "")
        }
        .confirmationDialog(
            folderRemovalTitle,
            isPresented: folderRemovalPresented,
            titleVisibility: .visible
        ) {
            Button("Remove Folder and Downloads", role: .destructive) {
                library.removePendingFolder(moveFilesToTrash: false)
            }
            Button("Remove and Move Files to Trash", role: .destructive) {
                library.removePendingFolder(moveFilesToTrash: true)
            }
            Button("Cancel", role: .cancel) {
                library.pendingFolderRemoval = nil
            }
        } message: {
            Text(folderRemovalMessage)
        }
        .confirmationDialog(
            folderReplacementTitle,
            isPresented: folderReplacementPresented,
            titleVisibility: .visible
        ) {
            if library.pendingFolderReplacement?.canRelink == true {
                Button("Relink to Chosen Folder") {
                    library.resolvePendingFolderReplacement(relink: true)
                }
            }
            Button("Add as New Default Folder") {
                library.resolvePendingFolderReplacement(relink: false)
            }
            Button("Cancel", role: .cancel) {
                library.pendingFolderReplacement = nil
            }
        } message: {
            Text(folderReplacementMessage)
        }
        .background {
            WorkspacePasteMonitor(isEnabled: library.selectedWorkspace != nil) {
                library.pasteClipboard()
            }
        }
        .background(WindowChromeConfigurator())
        .modifier(SoftTopScrollEdgeEffect())
        .onOpenURL { url in
            library.open(url)
        }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.willTerminateNotification)) { _ in
            library.shutdown()
        }
        .frame(minWidth: 860, minHeight: 520)
    }

    private var videoToolsContext: VideoToolsContext {
        let status = library.mediaToolStatus
        var hasBrokenCustomYtDlp = false
        if case .customBroken = status?.ytDlp {
            hasBrokenCustomYtDlp = true
        }
        return VideoToolsContext(
            isGettingTools: status?.ytDlpPending == true,
            hasBrokenCustomYtDlp: hasBrokenCustomYtDlp,
            downloadOpulsCopy: { [library] id in
                library.replaceCustomYtDlpWithOpuls(retrying: id)
            }
        )
    }

    private var videoToolsPromptTitle: String {
        guard let path = library.videoToolsPrompt?.brokenCustomPath else {
            return "Video downloads need yt-dlp"
        }
        return "Your yt-dlp at \(path) isn't working"
    }

    private func videoToolsDownloadTitle(_ prompt: VideoToolsPrompt) -> String {
        let size = library.mediaToolStatus?.downloadSizeText.map { " (\($0))" } ?? ""
        return prompt.brokenCustomPath == nil ? "Download\(size)" : "Download Opul's copy\(size)"
    }

    private var videoToolsPromptPresented: Binding<Bool> {
        Binding(
            get: { library.videoToolsPrompt != nil },
            set: { isPresented in
                if !isPresented {
                    library.videoToolsPrompt = nil
                }
            }
        )
    }

    private var deletePresented: Binding<Bool> {
        Binding(
            get: { library.pendingDelete != nil },
            set: { isPresented in
                if !isPresented {
                    library.pendingDelete = nil
                }
            }
        )
    }

    private var folderRemovalPresented: Binding<Bool> {
        Binding(
            get: { library.pendingFolderRemoval != nil },
            set: { isPresented in
                if !isPresented {
                    library.pendingFolderRemoval = nil
                }
            }
        )
    }

    private var folderRemovalTitle: String {
        guard let request = library.pendingFolderRemoval else { return "Remove Saved Folder?" }
        return "Remove \(request.folderName)?"
    }

    private var folderRemovalMessage: String {
        guard let request = library.pendingFolderRemoval else { return "" }
        let count = request.downloadIDs.count
        let downloads = count == 1 ? "1 download uses" : "\(count) downloads use"
        return "\(downloads) this saved folder. Removing it stops active transfers and removes those downloads from Opul. Files stay where they are unless you choose to move them, including partially downloaded files, to the Trash."
    }

    private var folderReplacementPresented: Binding<Bool> {
        Binding(
            get: { library.pendingFolderReplacement != nil },
            set: { isPresented in
                if !isPresented {
                    library.pendingFolderReplacement = nil
                }
            }
        )
    }

    private var folderReplacementTitle: String {
        guard let request = library.pendingFolderReplacement else { return "Change Folder Location?" }
        return "Change \(request.folderName) Location?"
    }

    private var folderReplacementMessage: String {
        guard let request = library.pendingFolderReplacement else { return "" }
        let downloads = request.downloadCount == 1 ? "1 download uses" : "\(request.downloadCount) downloads use"
        if request.canRelink {
            return "\(downloads) this saved folder. Relink only if you already moved its files. Otherwise, add the chosen folder as the new default and keep this location for existing downloads."
        }
        return "\(downloads) this saved folder and may still depend on its location. Add the chosen folder as the new default. This location must remain until its downloads are removed."
    }

    private var inspectorBinding: Binding<InspectorRequest?> {
        Binding(
            get: { library.inspectorDownloadID.map(InspectorRequest.init(downloadID:)) },
            set: { library.inspectorDownloadID = $0?.downloadID }
        )
    }
}

private struct WindowChromeConfigurator: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        WindowChromeView()
    }

    func updateNSView(_ nsView: NSView, context: Context) {}
}

private final class WindowChromeView: NSView {
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        configureWindowChrome()
        DispatchQueue.main.async { [weak self] in
            self?.configureWindowChrome()
        }
    }

    override func layout() {
        super.layout()
        configureWindowChrome()
    }

    private func configureWindowChrome() {
        guard let window else { return }
        if !window.styleMask.contains(.fullSizeContentView) {
            window.styleMask.insert(.fullSizeContentView)
        }
        if !window.titlebarAppearsTransparent {
            window.titlebarAppearsTransparent = true
        }
        if window.titlebarSeparatorStyle != .none {
            window.titlebarSeparatorStyle = .none
        }
    }
}

struct SoftTopScrollEdgeEffect: ViewModifier {
    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(macOS 26, *) {
            content.scrollEdgeEffectStyle(.soft, for: .top)
        } else {
            content
        }
    }
}

private struct InspectorRequest: Identifiable {
    let downloadID: Download.ID
    var id: Download.ID { downloadID }
}
