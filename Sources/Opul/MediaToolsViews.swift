import AppKit
import OpulCore
import SwiftUI

/// How the tools behind video downloads read in the welcome sheet, Settings and the list.
extension MediaToolStatus {
    var downloadSizeText: String? {
        downloadBytes.map { Formatters.byteString(Int64(clamping: $0)) }
    }

    var isCustomYtDlp: Bool {
        switch ytDlp {
        case .custom, .customBroken: true
        default: false
        }
    }

    var isManagedYtDlpInstalled: Bool {
        if case .installed = ytDlp { return true }
        return false
    }

    var downloadProgress: Double? {
        if case .downloading(let progress) = activity { return progress }
        return nil
    }

    /// The status line in Settings ▸ Media.
    var ytDlpSummary: String {
        switch ytDlp {
        case .notInstalled:
            if let progress = downloadProgress {
                return "Downloading \(percent(progress))"
            }
            return activity == .checking ? "Getting ready to download…" : "Not installed"
        case .installed(let version, let lastCheckedMs):
            if let progress = downloadProgress {
                return "\(version), updating \(percent(progress))"
            }
            if activity == .checking {
                return "\(version), checking for updates…"
            }
            guard let lastCheckedMs else { return version }
            let checked = Date(timeIntervalSince1970: Double(lastCheckedMs) / 1_000)
            return "\(version), checked \(checked.formatted(.relative(presentation: .named)))"
        case .failed:
            if let progress = downloadProgress {
                return "Downloading \(percent(progress))"
            }
            if case .waitingToRetry(let seconds) = activity {
                return "Download failed, trying again in \(Formatters.durationString(Int(seconds)))"
            }
            return "Download failed"
        case .custom(let path, let version):
            return "Using your yt-dlp at \(path) (\(version ?? "checking…"))"
        case .customBroken(let path, _):
            return "Your yt-dlp at \(path) isn't working"
        }
    }

    var ytDlpProblem: String? {
        switch ytDlp {
        case .failed(let reason): reason
        case .customBroken(_, let reason): "It \(reason)."
        default: updateError
        }
    }

    private func percent(_ fraction: Double) -> String {
        fraction.formatted(.percent.precision(.fractionLength(0)))
    }
}

/// Lets a person pick a tool executable. Opul checks it before saving the path.
enum ToolFilePicker {
    static func choose(_ title: String) -> URL? {
        let panel = NSOpenPanel()
        panel.title = title
        panel.prompt = "Choose"
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.treatsFilePackagesAsDirectories = true
        return panel.runModal() == .OK ? panel.url : nil
    }
}

/// What download rows need to know about video tools.
struct VideoToolsContext {
    /// Opul is getting yt-dlp, so video links wait.
    var isGettingTools = false
    /// The person's own yt-dlp stopped working.
    var hasBrokenCustomYtDlp = false
    var downloadOpulsCopy: (Download.ID) -> Void = { _ in }
}

private struct VideoToolsContextKey: EnvironmentKey {
    static let defaultValue = VideoToolsContext()
}

extension EnvironmentValues {
    var videoTools: VideoToolsContext {
        get { self[VideoToolsContextKey.self] }
        set { self[VideoToolsContextKey.self] = newValue }
    }
}

extension Download {
    /// A video link added while Opul gets yt-dlp, which starts once yt-dlp is ready.
    func isWaitingForVideoTools(_ context: VideoToolsContext) -> Bool {
        context.isGettingTools
            && source == .videoPage
            && [.queued, .fetching].contains(state)
            && bytesDownloaded == 0
    }

    func offersOpulsYtDlp(_ context: VideoToolsContext) -> Bool {
        context.hasBrokenCustomYtDlp
            && source == .videoPage
            && state == .failed
            && errorMessage?.hasPrefix("Your yt-dlp at ") == true
    }
}

/// Settings ▸ Media ▸ yt-dlp.
struct YtDlpSettingsSection: View {
    let library: DownloadLibrary
    @State private var chooserError: String?

    private var status: MediaToolStatus? { library.mediaToolStatus }

