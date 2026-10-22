import Foundation
import OpulCore

enum SidebarSelection: Hashable {
    case workspace(Workspace.ID)
    case folder(workspace: Workspace.ID, folder: SavedFolder.ID)

    var workspaceID: Workspace.ID {
        switch self {
        case .workspace(let id): id
        case .folder(let id, _): id
        }
    }

    var folderID: SavedFolder.ID? {
        if case .folder(_, let folderID) = self { return folderID }
        return nil
    }
}

struct SidebarEntry: Identifiable {
    let selection: SidebarSelection
    let title: String
    let symbolName: String?
    let emoji: String?
    let isFolder: Bool
    let isDefault: Bool
    let finishedCount: Int
    let path: String?
    let workspaceID: Workspace.ID
    let folderID: SavedFolder.ID?
    var id: SidebarSelection { selection }
}

enum LibrarySheet: Identifiable, Equatable {
    case create
    case guided
    case settings(Workspace.ID)

    var id: String {
        switch self {
        case .create:
            "create"
        case .guided:
            "guided"
        case .settings(let workspaceID):
            "settings-\(workspaceID.uuidString)"
        }
    }

    var workspaceID: Workspace.ID? {
        if case .settings(let workspaceID) = self { return workspaceID }
        return nil
    }

    var title: String {
        switch self {
        case .create: "New Workspace"
        case .guided: "Guided Extraction"
        case .settings: "Workspace Settings"
        }
    }

    var confirmTitle: String {
        switch self {
        case .create: "Create"
        case .guided: "Done"
        case .settings: "Save"
        }
    }
}
struct DeleteRequest: Equatable {
    let workspaceID: Workspace.ID
    let title: String
    let message: String
}

enum FolderChoice: Equatable {
    case add(Workspace.ID)
    case addAsDefault(Workspace.ID)
    case addAndMove(workspace: Workspace.ID, downloads: [Download.ID])
    case replace(workspace: Workspace.ID, folder: SavedFolder.ID)
}

struct DownloadRemovalRequest: Equatable {
    let downloadIDs: [Download.ID]
}

struct FolderRemovalRequest: Equatable {
    let workspaceID: Workspace.ID
    let folderID: SavedFolder.ID
    let folderName: String
    let downloadIDs: [Download.ID]
}

struct FolderReplacementRequest: Equatable {
    let workspaceID: Workspace.ID
    let folderID: SavedFolder.ID
    let folderName: String
    let replacementName: String
    let replacementPath: String
    let downloadCount: Int
    let canRelink: Bool
}

struct PasteNotice: Equatable {
    var text: String
    var isWarning: Bool
}

struct MediaImportRequest: Identifiable {
    let id = UUID()
    let workspaceID: Workspace.ID
    let folderID: SavedFolder.ID
    let probe: MediaProbe
}

enum DownloadFileOrganization: String, CaseIterable, Identifiable, Equatable, Codable, Sendable {
    case automatic
    case alwaysCreateFolder
    case keepInSaveLocation

    var id: Self { self }

    var title: String {
        switch self {
        case .automatic: "Automatic (Recommended)"
        case .alwaysCreateFolder: "Always Create a Folder"
        case .keepInSaveLocation: "Keep Files in the Save Location"
        }
    }

    var explanation: String {
        switch self {
        case .automatic:
            "Single files stay in the save location. Downloads with multiple or companion files get a named folder."
        case .alwaysCreateFolder:
            "Every download gets its own named folder."
        case .keepInSaveLocation:
            "All files are placed directly in the save location. Files from different downloads may be mixed together."
        }
    }

    var contentLayout: DownloadContentLayout {
        switch self {
        case .automatic: .original
        case .alwaysCreateFolder: .subfolder
        case .keepInSaveLocation: .noSubfolder
        }
    }
}

struct Workspace: Identifiable, Equatable, Codable {
    let id: UUID
    var name: String
    var icon: WorkspaceIcon
    var savedFolders: [SavedFolder]
    var defaultFormat: Format
    var fileOrganizationOverride: DownloadFileOrganization? = nil
    var view: WorkspaceViewSettings
    var downloads: [Download]

