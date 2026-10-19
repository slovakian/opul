import SwiftUI

struct CreateWorkspaceSheet: View {
    @Environment(\.dismiss) private var dismiss
    let library: DownloadLibrary
    let sheet: LibrarySheet

    @State private var name: String
    @State private var mode: IconMode
    @State private var symbolName: String
    @State private var emoji: String
    @State private var folderPath: String
    @State private var fileOrganizationOverride: DownloadFileOrganization?
    @State private var isChoosingFolder = false

    init(library: DownloadLibrary, sheet: LibrarySheet) {
        self.library = library
        self.sheet = sheet
        let existing = sheet.workspaceID.flatMap { id in
            library.workspaces.first { $0.id == id }
        }
        let seededMode: IconMode
        let seededSymbol: String
        let seededEmoji: String
        switch existing?.icon {
        case .emoji(let value):
            seededMode = .emoji
            seededSymbol = SidebarSymbol.choices[0].name
            seededEmoji = value
        case .symbol(let value):
            seededMode = .symbol
            seededSymbol = value
            seededEmoji = ""
        case nil:
            seededMode = .symbol
            seededSymbol = SidebarSymbol.choices[0].name
            seededEmoji = ""
        }
        // One-time draft for this presentation. Later edits stay in the sheet.
        self.name = existing?.name ?? ""
        self.mode = seededMode
        self.symbolName = seededSymbol
        self.emoji = seededEmoji
        self.fileOrganizationOverride = existing?.fileOrganizationOverride
        let defaultPath = MacFolder.path(for: .downloadsDirectory)
        _folderPath = State(initialValue: existing?.savedFolders.first(where: \.isDefault)?.path ?? defaultPath)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(sheet.title)
                .font(.title2)
            Form {
                TextField("Name", text: $name)
                    .onSubmit {
                        if canSave { save() }
                    }
                Picker("Icon", selection: $mode) {
                    Text("Symbol").tag(IconMode.symbol)
                    Text("Emoji").tag(IconMode.emoji)
                }
                .pickerStyle(.segmented)

                if case .create = sheet {
                    Section("Default Location") {
                        HStack {
                            Image(systemName: "folder")
                                .foregroundStyle(.secondary)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(folderDisplayName)
                                    .font(.body)
                                Text(folderPath)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                            }
                            Spacer()
                            Button("Choose…") {
                                isChoosingFolder = true
                            }
                            if folderPath != defaultDownloadsPath {
                                Button("Reset") {
                                    folderPath = defaultDownloadsPath
                                }
                                .help("Reset to macOS default (Downloads)")
                            }
                        }
                    }
                }

                if case .settings = sheet {
                    Section("File Organization") {
                        Picker("Organization", selection: $fileOrganizationOverride) {
                            Text("Use App Default").tag(nil as DownloadFileOrganization?)
                            ForEach(DownloadFileOrganization.allCases) { organization in
                                Text(organization.title).tag(Optional(organization))
                            }
                        }
                        Text(fileOrganizationExplanation)
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .formStyle(.grouped)
            iconChooser
            HStack {
                Button("Cancel") {
                    dismiss()
                }
                Spacer()
                Button(sheet.confirmTitle, action: save)
                    .buttonStyle(.borderedProminent)
                    .disabled(!canSave)
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(minWidth: 440, minHeight: 440)
        .fileImporter(
            isPresented: $isChoosingFolder,
            allowedContentTypes: [.folder],
            allowsMultipleSelection: false
        ) { result in
            if case .success(let urls) = result, let url = urls.first {
                let accessed = url.startAccessingSecurityScopedResource()
                defer { if accessed { url.stopAccessingSecurityScopedResource() } }
                folderPath = url.path(percentEncoded: false)
            }
        }
    }

    @ViewBuilder private var iconChooser: some View {
        if mode == .symbol {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 44), spacing: 8)], spacing: 8) {
                ForEach(SidebarSymbol.choices) { symbol in
                    Button {
                        symbolName = symbol.name
                    } label: {
                        Image(systemName: symbol.name)
                            .frame(width: 36, height: 28)
                    }
                    .buttonStyle(.bordered)
                    .tint(symbolName == symbol.name ? Color.accentColor : Color.secondary)
                    .accessibilityLabel(symbol.title)
                    .accessibilityAddTraits(symbolName == symbol.name ? .isSelected : [])
                }
            }
        } else {
            TextField("Emoji", text: $emoji, prompt: Text("One emoji"))
                .onChange(of: emoji) { _, newValue in
                    let one = newValue.first.map(String.init) ?? ""
                    if emoji != one {
                        emoji = one
                    }
                }
            HStack(spacing: 8) {
                ForEach(SuggestedEmoji.choices) { suggestion in
                    Button(suggestion.value) {
                        emoji = suggestion.value
                    }
                    .buttonStyle(.bordered)
                    .accessibilityLabel("Emoji \(suggestion.value)")
                }
                Spacer(minLength: 0)
            }
        }
    }

    private var trimmedName: String {
        name.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var trimmedEmoji: String {
        emoji.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var canSave: Bool {
        if trimmedName.isEmpty { return false }
        if mode == .emoji { return !trimmedEmoji.isEmpty }
        return true
    }

    private var chosenIcon: WorkspaceIcon {
        if mode == .emoji, let character = trimmedEmoji.first {
            return .emoji(String(character))
        }
        return .symbol(symbolName)
    }

    private var defaultDownloadsPath: String {
        MacFolder.path(for: .downloadsDirectory)
    }

    private var folderDisplayName: String {
        if folderPath == defaultDownloadsPath {
            return "Downloads (macOS Default)"
        }
        let last = URL(fileURLWithPath: folderPath).lastPathComponent
        return last.isEmpty ? folderPath : last
    }

    private var fileOrganizationExplanation: String {
        let organization = fileOrganizationOverride ?? library.preferences.fileOrganization
        let prefix = fileOrganizationOverride == nil ? "Using App Default. " : ""
        return prefix + organization.explanation
    }

    private func save() {
        guard canSave else { return }
        switch sheet {
        case .create:
            library.createWorkspace(name: trimmedName, icon: chosenIcon, folderPath: folderPath)
        case .guided:
            return
        case .settings(let id):
            library.updateWorkspaceSettings(
                id,
                to: trimmedName,
                icon: chosenIcon,
                fileOrganizationOverride: fileOrganizationOverride
            )
        }
        dismiss()
    }
}

extension CreateWorkspaceSheet {
    private enum IconMode: Hashable {
        case symbol
        case emoji
    }
}

private struct SuggestedEmoji: Identifiable {
    let value: String
    var id: String { value }

    static let choices: [SuggestedEmoji] = ["🎓", "🎬", "🎧", "📚", "⭐️"].map(SuggestedEmoji.init)
}

// PreviewProvider, not #Preview: SwiftPM builds this target and has no PreviewsMacros plugin.
struct CreateWorkspaceSheetPreview: PreviewProvider {
    static var previews: some View {
        CreateWorkspaceSheet(library: DownloadLibrary(), sheet: .create)
    }
}
