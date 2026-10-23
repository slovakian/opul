import AppKit
import OpulCore
import SwiftUI

/// Introduces Opul on first launch and asks before downloading yt-dlp. Closing it counts as
/// Not Now. Help ▸ Welcome to Opul opens it again with the current yt-dlp state.
struct WelcomeSheet: View {
    @Environment(\.dismiss) private var dismiss
    let library: DownloadLibrary
    @State private var chooserError: String?

    private var status: MediaToolStatus? { library.mediaToolStatus }

    private enum VideoState {
        case offer
        case downloading(Double)
        case failed(String)
        case ready(String)
        case custom(String)
        case customBroken(String)
    }

    private var videoState: VideoState {
        guard let status else { return .offer }
        switch status.ytDlp {
        case .custom(let path, _): return .custom(path)
        case .customBroken(let path, _): return .customBroken(path)
        case .installed(let version, _): return .ready(version)
        case .failed(let reason):
            if let progress = status.downloadProgress { return .downloading(progress) }
            return status.ytDlpPending ? .downloading(0) : .failed(reason)
        case .notInstalled:
            if let progress = status.downloadProgress { return .downloading(progress) }
            return status.ytDlpPending ? .downloading(0) : .offer
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 22) {
            header
            features
            Divider()
            videoDownloads
            footer
        }
        .padding(28)
        .frame(width: 540)
        .onAppear(perform: library.requestYtDlpDownloadSize)
    }

    private var header: some View {
        HStack(spacing: 16) {
            Image(nsImage: NSApp.applicationIconImage)
                .resizable()
                .frame(width: 64, height: 64)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 4) {
                Text("Welcome to Opul")
                    .font(.largeTitle.weight(.semibold))
                    .accessibilityAddTraits(.isHeader)
                Text("Downloads of every kind, kept in order.")
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var features: some View {
        VStack(alignment: .leading, spacing: 12) {
            FeatureRow(symbol: "doc.on.clipboard", text: "Paste any link: ⌘V anywhere in the window.")
            FeatureRow(symbol: "play.rectangle.on.rectangle", text: "Video pages and playlists from thousands of sites.")
            FeatureRow(symbol: "arrow.down.circle", text: "Torrents and magnet links, built in.")
            FeatureRow(symbol: "square.stack.3d.up", text: "Workspaces and saved folders keep downloads sorted.")
            FeatureRow(symbol: "safari", text: "Browse for Media finds files on pages that hide them.")
        }
    }

    @ViewBuilder
    private var videoDownloads: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Video downloads")
                .font(.headline)
                .accessibilityAddTraits(.isHeader)
            switch videoState {
            case .offer:
                Text("Video pages need yt-dlp, a free tool Opul downloads from GitHub. Nothing is downloaded until you choose to.")
                    .fixedSize(horizontal: false, vertical: true)
            case .downloading(let progress):
                Text("Downloading yt-dlp. You can close this window; the download carries on.")
                    .fixedSize(horizontal: false, vertical: true)
                ProgressView(value: progress)
                    .accessibilityLabel("yt-dlp download")
                    .accessibilityValue(progress.formatted(.percent.precision(.fractionLength(0))))
            case .failed(let reason):
                Text("Opul couldn't download yt-dlp.")
                Text(reason)
                    .font(.callout)
                    .foregroundStyle(.red)
                    .fixedSize(horizontal: false, vertical: true)
            case .ready(let version):
                Label("yt-dlp \(version) is ready. Paste a video link to start.", systemImage: "checkmark.circle.fill")
                    .foregroundStyle(.green)
            case .custom(let path):
                Text("Using your yt-dlp at \(path).")
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
            case .customBroken(let path):
                Text("Your yt-dlp at \(path) isn't working. Opul can download its own copy instead.")
                    .foregroundStyle(.red)
                    .fixedSize(horizontal: false, vertical: true)
            }
            if showsAutoUpdate {
                Toggle("Keep yt-dlp up to date automatically", isOn: Binding(
                    get: { library.preferences.ytDlpAutoUpdate },
                    set: { library.preferences.ytDlpAutoUpdate = $0 }
                ))
                .toggleStyle(.checkbox)
            }
            if showsChooser {
                Button("Use my own yt-dlp…", action: chooseYtDlp)
                    .buttonStyle(.link)
                    .font(.callout)
            }
            if let chooserError {
                Text(chooserError)
                    .font(.callout)
                    .foregroundStyle(.red)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    @ViewBuilder
    private var footer: some View {
        HStack {
            Spacer()
            switch videoState {
            case .offer, .failed, .customBroken:
                Button("Not Now") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button(downloadTitle) {
                    if case .customBroken = videoState {
                        library.useOpulsYtDlp()
                    }
                    library.downloadYtDlp()
                }
                .keyboardShortcut(.defaultAction)
                .buttonStyle(.borderedProminent)
            case .downloading:
                Button("Close") { dismiss() }
                    .keyboardShortcut(.defaultAction)
                    .background(escapeCloses)
            case .ready, .custom:
                Button("Done") { dismiss() }
                    .keyboardShortcut(.defaultAction)
                    .background(escapeCloses)
            }
        }
    }

    /// A button can carry one shortcut, so Esc gets an invisible one of its own.
    private var escapeCloses: some View {
        Button("Close Welcome") { dismiss() }
            .keyboardShortcut(.cancelAction)
            .opacity(0)
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
    }

    private var downloadTitle: String {
        let size = status?.downloadSizeText.map { " (\($0))" } ?? ""
        switch videoState {
        case .failed: return "Try Again\(size)"
        case .customBroken: return "Download Opul's yt-dlp\(size)"
        default: return "Download yt-dlp\(size)"
        }
    }

    private var showsAutoUpdate: Bool {
        switch videoState {
        case .custom, .customBroken: false
        default: true
        }
    }

    private var showsChooser: Bool {
        switch videoState {
        case .offer, .failed, .customBroken: true
        default: false
        }
    }

    private func chooseYtDlp() {
        guard let url = ToolFilePicker.choose("Choose yt-dlp") else { return }
        Task { chooserError = await library.useCustomYtDlp(at: url) }
    }
}

private struct FeatureRow: View {
    let symbol: String
    let text: String

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: symbol)
                .font(.title3)
                .foregroundStyle(Color.accentColor)
                .frame(width: 28)
                .accessibilityHidden(true)
            Text(text)
        }
        .accessibilityElement(children: .combine)
    }
}
