import OpulClipboard
import SwiftUI

struct WorkspaceArea: View {
    @Bindable var library: DownloadLibrary
    let onOpenTorrent: () -> Void
    let onOpenPreferences: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            if let recoveryNotice = library.recoveryNotice {
                RecoveryNoticeBanner(library: library, recoveryNotice: recoveryNotice)
            }
            if let workspace = library.selectedWorkspace {
                WorkspaceDetail(
                    library: library,
                    workspace: workspace,
                    onOpenTorrent: onOpenTorrent,
                    onOpenPreferences: onOpenPreferences
                )
            } else {
                ContentUnavailableView(
                    "No Workspace",
                    systemImage: "square.stack.3d.up",
                    description: Text("Create a Workspace to keep a pile of Downloads.")
                )
                .navigationTitle("Opul")
            }
        }
    }
}

private struct RecoveryNoticeBanner: View {
    @Bindable var library: DownloadLibrary
    let recoveryNotice: String

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(Color.orange)
                .accessibilityHidden(true)
            Text(recoveryNotice)
                .font(.callout)
                .foregroundStyle(Color.primary)
                .accessibilityLabel("Recovery notice: \(recoveryNotice)")
            Spacer()
            if library.machineSession == .recoveryRequired {
                let buttonTitle = recoveryNotice.contains("recovered from an older backup") ? "Resume from Backup" : "Start Fresh"
                Button(buttonTitle) {
                    library.acknowledgeRecovery()
                }
            }
            Button {
                library.recoveryNotice = nil
            } label: {
                Image(systemName: "xmark")
                    .font(.caption)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Dismiss recovery notice")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .background(Color.orange.opacity(0.12))
        .accessibilityIdentifier("recovery-notice")
    }
}

