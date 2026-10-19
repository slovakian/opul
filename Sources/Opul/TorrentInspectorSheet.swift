import SwiftUI
import OpulCore

struct TorrentInspectorSheet: View {
    @Environment(\.dismiss) private var dismiss
    let library: DownloadLibrary
    let downloadID: Download.ID

    @State private var selectedTab: InspectorTab = .general

    enum InspectorTab: String, CaseIterable, Identifiable {
        case general = "General"
        case files = "Files"
        case trackers = "Trackers"
        case swarm = "Swarm"

        var id: String { rawValue }

        var symbolName: String {
            switch self {
            case .general: "info.circle"
            case .files: "doc.on.doc"
            case .trackers: "network"
            case .swarm: "person.2"
            }
        }
    }

    private var download: Download? {
        library.findDownload(downloadID)
    }

    private var folderName: String {
        guard let download else { return "" }
        return library.folderName(for: download.folderID)
    }

    var body: some View {
        VStack(spacing: 0) {
            if let download {
                headerView(download)
                Divider()
                tabPicker
                Divider()
                tabContent(download)
                    .frame(minWidth: 540, minHeight: 380)
            } else {
                ContentUnavailableView(
                    "Download Not Found",
                    systemImage: "exclamationmark.triangle",
                    description: Text("This torrent is no longer in the workspace.")
                )
            }
            Divider()
            footerView
        }
        .frame(minWidth: 560, minHeight: 480)
    }

    private func headerView(_ download: Download) -> some View {
        HStack(spacing: 12) {
            Image(systemName: download.state.symbolName)
                .font(.system(size: 28))
                .foregroundStyle(download.state == .failed ? .red : (download.state == .seeding ? .green : .accentColor))
            VStack(alignment: .leading, spacing: 2) {
                Text(download.name)
                    .font(.headline)
                    .lineLimit(1)
                Text("\(download.state.title) · \(folderName)")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if download.capabilities.canStopSeeding {
                Button("Stop Seeding") {
                    library.stopSeeding(download.id)
                }
                .buttonStyle(.borderedProminent)
            } else if download.capabilities.canPause {
                Button("Pause") {
                    library.pauseDownload(download.id)
                }
                .buttonStyle(.bordered)
            } else if download.capabilities.canResume {
                Button("Resume") {
                    library.resumeDownload(download.id)
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding(16)
    }

    private var tabPicker: some View {
        Picker("Inspector Tab", selection: $selectedTab) {
            ForEach(InspectorTab.allCases) { tab in
                Label(tab.rawValue, systemImage: tab.symbolName).tag(tab)
            }
        }
        .pickerStyle(.segmented)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private func tabContent(_ download: Download) -> some View {
        switch selectedTab {
        case .general:
            generalTab(download)
        case .files:
            filesTab(download)
        case .trackers:
            trackersTab(download)
        case .swarm:
            swarmTab(download)
        }
    }

    private func generalTab(_ download: Download) -> some View {
        Form {
            Section("Identifiers") {
                LabeledContent("Name", value: download.name)
                if let hash = download.infoHash {
                    LabeledContent("Info Hash") {
                        HStack(spacing: 6) {
                            Text(hash)
                                .font(.system(.footnote, design: .monospaced))
                                .textSelection(.enabled)
                            Button("Copy", systemImage: "doc.on.doc") {
                                library.copyInfoHash(download.id)
                            }
                            .buttonStyle(.borderless)
                        }
                    }
                }
                LabeledContent("Source", value: download.source.title)
                LabeledContent("Kind", value: download.kind?.title ?? "Not known yet")
            }

            Section("Transfer") {
                LabeledContent("State", value: download.state.title)
                LabeledContent("Total Size", value: Formatters.byteString(download.totalSize))
                if download.effectiveSize != download.totalSize {
                    LabeledContent("Selected Size", value: Formatters.byteString(download.effectiveSize))
                }
                LabeledContent("Downloaded", value: Formatters.byteString(download.bytesDownloaded))
                LabeledContent("Uploaded", value: Formatters.byteString(download.bytesUploaded))
                LabeledContent("Seed Ratio", value: String(format: "%.2f", download.seedRatio))
                if let eta = download.etaSeconds {
                    LabeledContent("ETA", value: Formatters.durationString(eta))
                }
            }

            Section("Pieces & Metadata") {
                if download.pieceLength > 0 {
                    LabeledContent("Piece Size", value: Formatters.byteString(download.pieceLength))
                }
                if download.pieceCount > 0 {
                    LabeledContent("Pieces", value: "\(download.pieceCount) pieces")
                }
                if let comment = download.comment, !comment.isEmpty {
                    LabeledContent("Comment", value: comment)
                }
            }

            Section("Actions") {
                HStack(spacing: 12) {
                    Button("Copy Magnet URI") {
                        library.copyMagnetURI(download.id)
                    }
                    Button("Force Re-check") {
                        library.forceRecheck(download.id)
                    }
                    Button("Reveal in Finder") {
                        library.revealInFinder(download.id)
                    }
                }
            }
        }
        .formStyle(.grouped)
    }

    private func filesTab(_ download: Download) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                let includedCount = download.files.filter(\.isIncluded).count
                Text("\(includedCount) of \(download.files.count) files selected (Size: \(Formatters.byteString(download.effectiveSize)))")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Select All") {
                    library.selectAllFiles(download.id, included: true)
                }
                .buttonStyle(.borderless)
                .disabled(!download.capabilities.canChangeFileSelection)
                Button("Deselect All") {
                    library.selectAllFiles(download.id, included: false)
                }
                .buttonStyle(.borderless)
                .disabled(!download.capabilities.canChangeFileSelection)
            }
            .padding(.horizontal, 16)
            .padding(.top, 8)

            if download.files.isEmpty {
                ContentUnavailableView(
                    "No Files Yet",
                    systemImage: "doc.badge.arrow.up",
                    description: Text(download.state == .fetching ? "Fetching torrent metadata from swarm…" : "Single file torrent.")
                )
            } else {
                List {
                    ForEach(download.files) { file in
                        HStack(spacing: 8) {
                            Toggle("", isOn: Binding(
                                get: { file.isIncluded },
                                set: { _ in library.toggleFileInclusion(downloadID: download.id, fileIndex: file.fileIndex) }
                            ))
                            .labelsHidden()
                            .disabled(!download.capabilities.canChangeFileSelection)

                            Image(systemName: iconForFile(file.name))
                                .foregroundStyle(.secondary)
                                .frame(width: 16)

                            VStack(alignment: .leading, spacing: 2) {
                                Text(file.name)
                                    .lineLimit(1)
                                Text(file.path)
                                    .font(.caption2)
                                    .foregroundStyle(.tertiary)
                                    .lineLimit(1)
                            }

                            Spacer(minLength: 8)

                            ProgressView(value: file.progress)
                                .frame(width: 60)

                            Text(file.formattedSize)
                                .font(.caption)
                                .monospacedDigit()
                                .foregroundStyle(.secondary)
                                .frame(width: 60, alignment: .trailing)

                            Menu {
                                ForEach(TorrentFilePriority.allCases) { priority in
                                    Button(priority.rawValue) {
                                        library.setFilePriority(downloadID: download.id, fileIndex: file.fileIndex, priority: priority)
                                    }
                                }
                            } label: {
                                Text(file.priority.rawValue)
                                    .font(.caption)
                            }
                            .menuStyle(.borderlessButton)
                            .frame(width: 60)
                            .disabled(!download.capabilities.canChangeFileSelection)
                        }
                        .padding(.vertical, 2)
                    }
                }
                .listStyle(.inset)
            }
        }
    }

    private func trackersTab(_ download: Download) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("\(download.trackers.count) Trackers")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Update Trackers", systemImage: "arrow.clockwise") {
                    library.updateTrackers(download.id)
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
            }
            .padding(.horizontal, 16)
            .padding(.top, 8)