    init(
        id: UUID = UUID(),
        name: String,
        icon: WorkspaceIcon,
        savedFolders: [SavedFolder],
        defaultFormat: Format,
        view: WorkspaceViewSettings = WorkspaceViewSettings(),
        downloads: [Download] = []
    ) {
        self.id = id
        self.name = name
        self.icon = icon
        self.savedFolders = savedFolders
        self.defaultFormat = defaultFormat
        self.view = view
        self.downloads = downloads
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.id = try container.decode(UUID.self, forKey: .id)
        self.name = try container.decode(String.self, forKey: .name)
        self.icon = try container.decode(WorkspaceIcon.self, forKey: .icon)
        self.savedFolders = try container.decode([SavedFolder].self, forKey: .savedFolders)
        self.defaultFormat = try container.decode(Format.self, forKey: .defaultFormat)
        self.view = try container.decode(WorkspaceViewSettings.self, forKey: .view)
        self.downloads = try container.decode([Download].self, forKey: .downloads)
    }
}

enum WorkspaceIcon: Equatable, Hashable, Codable {
    case symbol(String)
    case emoji(String)

    var symbolName: String? {
        if case .symbol(let name) = self { return name }
        return nil
    }

    var emoji: String? {
        if case .emoji(let value) = self { return value }
        return nil
    }

}

struct SidebarSymbol: Identifiable, Hashable {
    let name: String
    let title: String
    var id: String { name }

    static let choices: [SidebarSymbol] = [
        SidebarSymbol(name: "arrow.down.circle", title: "Downloads"),
        SidebarSymbol(name: "film", title: "Film"),
        SidebarSymbol(name: "music.note", title: "Music"),
        SidebarSymbol(name: "graduationcap", title: "Study"),
        SidebarSymbol(name: "book", title: "Book"),
        SidebarSymbol(name: "headphones", title: "Audio"),
        SidebarSymbol(name: "star", title: "Star"),
        SidebarSymbol(name: "folder", title: "Folder"),
    ]
}

struct SavedFolder: Identifiable, Equatable, Hashable, Codable {
    let id: UUID
    var name: String
    var path: String
    var isDefault: Bool

    init(
        id: UUID = UUID(),
        name: String,
        path: String,
        isDefault: Bool = false
    ) {
        self.id = id
        self.name = name
        self.path = path
        self.isDefault = isDefault
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.id = try container.decode(UUID.self, forKey: .id)
        self.name = try container.decode(String.self, forKey: .name)
        self.path = try container.decode(String.self, forKey: .path)
        self.isDefault = try container.decode(Bool.self, forKey: .isDefault)
    }
}

struct Download: Identifiable, Equatable, Codable {
    let id: UUID
    var name: String
    var address: String
    var pageURL: String? = nil
    var state: DownloadState
    var source: Source
    var kind: Kind?
    var format: Format?
    var folderID: SavedFolder.ID
    var infoHash: String?
    var totalSize: Int64 = 0
    var bytesDownloaded: Int64 = 0
    var bytesUploaded: Int64 = 0
    var downloadRate: Int64 = 0
    var uploadRate: Int64 = 0
    var peersCount: Int = 0
    var seedsCount: Int = 0
    var seedRatio: Double = 0
    var seedRatioLimit: Double? = 1
    var etaSeconds: Int?
    var pieceLength: Int64 = 0
    var pieceCount: Int = 0
    var comment: String?
    var trackers: [String] = []
    var files: [TorrentFileItem] = []
    var isVerifying: Bool = false
    var verificationProgress: Double = 0
    var transferID: String? = nil
    var progress: Double = 0
    var outputPath: String? = nil
    var contentFolder: String? = nil
    var thumbnailURL: String? = nil
    var extractor: String? = nil
    var errorMessage: String? = nil
    var playlistIndex: Int? = nil
    var capabilities = DownloadCapabilities.none