    var body: some View {
        Section {
            LabeledContent("Status") {
                Text(status?.ytDlpSummary ?? "Not available")
                    .multilineTextAlignment(.trailing)
                    .textSelection(.enabled)
            }
            if let progress = status?.downloadProgress {
                ProgressView(value: progress)
                    .accessibilityLabel("yt-dlp download")
            }
            if let problem = status?.ytDlpProblem ?? chooserError {
                Text(problem)
                    .font(.footnote)
                    .foregroundStyle(.red)
            }
            HStack {
                buttons
            }
            Toggle("Update yt-dlp automatically", isOn: Binding(
                get: { library.preferences.ytDlpAutoUpdate },
                set: { library.preferences.ytDlpAutoUpdate = $0 }
            ))
            .disabled(status?.isCustomYtDlp == true)
        } header: {
            Text("yt-dlp")
        } footer: {
            if status?.isCustomYtDlp == true {
                Text("Opul doesn't update a yt-dlp you provide.")
            } else {
                Text("yt-dlp reads video pages. Opul downloads it from yt-dlp's GitHub releases and checks its signature.")
            }
        }
        .onAppear { library.requestYtDlpDownloadSize() }
        .confirmationDialog(
            "Remove yt-dlp?",
            isPresented: Binding(
                get: { library.isConfirmingYtDlpRemoval },
                set: { library.isConfirmingYtDlpRemoval = $0 }
            ),
            titleVisibility: .visible
        ) {
            Button("Remove yt-dlp", role: .destructive, action: library.removeYtDlp)
            Button("Cancel", role: .cancel) {
                library.isConfirmingYtDlpRemoval = false
            }
        } message: {
            let count = library.activeVideoDownloadCount
            Text("\(count == 1 ? "1 video download is" : "\(count) video downloads are") queued or running. Running downloads finish with the copy they started with; queued ones can't start until you download yt-dlp again.")
        }
    }

    @ViewBuilder
    private var buttons: some View {
        switch status?.ytDlp {
        case .installed?:
            Button("Check for Updates", action: library.checkForYtDlpUpdates)
                .disabled(status?.activity != .idle)
            Button("Remove", role: .destructive, action: library.askToRemoveYtDlp)
                .disabled(status?.downloadProgress != nil)
            chooseButton
        case .custom?, .customBroken?:
            Button("Use Opul's yt-dlp", action: library.useOpulsYtDlp)
            chooseButton
        default:
            if status?.downloadProgress == nil {
                Button(status?.downloadSizeText.map { "Download (\($0))" } ?? "Download",
                       action: library.downloadYtDlp)
                    .disabled(status?.activity == .checking)
            }
            chooseButton
        }
    }

    private var chooseButton: some View {
        Button("Choose…") {
            guard let url = ToolFilePicker.choose("Choose yt-dlp") else { return }
            Task { chooserError = await library.useCustomYtDlp(at: url) }
        }
    }
}

/// Settings ▸ Media ▸ Bundled Tools.
struct BundledToolsSection: View {
    let library: DownloadLibrary
    @State private var chooserError: String?

    private var status: MediaToolStatus? { library.mediaToolStatus }

    var body: some View {
        Section {
            toolRow("FFmpeg", status?.ffmpeg)
            toolRow("ffprobe", status?.ffprobe)
            toolRow("QuickJS", status?.quickjs)
            if let problem = status?.ffmpeg.problem ?? status?.ffprobe.problem ?? chooserError {
                Text(problem)
                    .font(.footnote)
                    .foregroundStyle(.red)
            }
            HStack {
                Button("Choose FFmpeg…") {
                    guard let url = ToolFilePicker.choose("Choose FFmpeg") else { return }
                    Task { chooserError = await library.useCustomFFmpeg(at: url) }
                }
                Button("Use Bundled", action: library.useBundledFFmpeg)
                    .disabled(library.preferences.ffmpegPath.isEmpty)
            }
        } header: {
            Text("Bundled Tools")
        } footer: {
            Text("FFmpeg merges formats, converts audio and embeds subtitles. QuickJS runs the scripts YouTube needs, without access to your files.")
        }
    }

    private func toolRow(_ name: String, _ tool: BundledToolStatus?) -> some View {
        LabeledContent(name) {
            HStack(spacing: 8) {
                Text(tool?.version ?? "Unavailable")
                    .foregroundStyle(.secondary)
                Text(tool?.custom == true ? "Custom" : "Bundled")
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Color.secondary.opacity(0.15), in: Capsule())
            }
        }
        .help(tool?.path ?? "")
        .accessibilityElement(children: .combine)
    }
}