private struct WorkspaceDetail: View {
    @Bindable var library: DownloadLibrary
    let workspace: Workspace
    let onOpenTorrent: () -> Void
    let onOpenPreferences: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let notice = library.pasteNotice {
                Text(notice.text)
                    .font(.callout)
                    .foregroundStyle(notice.isWarning ? Color.red : Color.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    .accessibilityIdentifier("paste-notice")
            }
            DownloadBrowser(
                library: library,
                downloads: filteredDownloads,
                workspaceID: workspace.id,
                folders: workspace.savedFolders,
                settings: workspace.view,
                isFolderFilter: workspace.savedFolders.count > 1 && library.selection?.folderID != nil,
                isSearching: !searchQuery.isEmpty,
                onInspect: { downloadID in
                    library.inspectorDownloadID = downloadID
                }
            )
        }
        .navigationTitle("")
        .toolbar { workspaceToolbar }
        .confirmationDialog(
            removalTitle,
            isPresented: removalPresented,
            titleVisibility: .visible
        ) {
            Button(role: .destructive) {
                library.removePendingDownloads(moveFilesToTrash: true)
            } label: {
                Label("Remove and Move Files to Trash", systemImage: "trash")
            }
            Button(role: .destructive) {
                library.removePendingDownloads(moveFilesToTrash: false)
            } label: {
                Label("Remove from List", systemImage: "minus.circle")
            }
            Button("Cancel", role: .cancel) {
                library.pendingDownloadRemoval = nil
            }
        } message: {
            Text(removalMessage)
        }
    }

    private var searchQuery: String {
        library.searchText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var filteredDownloads: [Download] {
        library.displayedDownloads(in: workspace)
    }

    private func handleSearchOrPasteSubmit(_ text: String) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        if isDownloadableAddress(trimmed) {
            library.paste(address: trimmed)
            library.searchText = ""
        }
    }

    @ToolbarContentBuilder
    private var workspaceToolbar: some ToolbarContent {
        ToolbarItem(placement: .principal) {
            HStack(spacing: 12) {
                if let defaultFolder {
                    UnifiedSearchField(
                        text: $library.searchText,
                        isLoading: library.isExtracting,
                        onSubmit: handleSearchOrPasteSubmit
                    ) {
                        DefaultFolderMenu(
                            library: library,
                            workspace: workspace,
                            defaultFolder: defaultFolder
                        )
                    }
                }

                AppActionCluster(
                    onGuidedExtraction: { library.sheet = .guided },
                    onOpenTorrent: onOpenTorrent,
                    onOpenPreferences: onOpenPreferences
                )
                .fixedSize()
            }
        }
        .separatedGlass()
        ToolbarItem(placement: .primaryAction) {
            controlCluster
        }
        .separatedGlass()
    }

    private var defaultFolder: SavedFolder? {
        workspace.savedFolders.first(where: \.isDefault) ?? workspace.savedFolders.first
    }

    private var removalPresented: Binding<Bool> {
        Binding(
            get: { library.pendingDownloadRemoval != nil },
            set: { isPresented in
                if !isPresented {
                    library.pendingDownloadRemoval = nil
                }
            }
        )
    }

    private var removalTitle: String {
        guard let request = library.pendingDownloadRemoval else { return "Remove Download?" }
        return request.downloadIDs.count == 1 ? "Remove Download?" : "Remove \(request.downloadIDs.count) Downloads?"
    }

    private var removalMessage: String {
        "Removing stops any transfer in progress. Remove from List keeps files on disk; the other option moves them to the Trash, including partially downloaded files."
    }

    @ViewBuilder
    private var controlCluster: some View {
        if #available(macOS 26, *) {
            GlassEffectContainer(spacing: 12) {
                HStack(spacing: 12) {
                    layoutMenu
                    groupMenu
                    sortMenu
                    actionsMenu
                }
            }
        } else {
            HStack(spacing: 12) {
                layoutMenu
                groupMenu
                sortMenu
                actionsMenu
            }
        }
    }

    private var layoutMenu: some View {
        CircleIconMenu(title: "Layout", systemImage: workspace.view.layout.symbolName) {
            Picker("Layout", selection: layoutBinding) {
                ForEach(DownloadLayout.allCases, id: \.self) { layout in
                    Label(layout.title, systemImage: layout.symbolName).tag(layout)
                }
            }
            .pickerStyle(.inline)
        }
    }

    private var groupMenu: some View {
        CircleIconMenu(
            title: "Group",
            systemImage: "square.grid.3x1.below.line.grid.1x2"
        ) {
            Picker("Group", selection: groupingBinding) {
                ForEach(DownloadGrouping.allCases, id: \.self) { grouping in
                    Label(grouping.title, systemImage: grouping.symbolName).tag(grouping)
                }
            }
            .pickerStyle(.inline)
        }
    }

    private var sortMenu: some View {
        CircleIconMenu(title: "Sort", systemImage: "arrow.up.arrow.down") {
            Picker("Sort", selection: sortBinding) {
                ForEach(DownloadSort.allCases, id: \.self) { sort in
                    Label(sort.title, systemImage: sort.symbolName).tag(sort)
                }
            }
            .pickerStyle(.inline)
        }
    }

    private var actionsMenu: some View {
        CircleIconMenu(title: "Workspace actions", systemImage: "ellipsis") {
            Menu("Default format") {
                Picker("Default format", selection: formatBinding) {
                    ForEach(Format.allCases) { format in
                        Text(format.title).tag(format)
                    }
                }
                .pickerStyle(.inline)
            }
            Button("Workspace Settings…") {
                library.sheet = .settings(workspace.id)
            }
            Button("Add Saved Folder…") {
                library.beginAddFolder(in: workspace.id)
            }
            Button("Clear Finished") {
                library.clearFinished(in: workspace.id)
            }
            .disabled(!workspace.downloads.contains { $0.state == .finished || $0.state == .cancelled })
            Divider()
            Button("Delete…", role: .destructive) {
                library.askToDelete(workspace.id)
            }
        }
    }

    private var layoutBinding: Binding<DownloadLayout> {
        Binding(
            get: { workspace.view.layout },
            set: { layout in
                var view = workspace.view
                view.layout = layout
                library.setView(view, on: workspace.id)
            }
        )
    }

    private var groupingBinding: Binding<DownloadGrouping> {
        Binding(
            get: { workspace.view.grouping },
            set: { grouping in
                var view = workspace.view
                view.grouping = grouping
                library.setView(view, on: workspace.id)
            }
        )
    }

    private var sortBinding: Binding<DownloadSort> {
        Binding(
            get: { workspace.view.sort },
            set: { sort in
                var view = workspace.view
                view.sort = sort
                library.setView(view, on: workspace.id)
            }
        )
    }

    private var formatBinding: Binding<Format> {
        Binding(
            get: { workspace.defaultFormat },
            set: { library.setDefaultFormat($0, on: workspace.id) }
        )
    }
}

private struct DefaultFolderMenu: View {
    let library: DownloadLibrary
    let workspace: Workspace
    let defaultFolder: SavedFolder

    var body: some View {
        Menu {
            Picker(
                "Default Folder",
                selection: Binding(
                    get: {
                        workspace.savedFolders.first(where: \.isDefault)?.id ?? defaultFolder.id
                    },
                    set: { library.makeDefault($0, in: workspace.id) }
                )
            ) {
                ForEach(workspace.savedFolders) { folder in
                    Label(folder.name, systemImage: "folder")
                        .tag(folder.id)
                }
            }
            .pickerStyle(.inline)
            Divider()
            Button("Choose Another Folder…", systemImage: "folder.badge.plus") {
                library.beginChooseDefaultFolder(in: workspace.id)
            }
        } label: {
            HStack(spacing: 5) {
                Image(systemName: "folder")
                Text(defaultFolder.name)
                    .lineLimit(1)
                    .frame(maxWidth: 140)
            }
            .contentShape(Rectangle())
        }
        .menuStyle(.button)
        .buttonStyle(.borderless)
        .help(defaultFolder.path)
        .accessibilityLabel("Default folder, \(defaultFolder.name)")
    }
}