    init(
        id: UUID = UUID(),
        name: String,
        address: String,
        pageURL: String? = nil,
        state: DownloadState = .queued,
        source: Source = .fileURL,
        kind: Kind? = nil,
        format: Format? = nil,
        folderID: SavedFolder.ID,
        infoHash: String? = nil,
        totalSize: Int64 = 0,
        bytesDownloaded: Int64 = 0,
        bytesUploaded: Int64 = 0,
        downloadRate: Int64 = 0,
        uploadRate: Int64 = 0,
        peersCount: Int = 0,
        seedsCount: Int = 0,
        seedRatio: Double = 0,
        seedRatioLimit: Double? = 1,
        etaSeconds: Int? = nil,
        pieceLength: Int64 = 0,
        pieceCount: Int = 0,
        comment: String? = nil,
        trackers: [String] = [],
        files: [TorrentFileItem] = [],
        isVerifying: Bool = false,
        verificationProgress: Double = 0,
        transferID: String? = nil,
        progress: Double = 0,
        outputPath: String? = nil,
        contentFolder: String? = nil,
        thumbnailURL: String? = nil,
        extractor: String? = nil,
        errorMessage: String? = nil,
        playlistIndex: Int? = nil,
        capabilities: DownloadCapabilities = .none
    ) {
        self.id = id
        self.name = name
        self.address = address
        self.pageURL = pageURL
        self.state = state
        self.source = source
        self.kind = kind
        self.format = format
        self.folderID = folderID
        self.infoHash = infoHash
        self.totalSize = totalSize
        self.bytesDownloaded = bytesDownloaded
        self.bytesUploaded = bytesUploaded
        self.downloadRate = downloadRate
        self.uploadRate = uploadRate
        self.peersCount = peersCount
        self.seedsCount = seedsCount
        self.seedRatio = seedRatio
        self.seedRatioLimit = seedRatioLimit
        self.etaSeconds = etaSeconds
        self.pieceLength = pieceLength
        self.pieceCount = pieceCount
        self.comment = comment
        self.trackers = trackers
        self.files = files
        self.isVerifying = isVerifying
        self.verificationProgress = verificationProgress
        self.transferID = transferID
        self.progress = progress
        self.outputPath = outputPath
        self.contentFolder = contentFolder
        self.thumbnailURL = thumbnailURL
        self.extractor = extractor
        self.errorMessage = errorMessage
        self.playlistIndex = playlistIndex
        self.capabilities = capabilities
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.id = try container.decode(UUID.self, forKey: .id)
        self.name = try container.decode(String.self, forKey: .name)
        self.address = try container.decode(String.self, forKey: .address)
        self.pageURL = try container.decodeIfPresent(String.self, forKey: .pageURL)
        self.state = try container.decode(DownloadState.self, forKey: .state)
        self.source = try container.decode(Source.self, forKey: .source)
        self.kind = try container.decodeIfPresent(Kind.self, forKey: .kind)
        self.format = try container.decodeIfPresent(Format.self, forKey: .format)
        self.folderID = try container.decode(SavedFolder.ID.self, forKey: .folderID)
        self.infoHash = try container.decodeIfPresent(String.self, forKey: .infoHash)
        self.totalSize = try container.decodeIfPresent(Int64.self, forKey: .totalSize) ?? 0
        self.bytesDownloaded = try container.decodeIfPresent(Int64.self, forKey: .bytesDownloaded) ?? 0
        self.bytesUploaded = try container.decodeIfPresent(Int64.self, forKey: .bytesUploaded) ?? 0
        self.downloadRate = try container.decodeIfPresent(Int64.self, forKey: .downloadRate) ?? 0
        self.uploadRate = try container.decodeIfPresent(Int64.self, forKey: .uploadRate) ?? 0
        self.peersCount = try container.decodeIfPresent(Int.self, forKey: .peersCount) ?? 0
        self.seedsCount = try container.decodeIfPresent(Int.self, forKey: .seedsCount) ?? 0
        self.seedRatio = try container.decodeIfPresent(Double.self, forKey: .seedRatio) ?? 0
        self.seedRatioLimit = try container.decodeIfPresent(Double.self, forKey: .seedRatioLimit) ?? 1
        self.etaSeconds = try container.decodeIfPresent(Int.self, forKey: .etaSeconds)
        self.pieceLength = try container.decodeIfPresent(Int64.self, forKey: .pieceLength) ?? 0
        self.pieceCount = try container.decodeIfPresent(Int.self, forKey: .pieceCount) ?? 0
        self.comment = try container.decodeIfPresent(String.self, forKey: .comment)
        self.trackers = try container.decodeIfPresent([String].self, forKey: .trackers) ?? []
        self.files = try container.decodeIfPresent([TorrentFileItem].self, forKey: .files) ?? []
        self.isVerifying = try container.decodeIfPresent(Bool.self, forKey: .isVerifying) ?? false
        self.verificationProgress = try container.decodeIfPresent(Double.self, forKey: .verificationProgress) ?? 0
        self.transferID = try container.decodeIfPresent(String.self, forKey: .transferID)
        self.progress = try container.decodeIfPresent(Double.self, forKey: .progress) ?? 0
        self.outputPath = try container.decodeIfPresent(String.self, forKey: .outputPath)
        self.thumbnailURL = try container.decodeIfPresent(String.self, forKey: .thumbnailURL)
        self.extractor = try container.decodeIfPresent(String.self, forKey: .extractor)
        self.errorMessage = try container.decodeIfPresent(String.self, forKey: .errorMessage)
        self.playlistIndex = try container.decodeIfPresent(Int.self, forKey: .playlistIndex)
        self.capabilities = try container.decodeIfPresent(
            DownloadCapabilities.self,
            forKey: .capabilities
        ) ?? .none
    }

