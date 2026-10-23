import Foundation
import SwiftUI
import OpulCore

/// One file within a multi-file torrent.
public struct TorrentFileItem: Identifiable, Equatable, Hashable, Sendable, Codable {
    public let id: UUID
    public let fileIndex: Int
    public var path: String
    public var name: String
    public var size: Int64
    public var bytesCompleted: Int64
    public var isIncluded: Bool
    public var priority: TorrentFilePriority

    public init(
        id: UUID = UUID(),
        fileIndex: Int,
        path: String,
        name: String,
        size: Int64,
        bytesCompleted: Int64 = 0,
        isIncluded: Bool = true,
        priority: TorrentFilePriority = .normal
    ) {
        self.id = id
        self.fileIndex = fileIndex
        self.path = path
        self.name = name
        self.size = size
        self.bytesCompleted = bytesCompleted
        self.isIncluded = isIncluded
        self.priority = priority
    }

    public var progress: Double {
        guard size > 0 else { return 1.0 }
        return min(1.0, Double(bytesCompleted) / Double(size))
    }

    public var formattedSize: String {
        ByteCountFormatter.string(fromByteCount: size, countStyle: .file)
    }

    public var formattedCompleted: String {
        ByteCountFormatter.string(fromByteCount: bytesCompleted, countStyle: .file)
    }
}

public enum TorrentFilePriority: String, CaseIterable, Identifiable, Equatable, Sendable, Codable {
    case normal = "Normal"
    case high = "High"
    case low = "Low"
    case skip = "Skip"

    public var id: String { rawValue }

    public var symbolName: String {
        switch self {
        case .normal: "equal"
        case .high: "chevron.up"
        case .low: "chevron.down"
        case .skip: "xmark"
        }
    }
}

/// Global preferences for media downloads, torrenting, and bandwidth.
public struct TorrentPreferences: Equatable, Sendable, Codable {
    public var defaultSeedRatio: Double = 1.0
    public var autoStopSeeding: Bool = true
    public var listenPort: Int = 6881
    public var randomizePort: Bool = false
    public var enableDHT: Bool = true
    public var maxConnections: Int = 200
    public var maxDownloadRateKBps: Int = 0 // 0 = unlimited
    public var maxUploadRateKBps: Int = 0   // 0 = unlimited
    public var maxConcurrentMediaDownloads: Int = 3
    var fileOrganization: DownloadFileOrganization = .automatic
    public var downloadSubtitles: Bool = true
    public var subtitleLanguages: String = "en.*,en"
    public var embedSubtitles: Bool = false
    public var writeThumbnail: Bool = true
    public var writeMetadata: Bool = true
    public var filenameTemplate: String = "%(title).180B [%(id)s].%(ext)s"
    /// A yt-dlp the person chose. Empty means Opul's own copy.
    public var ytDlpPath: String = ""
    /// An FFmpeg the person chose. Empty means the bundled one.
    public var ffmpegPath: String = ""
    public var ytDlpAutoUpdate: Bool = true
    public var hasSeenWelcome: Bool = false
    var interactionSounds = InteractionSoundPreferences()

    public init() {}

    public init(from decoder: Decoder) throws {
        self.init()
        let values = try decoder.container(keyedBy: CodingKeys.self)
        defaultSeedRatio = try values.decodeIfPresent(Double.self, forKey: .defaultSeedRatio) ?? defaultSeedRatio
        autoStopSeeding = try values.decodeIfPresent(Bool.self, forKey: .autoStopSeeding) ?? autoStopSeeding
        listenPort = try values.decodeIfPresent(Int.self, forKey: .listenPort) ?? listenPort
        randomizePort = try values.decodeIfPresent(Bool.self, forKey: .randomizePort) ?? randomizePort
        enableDHT = try values.decodeIfPresent(Bool.self, forKey: .enableDHT) ?? enableDHT
        maxConnections = try values.decodeIfPresent(Int.self, forKey: .maxConnections) ?? maxConnections
        maxDownloadRateKBps = try values.decodeIfPresent(Int.self, forKey: .maxDownloadRateKBps) ?? maxDownloadRateKBps
        maxUploadRateKBps = try values.decodeIfPresent(Int.self, forKey: .maxUploadRateKBps) ?? maxUploadRateKBps
        maxConcurrentMediaDownloads = try values.decodeIfPresent(Int.self, forKey: .maxConcurrentMediaDownloads) ?? maxConcurrentMediaDownloads
        fileOrganization = try values.decodeIfPresent(DownloadFileOrganization.self, forKey: .fileOrganization) ?? fileOrganization
        downloadSubtitles = try values.decodeIfPresent(Bool.self, forKey: .downloadSubtitles) ?? downloadSubtitles
        subtitleLanguages = try values.decodeIfPresent(String.self, forKey: .subtitleLanguages) ?? subtitleLanguages
        embedSubtitles = try values.decodeIfPresent(Bool.self, forKey: .embedSubtitles) ?? embedSubtitles
        writeThumbnail = try values.decodeIfPresent(Bool.self, forKey: .writeThumbnail) ?? writeThumbnail
        writeMetadata = try values.decodeIfPresent(Bool.self, forKey: .writeMetadata) ?? writeMetadata
        filenameTemplate = try values.decodeIfPresent(String.self, forKey: .filenameTemplate) ?? filenameTemplate
        ytDlpPath = try values.decodeIfPresent(String.self, forKey: .ytDlpPath) ?? ytDlpPath
        ffmpegPath = try values.decodeIfPresent(String.self, forKey: .ffmpegPath) ?? ffmpegPath
        ytDlpAutoUpdate = try values.decodeIfPresent(Bool.self, forKey: .ytDlpAutoUpdate) ?? ytDlpAutoUpdate
        hasSeenWelcome = try values.decodeIfPresent(Bool.self, forKey: .hasSeenWelcome) ?? hasSeenWelcome
        interactionSounds = try values.decodeIfPresent(InteractionSoundPreferences.self, forKey: .interactionSounds) ?? interactionSounds
    }
}

public enum Formatters {
    public static func speedString(_ bytesPerSec: Int64) -> String {
        if bytesPerSec <= 0 { return "0 KB/s" }
        let kb = Double(bytesPerSec) / 1024.0
        if kb < 1024.0 {
            return String(format: "%.0f KB/s", kb)
        }
        let mb = kb / 1024.0
        return String(format: "%.1f MB/s", mb)
    }

    public static func byteString(_ bytes: Int64) -> String {
        ByteCountFormatter.string(fromByteCount: bytes, countStyle: .file)
    }

    public static func durationString(_ seconds: Int) -> String {
        if seconds <= 0 { return "—" }
        if seconds < 60 { return "\(seconds)s" }
        let minutes = seconds / 60
        if minutes < 60 { return "\(minutes)m" }
        let hours = minutes / 60
        let remainingMin = minutes % 60
        return "\(hours)h \(remainingMin)m"
    }
}