private extension ToolbarContent {
    @ToolbarContentBuilder
    func separatedGlass() -> some ToolbarContent {
        if #available(macOS 26, *) {
            sharedBackgroundVisibility(.hidden)
        } else {
            self
        }
    }
}

private struct UnifiedSearchField<FolderPicker: View>: View {
    @Binding var text: String
    let isLoading: Bool
    let onSubmit: (String) -> Void
    @ViewBuilder let folderPicker: FolderPicker
    @FocusState private var isFocused: Bool

    private var trimmed: String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var isLink: Bool {
        isDownloadableAddress(trimmed)
    }

    var body: some View {
        ZStack {
            Button {
                isFocused = true
            } label: {
                Capsule()
                    .fill(.clear)
                    .contentShape(Capsule())
            }
            .buttonStyle(.plain)
            .accessibilityHidden(true)

            HStack(spacing: 8) {
                if isLoading {
                    ProgressView()
                        .controlSize(.small)
                        .accessibilityLabel("Inspecting link")
                        .allowsHitTesting(false)
                } else if isLink {
                    Image(systemName: "link")
                        .foregroundStyle(.tint)
                        .accessibilityHidden(true)
                        .allowsHitTesting(false)
                } else {
                    Image(systemName: "magnifyingglass")
                        .foregroundStyle(.secondary)
                        .accessibilityHidden(true)
                        .allowsHitTesting(false)
                }

                TextField("Search downloads or paste link…", text: $text)
                    .textFieldStyle(.plain)
                    .focused($isFocused)
                    .autocorrectionDisabled()
                    .frame(maxWidth: .infinity)
                    .onSubmit {
                        onSubmit(text)
                    }

                if !text.isEmpty {
                    Button {
                        text = ""
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.secondary)
                            .contentShape(Circle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Clear text")
                }

                Divider()
                    .frame(height: 16)

                folderPicker

                if isLink {
                    Button {
                        onSubmit(text)
                    } label: {
                        Image(systemName: "arrow.right.circle.fill")
                            .foregroundStyle(.tint)
                            .contentShape(Circle())
                    }
                    .buttonStyle(.plain)
                    .help("Download link")
                    .accessibilityLabel("Download link")
                }
            }
            .padding(.horizontal, 12)
        }
        .frame(minWidth: 160, idealWidth: 480, maxWidth: .infinity)
        .frame(height: 32)
        .modifier(GlassSearchField(isFocused: isFocused))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Search downloads or paste link")
    }
}

private struct AppActionCluster: View {
    let onGuidedExtraction: () -> Void
    let onOpenTorrent: () -> Void
    let onOpenPreferences: () -> Void

    var body: some View {
        HStack(spacing: 0) {
            ToolbarActionButton(
                title: "Guided Extraction",
                systemImage: "safari",
                help: "Open a browser that detects media as pages load",
                action: onGuidedExtraction
            )
            ToolbarActionButton(
                title: "Open Torrent",
                systemImage: "plus.circle",
                help: "Open a .torrent file",
                action: onOpenTorrent
            )
            ToolbarActionButton(
                title: "Preferences",
                systemImage: "gearshape",
                help: "Open Preferences",
                action: onOpenPreferences
            )
        }
        .padding(.horizontal, 2)
        .frame(height: 32)
        .modifier(GlassCapsuleFill())
    }
}

private struct ToolbarActionButton: View {
    let title: String
    let systemImage: String
    let help: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .frame(width: 28, height: 28)
                .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .help(help)
        .accessibilityLabel(title)
    }
}

private struct GlassCapsuleFill: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 26, *) {
            content.glassEffect(.regular.interactive(), in: .capsule)
        } else {
            content
                .background(.ultraThinMaterial, in: Capsule())
                .overlay {
                    Capsule()
                        .strokeBorder(Color.primary.opacity(0.1), lineWidth: 1)
                }
        }
    }
}

private struct GlassSearchField: ViewModifier {
    let isFocused: Bool

    func body(content: Content) -> some View {
        if #available(macOS 26, *) {
            content
                .glassEffect(.regular.interactive(), in: .capsule)
                .overlay {
                    Capsule()
                        .strokeBorder(isFocused ? Color.accentColor.opacity(0.55) : .clear, lineWidth: 1)
                }
        } else {
            content
                .background(.ultraThinMaterial, in: Capsule())
                .overlay {
                    Capsule()
                        .strokeBorder(
                            isFocused ? Color.accentColor.opacity(0.55) : Color.primary.opacity(0.1),
                            lineWidth: 1
                        )
                }
        }
    }
}