    /// Bytes Opul will transfer. Skipped files stay out of this total.
    var effectiveSize: Int64 {
        let selected = files.filter(\.isIncluded).reduce(Int64(0)) { $0 + $1.size }
        if files.isEmpty { return totalSize }
        return selected
    }

    var progressFraction: Double? {
        if progress.isFinite, progress > 0 {
            return min(progress, 1)
        }
        if effectiveSize > 0 {
            let fraction = Double(bytesDownloaded) / Double(effectiveSize)
            return min(max(fraction, 0), 1)
        }
        return nil
    }

    var progressPercentage: Int? {
        progressFraction.map { Int($0 * 100) }
    }

    /// The file to open, preview, or copy the path of: the main file, else the download's folder.
    var openableURL: URL? {
        (outputPath ?? contentFolder).map { URL(fileURLWithPath: $0) }
    }

    var magnetLink: String? {
        if address.lowercased().hasPrefix("magnet:") { return address }
        guard let infoHash else { return nil }
        return createMagnetUri(infoHash: infoHash, name: name, trackers: trackers)
    }

    /// What Copy Link puts on the clipboard. Pasting it back into Opul adds the same download.
    var shareableLink: String? {
        if source == .torrent {
            return magnetLink ?? (address.lowercased().hasPrefix("http") ? address : nil)
        }
        return address.isEmpty ? nil : address
    }

    /// The web page the media came from, when it differs from the link itself:
    /// the playlist of a video page, or the page a guided extraction ran on.
    var distinctPageLink: String? {
        guard source != .torrent, let pageURL, !pageURL.isEmpty, pageURL != address else { return nil }
        return pageURL
    }

    /// The page to open in a browser. A video page is its own link; a guided download's
    /// link is the raw media, so its page is the one it was found on.
    var webPageURL: URL? {
        let page: String? = switch source {
        case .videoPage: address
        case .guided: pageURL
        case .fileURL, .torrent: nil
        }
        guard let page, let url = URL(string: page),
              let scheme = url.scheme?.lowercased(), scheme == "http" || scheme == "https" else { return nil }
        return url
    }

    func detail(folderName: String) -> String {
        var parts = [kind?.title ?? "Kind not known yet"]
        if let format {
            parts.append(format.title)
        }
        if let progressPercentage {
            parts.append("\(progressPercentage)%")
        }
        if downloadRate > 0 {
            parts.append(Formatters.speedString(downloadRate))
        }
        if let etaSeconds, etaSeconds > 0 {
            parts.append("ETA \(Formatters.durationString(etaSeconds))")
        }
        parts.append(folderName)
        parts.append(source.title)
        return parts.joined(separator: " · ")
    }
}

/// Moving a finished download's files into another saved folder.
enum DownloadMove {
    enum Failure: LocalizedError {
        case nameTaken(String, folder: String)
        case outsideOwnFolder(String)

        var errorDescription: String? {
            switch self {
            case .nameTaken(let name, let folder):
                "“\(name)” already exists in \(folder), so it was not moved."
            case .outsideOwnFolder(let name):
                "Opul could not tell which files belong to “\(name)”, so it was not moved."
            }
        }
    }

    /// Where `path` ends up when `source`, the file or folder that contains it, moves to
    /// `destination`. Nil when `path` is not inside `source`.
    static func rebasedPath(_ path: String, from source: URL, to destination: URL) -> String? {
        let file = URL(fileURLWithPath: path).standardizedFileURL.pathComponents
        let base = source.standardizedFileURL.pathComponents
        guard file.starts(with: base) else { return nil }
        return file.dropFirst(base.count)
            .reduce(destination) { $0.appending(path: $1) }
            .path(percentEncoded: false)
    }
}

/// The downloads selected in a Workspace list, with the anchor a Shift-click extends from.
struct DownloadSelection: Equatable {
    var ids = Set<Download.ID>()
    private(set) var anchor: Download.ID?

