import SwiftUI

struct MediaImportSheet: View {
    @Environment(\.dismiss) private var dismiss
    let library: DownloadLibrary
    let request: MediaImportRequest

    @State private var selectedIndexes: Set<Int>
    @State private var format: Format
    @State private var folderID: SavedFolder.ID
    @State private var rememberFormat = false

    init(library: DownloadLibrary, request: MediaImportRequest) {
        self.library = library
        self.request = request
        let workspace = library.workspaces.first { $0.id == request.workspaceID }
        _selectedIndexes = State(initialValue: Set(request.probe.items.indices))
        _format = State(initialValue: workspace?.defaultFormat ?? .best)
        _folderID = State(initialValue: request.folderID)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 4) {
                Text(request.probe.isPlaylist ? "Choose Playlist Downloads" : "Download Media")
                    .font(.title2)
                Text(request.probe.title)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            .padding(20)

            Divider()

            List(request.probe.items.indices, id: \.self, selection: $selectedIndexes) { index in
                let item = request.probe.items[index]
                HStack(spacing: 10) {
                    Toggle("", isOn: selectionBinding(index))
                        .labelsHidden()
                    VStack(alignment: .leading, spacing: 2) {
                        Text(item.title)
                            .lineLimit(2)
                        HStack(spacing: 8) {
                            if let playlistIndex = item.playlistIndex {
                                Text("#\(playlistIndex)")
                            }
                            if let duration = item.durationSeconds {
                                Text(duration.formattedDuration)
                            }
                            if let extractor = item.extractor {
                                Text(extractor)
                            }
                        }
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    }
                    Spacer()
                }
                .tag(index)
                .padding(.vertical, 3)
            }
            .frame(minHeight: request.probe.isPlaylist ? 260 : 100)

            Divider()

            Form {
                Picker("Format", selection: $format) {
                    ForEach(Format.allCases) { format in
                        Text(format.title).tag(format)
                    }
                }
                Picker("Save To", selection: $folderID) {
                    ForEach(folders) { folder in
                        Text(folder.name).tag(folder.id)
                    }
                }
                Toggle("Make this the Workspace default", isOn: $rememberFormat)
                Toggle("Download subtitles", isOn: Binding(
                    get: { library.preferences.downloadSubtitles },
                    set: { library.preferences.downloadSubtitles = $0 }
                ))
                Toggle("Write thumbnail", isOn: Binding(
                    get: { library.preferences.writeThumbnail },
                    set: { library.preferences.writeThumbnail = $0 }
                ))
                Toggle("Write metadata JSON", isOn: Binding(
                    get: { library.preferences.writeMetadata },
                    set: { library.preferences.writeMetadata = $0 }
                ))
            }
            .formStyle(.grouped)

            Divider()

            HStack {
                Button("Cancel") { dismiss() }
                Spacer()
                Text("\(selectedIndexes.count) selected")
                    .foregroundStyle(.secondary)
                Button("Download") {
                    if rememberFormat {
                        library.setDefaultFormat(format, on: request.workspaceID)
                    }
                    library.queueMediaItems(
                        request,
                        itemIndexes: selectedIndexes,
                        format: format,
                        folderID: folderID
                    )
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.defaultAction)
                .disabled(selectedIndexes.isEmpty)
            }
            .padding(16)
        }
        .frame(minWidth: 600, minHeight: request.probe.isPlaylist ? 560 : 430)
    }

    private var folders: [SavedFolder] {
        library.workspaces.first { $0.id == request.workspaceID }?.savedFolders ?? []
    }

    private func selectionBinding(_ index: Int) -> Binding<Bool> {
        Binding(
            get: { selectedIndexes.contains(index) },
            set: { selected in
                if selected {
                    selectedIndexes.insert(index)
                } else {
                    selectedIndexes.remove(index)
                }
            }
        )
    }
}

private extension UInt64 {
    var formattedDuration: String {
        let hours = self / 3_600
        let minutes = (self % 3_600) / 60
        let seconds = self % 60
        if hours > 0 {
            return String(format: "%d:%02d:%02d", Int(hours), Int(minutes), Int(seconds))
        }
        return String(format: "%d:%02d", Int(minutes), Int(seconds))
    }
}