private struct CircleIconMenu<MenuContent: View>: View {
    let title: String
    let systemImage: String
    @ViewBuilder var menu: () -> MenuContent

    var body: some View {
        Menu {
            menu()
        } label: {
            ZStack {
                Circle()
                    .fill(.ultraThinMaterial)
                Image(systemName: systemImage)
                    .font(.system(size: 13, weight: .semibold))
            }
            .frame(width: 32, height: 32)
            .modifier(GlassCircleFill())
            .contentShape(Circle())
        }
        .menuStyle(.button)
        .buttonStyle(.plain)
        .menuIndicator(.hidden)
        .frame(width: 32, height: 32)
        .help(title)
        .accessibilityLabel(title)
    }
}

private struct GlassCircleFill: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 26, *) {
            content.glassEffect(.regular.interactive(), in: .circle)
        } else {
            content.background(.ultraThinMaterial, in: Circle())
        }
    }
}

private struct DownloadBrowser: View {
    let library: DownloadLibrary
    let downloads: [Download]
    let workspaceID: Workspace.ID
    let folders: [SavedFolder]
    let settings: WorkspaceViewSettings
    let isFolderFilter: Bool
    let isSearching: Bool
    let onInspect: (Download.ID) -> Void

    private var arrangement: DownloadArrangement {
        DownloadArrangement.make(downloads: downloads, folders: folders, settings: settings)
    }