    /// Finder-style clicks. `order` is the list as displayed, for Shift-click ranges.
    mutating func click(
        _ id: Download.ID,
        extending: Bool,
        toggling: Bool,
        in order: [Download.ID]
    ) {
        if extending,
           let anchor,
           let start = order.firstIndex(of: anchor),
           let end = order.firstIndex(of: id) {
            let range = order[min(start, end)...max(start, end)]
            ids = toggling ? ids.union(range) : Set(range)
            return
        }
        if toggling {
            if ids.contains(id) {
                ids.remove(id)
            } else {
                ids.insert(id)
            }
        } else {
            ids = [id]
        }
        anchor = id
    }
}

struct DownloadCapabilities: Equatable, Codable {
    var canPause: Bool
    var canResume: Bool
    var canRetry: Bool
    var canCancel: Bool
    var canStopSeeding: Bool
    var canChangeFileSelection: Bool
    var canRemove: Bool
    var canRevealOutput: Bool
    var canMove: Bool

    static let none = DownloadCapabilities(
        canPause: false,
        canResume: false,
        canRetry: false,
        canCancel: false,
        canStopSeeding: false,
        canChangeFileSelection: false,
        canRemove: false,
        canRevealOutput: false,
        canMove: false
    )
}

enum DownloadState: String, CaseIterable, Equatable, Codable {
    case queued
    case fetching
    case processing
    case paused
    case seeding
    case finished
    case failed
    case cancelled

    var title: String {
        switch self {
        case .queued: "Pending"
        case .fetching: "In progress"
        case .processing: "Finishing"
        case .paused: "Paused"
        case .seeding: "Sharing"
        case .finished: "Complete"
        case .failed: "Needs attention"
        case .cancelled: "Stopped"
        }
    }

    var symbolName: String {
        switch self {
        case .queued: "clock"
        case .fetching: "arrow.down.circle"
        case .processing: "gearshape.2"
        case .paused: "pause.circle"
        case .seeding: "arrow.up.circle"
        case .finished: "checkmark.circle"
        case .failed: "exclamationmark.circle"
        case .cancelled: "xmark.circle"
        }
    }

    var sortIndex: Int {
        Self.allCases.firstIndex(of: self) ?? 0
    }
}

enum Source: String, CaseIterable, Equatable, Codable {
    case videoPage
    case fileURL
    case guided
    case torrent

    var title: String {
        switch self {
        case .videoPage: "Video page"
        case .fileURL: "File URL"
        case .guided: "Guided extraction"
        case .torrent: "Torrent"
        }
    }
}

enum Kind: String, CaseIterable, Equatable, Codable {
    case video
    case audio
    case file

    var title: String {
        switch self {
        case .video: "Video"
        case .audio: "Audio"
        case .file: "File"
        }
    }
}

enum Format: String, CaseIterable, Identifiable, Equatable, Codable {
    case best
    case bestMp4
    case p1080
    case p720
    case original
    case audioMp3
    case audioM4a

    var id: String { rawValue }

    var title: String {
        switch self {
        case .best: "Best"
        case .bestMp4: "Best MP4"
        case .p1080: "1080p"
        case .p720: "720p"
        case .original: "Original"
        case .audioMp3: "Audio MP3"
        case .audioM4a: "Audio M4A"
        }
    }

    var isAudioOnly: Bool {
        self == .audioMp3 || self == .audioM4a
    }