            if download.trackers.isEmpty {
                ContentUnavailableView(
                    "No Trackers",
                    systemImage: "network.slash",
                    description: Text("This torrent relies on DHT and peer exchange.")
                )
            } else {
                List(download.trackers, id: \.self) { tracker in
                    HStack {
                        Image(systemName: "antenna.radiowaves.left.and.right")
                            .foregroundStyle(.secondary)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(tracker)
                                .lineLimit(1)
                                .textSelection(.enabled)
                            Text("Announce URL")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .padding(.vertical, 4)
                }
                .listStyle(.inset)
            }
        }
    }

    private func swarmTab(_ download: Download) -> some View {
        Form {
            Section("Swarm Activity") {
                LabeledContent("Seeds (Uploading)", value: "\(download.seedsCount) seeds")
                LabeledContent("Peers (Downloading)", value: "\(download.peersCount) peers")
                LabeledContent("Download Rate", value: Formatters.speedString(download.downloadRate))
                LabeledContent("Upload Rate", value: Formatters.speedString(download.uploadRate))
            }

            Section("Seeding Settings") {
                HStack {
                    Text("Seed Ratio Target")
                    Spacer()
                    Picker("", selection: Binding(
                        get: { download.seedRatioLimit ?? 0.0 },
                        set: { library.setSeedRatioLimit(download.id, limit: $0 == 0.0 ? nil : $0) }
                    )) {
                        Text("No limit").tag(0.0)
                        Text("0.5").tag(0.5)
                        Text("1.0 (100%)").tag(1.0)
                        Text("1.5 (150%)").tag(1.5)
                        Text("2.0 (200%)").tag(2.0)
                    }
                    .frame(width: 130)
                }
            }

            if download.isVerifying {
                Section("Piece Verification") {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Verifying local pieces…")
                            .font(.caption)
                        ProgressView(value: download.verificationProgress)
                    }
                }
            }
        }
        .formStyle(.grouped)
    }

    private var footerView: some View {
        HStack {
            Spacer()
            Button("Done") {
                dismiss()
            }
            .buttonStyle(.borderedProminent)
            .keyboardShortcut(.defaultAction)
        }
        .padding(14)
    }

    private func iconForFile(_ name: String) -> String {
        let lower = name.lowercased()
        if lower.hasSuffix(".mp4") || lower.hasSuffix(".mkv") || lower.hasSuffix(".mov") || lower.hasSuffix(".avi") {
            return "film"
        }
        if lower.hasSuffix(".mp3") || lower.hasSuffix(".m4a") || lower.hasSuffix(".flac") || lower.hasSuffix(".wav") {
            return "music.note"
        }
        if lower.hasSuffix(".jpg") || lower.hasSuffix(".png") || lower.hasSuffix(".webp") {
            return "photo"
        }
        if lower.hasSuffix(".txt") || lower.hasSuffix(".nfo") || lower.hasSuffix(".pdf") {
            return "doc.text"
        }
        return "doc"
    }
}
