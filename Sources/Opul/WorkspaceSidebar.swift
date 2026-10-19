import SwiftUI

struct WorkspaceSidebar: View {
    @Bindable var library: DownloadLibrary
    @State private var expandedIDs: Set<Workspace.ID>

    init(library: DownloadLibrary) {
        self.library = library
        _expandedIDs = State(initialValue: Set(library.workspaces.map(\.id)))
    }

    var body: some View {
        List(selection: $library.selection) {
            Section("Workspaces") {
                ForEach(workspaceEntries) { entry in
                    let workspaceFolders = folders(under: entry.workspaceID)
                    if workspaceFolders.count > 1 {
                        DisclosureGroup(isExpanded: expanded(entry.workspaceID)) {
                            ForEach(workspaceFolders) { folder in
                                SidebarRow(entry: folder, library: library)
                                    .tag(folder.selection)
                            }
                        } label: {
                            SidebarRow(entry: entry, library: library)
                        }
                        .tag(entry.selection)
                    } else {
                        SidebarRow(entry: entry, library: library)
                            .tag(entry.selection)
                    }
                }
            }
            Button("New Workspace", systemImage: "plus") {
                library.sheet = .create
            }
            .buttonStyle(.plain)
        }
        .listStyle(.sidebar)
        .padding(.bottom, -8)
        .navigationSplitViewColumnWidth(min: 200, ideal: 240, max: 320)
        .onChange(of: library.workspaces.map(\.id)) { _, ids in
            for id in ids where !expandedIDs.contains(id) {
                expandedIDs.insert(id)
            }
        }
    }

    private var workspaceEntries: [SidebarEntry] {
        library.sidebarEntries().filter { !$0.isFolder }
    }

    private func folders(under workspaceID: Workspace.ID) -> [SidebarEntry] {
        library.sidebarEntries().filter { $0.isFolder && $0.workspaceID == workspaceID }
    }

    private func expanded(_ id: Workspace.ID) -> Binding<Bool> {
        Binding(
            get: { expandedIDs.contains(id) },
            set: { isExpanded in
                if isExpanded {
                    expandedIDs.insert(id)
                } else {
                    expandedIDs.remove(id)
                }
            }
        )
    }
}

private struct SidebarRow: View {
    let entry: SidebarEntry
    let library: DownloadLibrary

    var body: some View {
        HStack(spacing: 6) {
            if entry.isFolder {
                Image(systemName: "folder")
                    .foregroundStyle(.secondary)
                    .frame(width: 17, height: 17)
            } else if let emoji = entry.emoji {
                Text(emoji)
                    .frame(width: 17, height: 17)
            } else if let symbolName = entry.symbolName {
                Image(systemName: symbolName)
                    .foregroundStyle(.secondary)
                    .frame(width: 17, height: 17)
            }
            Text(entry.title)
                .lineLimit(1)
            Spacer(minLength: 4)
            if entry.isFolder && entry.isDefault {
                Text("Default")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            } else if !entry.isFolder {
                Text(entry.finishedCount.formatted())
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                    .monospacedDigit()
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(accessibilityLabel)
        .help(help)
        .contextMenu {
            if entry.isFolder {
                Button {
                    guard let folderID = entry.folderID else { return }
                    library.makeDefault(folderID, in: entry.workspaceID)
                } label: {
                    Label(
                        entry.isDefault ? "Default Folder" : "Make Default",
                        systemImage: entry.isDefault ? "checkmark" : "checkmark.circle"
                    )
                }
                .disabled(entry.isDefault)
                Button("Show in Finder", systemImage: "folder") {
                    guard let folderID = entry.folderID else { return }
                    library.revealFolder(folderID)
                }
                Button("Change Folder Location…") {
                    guard let folderID = entry.folderID else { return }
                    library.beginReplaceFolder(folderID, in: entry.workspaceID)
                }
                Divider()
                Button(removeFolderTitle, role: .destructive) {
                    guard let folderID = entry.folderID else { return }
                    library.askToRemoveFolder(folderID, in: entry.workspaceID)
                }
            } else {
                Button("Workspace Settings…") {
                    library.sheet = .settings(entry.workspaceID)
                }
                if let defaultFolder = library.workspaces.first(where: { $0.id == entry.workspaceID })?.savedFolders.first,
                   library.workspaces.first(where: { $0.id == entry.workspaceID })?.savedFolders.count == 1 {
                    Button("Show in Finder", systemImage: "folder") {
                        library.revealFolder(defaultFolder.id)
                    }
                    Button("Choose Replacement Folder…") {
                        library.beginReplaceFolder(defaultFolder.id, in: entry.workspaceID)
                    }
                }
                Button("Add Saved Folder…") {
                    library.beginAddFolder(in: entry.workspaceID)
                }
                Divider()
                Button("Delete…", role: .destructive) {
                    library.askToDelete(entry.workspaceID)
                }
            }
        }
    }

    private var removeFolderTitle: String {
        guard let folderID = entry.folderID else { return "Remove Saved Folder" }
        let count = library.downloadCount(in: folderID, workspaceID: entry.workspaceID)
        return count == 0 ? "Remove Saved Folder" : "Remove Saved Folder…"
    }

    private var help: String {
        if entry.isFolder {
            return entry.path ?? entry.title
        }
        return "\(entry.finishedCount) finished"
    }

    private var accessibilityLabel: String {
        if entry.isFolder {
            return entry.isDefault ? "\(entry.title), Default" : entry.title
        }
        return "\(entry.title), \(entry.finishedCount) finished"
    }
}