    var mediaFormat: MediaFormat {
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

    init(_ mediaFormat: MediaFormat) {
        self = switch mediaFormat {
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

enum DownloadLayout: String, CaseIterable, Equatable, Codable {
    case list
    case table

    var title: String {
        switch self {
        case .list: "List"
        case .table: "Table"
        }
    }

    var symbolName: String {
        switch self {
        case .list: "list.bullet"
        case .table: "tablecells"
        }
    }
}

enum DownloadSort: String, CaseIterable, Equatable, Codable {
    case order
    case name
    case state
    case source

    var title: String {
        switch self {
        case .order: "Date Added"
        case .name: "Name"
        case .state: "State"
        case .source: "Source"
        }
    }

    var symbolName: String {
        switch self {
        case .order: "calendar.badge.plus"
        case .name: "textformat"
        case .state: "circle.dotted"
        case .source: "link"
        }
    }
}

enum DownloadGrouping: String, CaseIterable, Equatable, Codable {
    case none
    case state
    case folder
    case kind
    case source

    var title: String {
        switch self {
        case .none: "None"
        case .state: "State"
        case .folder: "Folder"
        case .kind: "Kind"
        case .source: "Source"
        }
    }

    var symbolName: String {
        switch self {
        case .none: "rectangle.dashed"
        case .state: "circle.dotted"
        case .folder: "folder"
        case .kind: "square.grid.2x2"
        case .source: "link"
        }
    }
}

struct WorkspaceViewSettings: Equatable, Codable {
    var layout: DownloadLayout
    var sort: DownloadSort
    var grouping: DownloadGrouping

    init(
        layout: DownloadLayout = .list,
        sort: DownloadSort = .order,
        grouping: DownloadGrouping = .none
    ) {
        self.layout = layout
        self.sort = sort
        self.grouping = grouping
    }

}

struct DownloadArrangement: Equatable {
    struct Section: Identifiable, Equatable {
        let id: String
        let title: String
        let lines: [Line]
    }

    struct Line: Identifiable, Equatable {
        let download: Download
        let folderName: String
        let groupTitle: String
        var id: Download.ID { download.id }
    }

    var sections: [Section]
    var showsHeaders: Bool

    var lines: [Line] {
        sections.flatMap(\.lines)
    }

    static func make(
        downloads: [Download],
        folders: [SavedFolder],
        settings: WorkspaceViewSettings
    ) -> DownloadArrangement {
        let groups: [(id: String, title: String, items: [Download])]
        if settings.grouping == .none {
            groups = [("all", "", ordered(downloads, sort: settings.sort))]
        } else {
            groups = grouped(downloads, by: settings.grouping, folders: folders).map { group in
                (group.id, group.title, ordered(group.items, sort: settings.sort))
            }
        }
        let sections = groups.map { group in
            Section(
                id: group.id,
                title: group.title,
                lines: group.items.map { download in
                    Line(
                        download: download,
                        folderName: folders.first { $0.id == download.folderID }?.name ?? "Folder",
                        groupTitle: group.title
                    )
                }
            )
        }
        return DownloadArrangement(sections: sections, showsHeaders: settings.grouping != .none)
    }

    private static func ordered(_ downloads: [Download], sort: DownloadSort) -> [Download] {
        switch sort {
        case .order:
            // Downloads are appended to their workspace as they are added.
            // Presenting that storage order in reverse keeps the newest item first.
            return Array(downloads.reversed())
        case .name:
            return downloads.sorted { lhs, rhs in
                let order = lhs.name.localizedStandardCompare(rhs.name)
                if order == .orderedSame { return lhs.id.uuidString < rhs.id.uuidString }
                return order == .orderedAscending
            }
        case .state:
            return downloads.sorted { lhs, rhs in
                if lhs.state.sortIndex != rhs.state.sortIndex {
                    return lhs.state.sortIndex < rhs.state.sortIndex
                }
                return lhs.name.localizedStandardCompare(rhs.name) == .orderedAscending
            }
        case .source:
            return downloads.sorted { lhs, rhs in
                let order = lhs.source.title.localizedStandardCompare(rhs.source.title)
                if order == .orderedSame {
                    return lhs.name.localizedStandardCompare(rhs.name) == .orderedAscending
                }
                return order == .orderedAscending
            }
        }
    }

    private static func grouped(
        _ downloads: [Download],
        by grouping: DownloadGrouping,
        folders: [SavedFolder]
    ) -> [(id: String, title: String, items: [Download])] {
        let keys: [(id: String, title: String)] = switch grouping {
        case .none:
            []
        case .state:
            DownloadState.allCases.map { ($0.rawValue, $0.title) }
        case .folder:
            folders.map { ($0.id.uuidString, $0.name) }
        case .kind:
            Kind.allCases.map { ($0.rawValue, $0.title) } + [("none", "Kind not known yet")]
        case .source:
            Source.allCases.map { ($0.rawValue, $0.title) }
        }
        return keys.compactMap { key in
            let items = downloads.filter { keyID(for: $0, grouping: grouping) == key.id }
            if items.isEmpty { return nil }
            return (key.id, key.title, items)
        }
    }

    private static func keyID(for download: Download, grouping: DownloadGrouping) -> String {
        switch grouping {
        case .none:
            "all"
        case .state:
            download.state.rawValue
        case .folder:
            download.folderID.uuidString
        case .kind:
            download.kind?.rawValue ?? "none"
        case .source:
            download.source.rawValue
        }
    }
}