    var body: some View {
        Group {
            if downloads.isEmpty {
                ContentUnavailableView(
                    "No Downloads",
                    systemImage: "arrow.down.circle",
                    description: Text(emptyDescription)
                )
            } else if settings.layout == .table {
                DownloadTable(
                    library: library,
                    lines: arrangement.lines,
                    workspaceID: workspaceID,
                    savedFolders: folders,
                    showsGroup: arrangement.showsHeaders,
                    onInspect: onInspect
                )
            } else {
                DownloadList(
                    library: library,
                    arrangement: arrangement,
                    workspaceID: workspaceID,
                    savedFolders: folders,
                    grouping: settings.grouping,
                    onInspect: onInspect
                )
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var emptyDescription: String {
        if isSearching {
            return "Nothing in this Workspace matches the search."
        }
        if isFolderFilter {
            return "Nothing in this saved folder. Paste a link to add one here."
        }
        return "Paste a link to add a Download to this Workspace."
    }
}

private struct DownloadList: View {
    let library: DownloadLibrary
    let arrangement: DownloadArrangement
    let workspaceID: Workspace.ID
    let savedFolders: [SavedFolder]
    let grouping: DownloadGrouping
    let onInspect: (Download.ID) -> Void
    @FocusState private var isFocused: Bool

    private var selection: Binding<Set<Download.ID>> {
        Binding(
            get: { library.selectedDownloadIDs },
            set: { library.selectedDownloadIDs = $0 }
        )
    }

    var body: some View {
        // A native List owns selection, focus, and the inactive look, so the search field
        // and the list hand focus back and forth like any other macOS controls.
        List(selection: selection) {
            ForEach(arrangement.sections) { section in
                if arrangement.showsHeaders {
                    Section {
                        rows(in: section)
                    } header: {
                        DownloadGroupHeader(section: section, grouping: grouping)
                    }
                } else {
                    rows(in: section)
                }
            }
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
        .focused($isFocused)
        .contextMenu(forSelectionType: Download.ID.self) { ids in
            let downloads = arrangement.lines.map(\.download).filter { ids.contains($0.id) }
            if !downloads.isEmpty {
                DownloadActionMenu(
                    library: library,
                    downloads: downloads,
                    workspaceID: workspaceID,
                    savedFolders: savedFolders,
                    isMovingDownloads: library.isMovingDownloads,
                    hasQuickLook: library.quickLookURL != nil,
                    placement: .contextMenu
                )
                .equatable()
            }
        } primaryAction: { ids in
            guard ids.count == 1,
                  let download = arrangement.lines.first(where: { ids.contains($0.id) })?.download else {
                return
            }
            if download.source == .torrent {
                onInspect(download.id)
            } else if download.state == .finished {
                library.revealInFinder(download.id)
            }
        }
        .modifier(SoftTopScrollEdgeEffect())
        .background(Color(nsColor: .windowBackgroundColor).opacity(0.55))
    }

    private func rows(in section: DownloadArrangement.Section) -> some View {
        ForEach(section.lines) { line in
            DownloadRow(
                line: line,
                isSelected: library.selectedDownloadIDs.contains(line.id),
                isEmphasized: isFocused
            )
                .tag(line.id)
                .listRowSeparator(.hidden)
                .listRowBackground(Color.clear)
                .listRowInsets(EdgeInsets(top: 4, leading: 14, bottom: 4, trailing: 14))
        }
    }
}

private struct DownloadGroupHeader: View {
    let section: DownloadArrangement.Section
    let grouping: DownloadGrouping

    private var color: Color {
        if grouping == .state, let state = DownloadState(rawValue: section.id) {
            return stateColor(state)
        }
        return switch grouping {
        case .none: .secondary
        case .state: .blue
        case .folder: .orange
        case .kind: .purple
        case .source: .teal
        }
    }

    var body: some View {
        HStack(spacing: 7) {
            Image(systemName: grouping.symbolName)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(color)
                .frame(width: 20, height: 20)
                .background(color.opacity(0.12), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
                .accessibilityHidden(true)
            Text(section.title)
                .font(.subheadline.weight(.semibold))
            Text(section.lines.count, format: .number)
                .font(.caption2.monospacedDigit())
                .foregroundStyle(.secondary)
                .padding(.horizontal, 6)
                .padding(.vertical, 1)
                .background(Color.primary.opacity(0.06), in: Capsule())
            Spacer()
        }
        .padding(.horizontal, 4)
        .accessibilityElement(children: .combine)
    }
}

private struct DownloadStateText: View {
    let download: Download
    @Environment(\.videoTools) private var videoTools

    var body: some View {
        if download.isWaitingForVideoTools(videoTools) {
            Text("Getting video tools…")
                .foregroundStyle(.secondary)
        } else {
            Text(download.state.title)
                .foregroundStyle(stateColor(download.state))
        }
    }
}

private struct DownloadRow: View {
    let line: DownloadArrangement.Line
    let isSelected: Bool
    /// Whether the list has keyboard focus; an unfocused selection turns gray like a native one.
    let isEmphasized: Bool
    @State private var isHovered = false
    @Environment(\.videoTools) private var videoTools

    private var download: Download { line.download }

    var body: some View {
        HStack(alignment: .center, spacing: 12) {
            DownloadArtwork(download: download)

            VStack(alignment: .leading, spacing: 6) {
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Text(download.name)
                        .font(.headline)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Spacer(minLength: 8)
                    DownloadStatusBadge(state: download.state)
                }

                HStack(spacing: 12) {
                    DownloadFact(
                        systemImage: download.mediaSymbolName,
                        text: download.mediaTitle,
                        accessibilityLabel: "Type, \(download.mediaTitle)"
                    )
                    DownloadFact(
                        systemImage: "folder.fill",
                        text: line.folderName,
                        accessibilityLabel: "Folder, \(line.folderName)"
                    )
                    if download.effectiveSize > 0 {
                        let size = Formatters.byteString(download.effectiveSize)
                        DownloadFact(
                            systemImage: "internaldrive",
                            text: size,
                            accessibilityLabel: "Size, \(size)"
                        )
                    }
                    Spacer(minLength: 0)
                    if download.showsWideProgress, download.downloadRate > 0 {
                        let speed = Formatters.speedString(download.downloadRate)
                        DownloadFact(
                            systemImage: "arrow.down",
                            text: speed,
                            accessibilityLabel: "Download speed, \(speed)"
                        )
                    }
                    if download.showsWideProgress,
                       let etaSeconds = download.etaSeconds,
                       etaSeconds > 0 {
                        let duration = Formatters.durationString(etaSeconds)
                        DownloadFact(
                            systemImage: "clock",
                            text: duration,
                            accessibilityLabel: "Time remaining, \(duration)"
                        )
                    }
                }

                if download.showsWideProgress {
                    DownloadProgress(
                        fraction: download.progressFraction ?? 0,
                        color: stateColor(download.state)
                    )
                }

                if download.isWaitingForVideoTools(videoTools) {
                    Label("Getting video tools…", systemImage: "shippingbox")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else if let error = download.errorMessage, !error.isEmpty {
                    HStack(spacing: 8) {
                        Label(error, systemImage: "exclamationmark.triangle.fill")
                            .font(.caption)
                            .foregroundStyle(.red)
                            .lineLimit(1)
                        if download.offersOpulsYtDlp(videoTools) {
                            Button("Download Opul's copy") {
                                videoTools.downloadOpulsCopy(download.id)
                            }
                            .controlSize(.small)
                        }
                    }
                }
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background {
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(Color(nsColor: .controlBackgroundColor).opacity(isHovered || isSelected ? 1 : 0.84))
            if isSelected {
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(selectionColor.opacity(0.14))
            }
        }
        .overlay {
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .strokeBorder(borderColor, lineWidth: isSelected ? 1.5 : 1)
        }
        // The card shows selection, so the table's own row highlight stays hidden.
        .background(TableSelectionHighlightRemover())
        .shadow(color: .black.opacity(isHovered ? 0.08 : 0.04), radius: isHovered ? 7 : 4, y: 2)
        .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(download.accessibilitySummary(folderName: line.folderName))
        .accessibilityHint(download.activationHint)
        .accessibilityActions {
            if download.offersOpulsYtDlp(videoTools) {
                Button("Download Opul's copy") {
                    videoTools.downloadOpulsCopy(download.id)
                }
            }
        }
        .help(download.address)
        .onHover { isHovered = $0 }
        .animation(.easeOut(duration: 0.14), value: isHovered)
        .animation(.easeOut(duration: 0.1), value: isSelected)
    }

    private var selectionColor: Color {
        isEmphasized ? .accentColor : .secondary
    }

    private var borderColor: Color {
        if isSelected { return selectionColor.opacity(0.7) }
        if isHovered { return stateColor(download.state).opacity(0.38) }
        return Color.primary.opacity(0.1)
    }
}

/// Turns off the row highlight of the table backing the enclosing `List`. With no highlight,
/// rows also keep their normal foreground styles instead of switching to the emphasized ones.
private struct TableSelectionHighlightRemover: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        TableSelectionHighlightRemoverView()
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        (nsView as? TableSelectionHighlightRemoverView)?.removeHighlight()
    }
}

private final class TableSelectionHighlightRemoverView: NSView {
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        removeHighlight()
    }

    func removeHighlight() {
        var view = superview
        while let current = view, !(current is NSTableView) {
            view = current.superview
        }
        guard let table = view as? NSTableView, table.selectionHighlightStyle != .none else { return }
        table.selectionHighlightStyle = .none
    }
}

private struct DownloadArtwork: View {
    let download: Download
    @ScaledMetric(relativeTo: .headline) private var size: CGFloat = 44

    private var thumbnailURL: URL? {
        guard let rawValue = download.thumbnailURL, !rawValue.isEmpty else { return nil }
        if let url = URL(string: rawValue), url.scheme != nil {
            return url
        }
        return URL(fileURLWithPath: rawValue)
    }

    var body: some View {
        ZStack {
            if let artwork = DownloadArtworkAssets.image(named: download.fallbackArtworkName) {
                Image(nsImage: artwork)
                    .resizable()
                    .scaledToFit()
            } else {
                Image(systemName: download.mediaSymbolName)
                    .font(.system(size: size * 0.42, weight: .semibold))
                    .foregroundStyle(.secondary)
            }

            if let thumbnailURL {
                AsyncImage(url: thumbnailURL) { phase in
                    if let image = phase.image {
                        image
                            .resizable()
                            .scaledToFill()
                            .clipShape(.rect(cornerRadius: 11))
                            .overlay {
                                RoundedRectangle(cornerRadius: 11, style: .continuous)
                                    .strokeBorder(.white.opacity(0.28), lineWidth: 1)
                            }
                    }
                }
            }
        }
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }
}

private enum DownloadArtworkAssets {
    private static let names = [
        "media-application",
        "media-archive",
        "media-audio",
        "media-book",
        "media-code",
        "media-disk-image",
        "media-document",
        "media-game",
        "media-generic",
        "media-image",
        "media-subtitle",
        "media-torrent",
        "media-video",
        "media-web",
    ]

    private static let images: [String: NSImage] = Dictionary(
        uniqueKeysWithValues: names.compactMap { name in
            guard let url = Bundle.opulResources.url(
                forResource: name,
                withExtension: "png",
                subdirectory: "FallbackArtwork"
            ), let image = NSImage(contentsOf: url) else {
                return nil
            }
            return (name, image)
        }
    )

    static func image(named name: String) -> NSImage? {
        images[name]
    }
}

private struct DownloadStatusBadge: View {
    let state: DownloadState

    var body: some View {
        Label(state.title, systemImage: state.symbolName)
            .font(.caption.weight(.semibold))
            .foregroundStyle(stateColor(state))
            .padding(.horizontal, 7)
            .padding(.vertical, 3)
            .background(stateColor(state).opacity(0.12), in: Capsule())
            .fixedSize()
    }
}

private struct DownloadFact: View {
    let systemImage: String
    let text: String
    let accessibilityLabel: String

    var body: some View {
        Label(text, systemImage: systemImage)
            .font(.caption)
            .foregroundStyle(.secondary)
            .lineLimit(1)
            .accessibilityLabel(accessibilityLabel)
    }
}

private struct DownloadProgress: View {
    let fraction: Double
    let color: Color
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var clampedFraction: Double {
        min(max(fraction, 0), 1)
    }

    var body: some View {
        HStack(spacing: 8) {
            GeometryReader { proxy in
                ZStack(alignment: .leading) {
                    Capsule()
                        .fill(Color.primary.opacity(0.09))
                    Capsule()
                        .fill(color)
                        .frame(width: proxy.size.width * clampedFraction)
                }
            }
            .frame(minWidth: 100)
            .frame(height: 6)

            Text("\(Int((clampedFraction * 100).rounded()))%")
                .monospacedDigit()
                .fontWeight(.semibold)
                .foregroundStyle(color)
                .frame(width: 38, alignment: .trailing)
        }
        .font(.caption2)
        .foregroundStyle(.secondary)
        .animation(reduceMotion ? nil : .linear(duration: 0.2), value: clampedFraction)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Download progress")
        .accessibilityValue("\(Int(clampedFraction * 100)) percent")
    }
}

private extension Download {
    var showsWideProgress: Bool {
        state == .fetching || state == .processing
    }

    var mediaTitle: String {
        kind?.title ?? source.title
    }

    var mediaSymbolName: String {
        switch kind {
        case .video: "film.fill"
        case .audio: "waveform"
        case .file: "doc.fill"
        case nil: source == .torrent ? "point.3.connected.trianglepath.dotted" : "link"
        }
    }

    var fallbackArtworkName: String {
        switch kind {
        case .video:
            return "media-video"
        case .audio:
            return "media-audio"
        case .file, nil:
            break
        }

        let fileExtension = artworkFileExtension
        if Self.imageExtensions.contains(fileExtension) { return "media-image" }
        if Self.subtitleExtensions.contains(fileExtension) { return "media-subtitle" }
        if Self.bookExtensions.contains(fileExtension) { return "media-book" }
        if Self.archiveExtensions.contains(fileExtension) { return "media-archive" }
        if Self.codeExtensions.contains(fileExtension) { return "media-code" }
        if Self.documentExtensions.contains(fileExtension) { return "media-document" }
        if Self.applicationExtensions.contains(fileExtension) { return "media-application" }
        if Self.diskImageExtensions.contains(fileExtension) { return "media-disk-image" }
        if Self.gameExtensions.contains(fileExtension) { return "media-game" }
        if Self.webExtensions.contains(fileExtension) { return "media-web" }
        if source == .torrent || fileExtension == "torrent" { return "media-torrent" }
        return "media-generic"
    }

    private var artworkFileExtension: String {
        var candidates = [outputPath, largestFileName, name, address]
        if let contentFolder {
            candidates.insert(contentFolder, at: 1)
        }
        for candidate in candidates.compactMap({ $0 }) {
            let remoteExtension = URL(string: candidate)?.pathExtension.lowercased() ?? ""
            if !remoteExtension.isEmpty { return remoteExtension }

            let localExtension = URL(fileURLWithPath: candidate).pathExtension.lowercased()
            if !localExtension.isEmpty { return localExtension }
        }
        return ""
    }

    private var largestFileName: String? {
        files.max(by: { $0.size < $1.size })?.name
    }

    private static let imageExtensions: Set<String> = [
        "avif", "bmp", "gif", "heic", "heif", "jpeg", "jpg", "png", "svg", "tif", "tiff", "webp",
    ]
    private static let subtitleExtensions: Set<String> = ["ass", "srt", "ssa", "sub", "vtt"]
    private static let bookExtensions: Set<String> = ["azw", "azw3", "cbr", "cbz", "epub", "mobi"]
    private static let archiveExtensions: Set<String> = [
        "7z", "bz2", "gz", "rar", "tar", "tbz", "tbz2", "tgz", "txz", "xz", "zip",
    ]
    private static let codeExtensions: Set<String> = [
        "c", "cc", "cpp", "css", "go", "h", "hpp", "java", "js", "json", "jsx", "kt", "kts", "lua",
        "m", "mm", "php", "pl", "py", "rb", "rs", "scss", "sh", "sql", "swift", "toml", "ts", "tsx",
        "xml", "yaml", "yml",
    ]
    private static let documentExtensions: Set<String> = [
        "csv", "doc", "docx", "key", "md", "numbers", "odf", "ods", "odt", "pages", "pdf", "ppt",
        "pptx", "rtf", "txt", "xls", "xlsx",
    ]
    private static let applicationExtensions: Set<String> = ["app", "exe", "ipa", "msi", "pkg"]
    private static let diskImageExtensions: Set<String> = ["dmg", "img", "iso"]
    private static let gameExtensions: Set<String> = [
        "3ds", "cia", "gb", "gba", "gbc", "gen", "n64", "nds", "nes", "rom", "sfc", "smc",
    ]
    private static let webExtensions: Set<String> = ["htm", "html", "url", "webarchive"]

    var activationHint: String {
        if source == .torrent { return "Double-click for torrent info." }
        if state == .finished { return "Double-click to reveal in Finder." }
        return ""
    }

    func accessibilitySummary(folderName: String) -> String {
        var parts = [name, state.title, mediaTitle, folderName]
        if showsWideProgress {
            parts.append("\(Int((progressFraction ?? 0) * 100)) percent")
        }
        return parts.joined(separator: ", ")
    }
}

private struct DownloadTable: View {
    let library: DownloadLibrary
    let lines: [DownloadArrangement.Line]
    let workspaceID: Workspace.ID
    let savedFolders: [SavedFolder]
    let showsGroup: Bool
    let onInspect: (Download.ID) -> Void

    var body: some View {
        // A conditional column inside Table needs macOS 14.4. This target is macOS 14.
        Group {
            if showsGroup {
                groupedTable
            } else {
                plainTable
            }
        }
        .tableStyle(.inset)
        .alternatingRowBackgrounds()
        .contextMenu(forSelectionType: Download.ID.self) { ids in
            let downloads = lines.map(\.download).filter { ids.contains($0.id) }
            if !downloads.isEmpty {
                DownloadActionMenu(
                    library: library,
                    downloads: downloads,
                    workspaceID: workspaceID,
                    savedFolders: savedFolders,
                    isMovingDownloads: library.isMovingDownloads,
                    hasQuickLook: library.quickLookURL != nil,
                    placement: .contextMenu
                )
                .equatable()
            }
        } primaryAction: { ids in
            guard ids.count == 1, let download = lines.first(where: { ids.contains($0.id) })?.download else {
                return
            }
            if download.source == .torrent {
                onInspect(download.id)
            } else if download.state == .finished {
                library.revealInFinder(download.id)
            }
        }
    }

    private var selection: Binding<Set<Download.ID>> {
        Binding(
            get: { library.selectedDownloadIDs },
            set: { library.selectedDownloadIDs = $0 }
        )
    }

    private var groupedTable: some View {
        Table(lines, selection: selection) {
            TableColumn("Group") { line in
                Text(line.groupTitle)
            }
            .width(min: 90, ideal: 120)
            TableColumn("Name") { line in
                Text(line.download.name)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .help(line.download.address)
            }
            .width(min: 160, ideal: 280)
            TableColumn("State") { line in
                DownloadStateText(download: line.download)
            }
            .width(min: 80, ideal: 100)
            TableColumn("Progress") { line in
                DownloadProgressText(percentage: line.download.progressPercentage)
            }
            .width(min: 65, ideal: 75)
            .alignment(.numeric)
            TableColumn("Source") { line in
                Text(line.download.source.title)
            }
            .width(min: 90, ideal: 120)
            TableColumn("Kind") { line in
                Text(line.download.kind?.title ?? "Not known yet")
            }
            .width(min: 90, ideal: 120)
            TableColumn("Format") { line in
                Text(line.download.format?.title ?? "—")
            }
            .width(min: 80, ideal: 100)
            TableColumn("Folder") { line in
                Text(line.folderName)
            }
            .width(min: 90, ideal: 120)
        }
    }

    private var plainTable: some View {
        Table(lines, selection: selection) {
            TableColumn("Name") { line in
                Text(line.download.name)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .help(line.download.address)
            }
            .width(min: 160, ideal: 280)
            TableColumn("State") { line in
                DownloadStateText(download: line.download)
            }
            .width(min: 80, ideal: 100)
            TableColumn("Progress") { line in
                DownloadProgressText(percentage: line.download.progressPercentage)
            }
            .width(min: 65, ideal: 75)
            .alignment(.numeric)
            TableColumn("Source") { line in
                Text(line.download.source.title)
            }
            .width(min: 90, ideal: 120)
            TableColumn("Kind") { line in
                Text(line.download.kind?.title ?? "Not known yet")
            }
            .width(min: 90, ideal: 120)
            TableColumn("Format") { line in
                Text(line.download.format?.title ?? "—")
            }
            .width(min: 80, ideal: 100)
            TableColumn("Folder") { line in
                Text(line.folderName)
            }
            .width(min: 90, ideal: 120)
        }
    }
}

private struct DownloadProgressText: View {
    let percentage: Int?

    var body: some View {
        Text(percentage.map { "\($0)%" } ?? "")
            .monospacedDigit()
            .accessibilityLabel("Download progress")
            .accessibilityValue(percentage.map { "\($0) percent" } ?? "")
            .accessibilityHidden(percentage == nil)
    }
}

private func stateColor(_ state: DownloadState) -> Color {
    switch state {
    case .queued: .orange
    case .fetching: .blue
    case .processing: .purple
    case .paused: .orange
    case .seeding: .teal
    case .finished: .green
    case .failed: .red
    case .cancelled: .secondary
    }
}
