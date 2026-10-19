import Foundation

enum LibraryLoadOutcome: Equatable {
    case loaded([Workspace])
    case recoveredFromBackup(workspaces: [Workspace], backupURL: URL, corruptURL: URL?)
    case preservedCorrupt(corruptURL: URL, errorDescription: String)
    case unavailable(errorDescription: String)
    case notFound
}

struct LibraryEnvelope: Codable, Equatable {
    static let currentVersion = 1
    var schemaVersion: Int
    var workspaces: [Workspace]

    init(schemaVersion: Int = Self.currentVersion, workspaces: [Workspace]) {
        self.schemaVersion = schemaVersion
        self.workspaces = workspaces
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let schemaVersion = try container.decode(Int.self, forKey: .schemaVersion)
        guard schemaVersion == Self.currentVersion else {
            throw DecodingError.dataCorruptedError(
                forKey: .schemaVersion,
                in: container,
                debugDescription: "Unsupported library schema version \(schemaVersion)."
            )
        }
        self.schemaVersion = schemaVersion
        self.workspaces = try container.decode([Workspace].self, forKey: .workspaces)
    }
}

enum LibraryStorage {
    /// Loads library state from `url`.
    /// If reading or decoding fails, preserves the bad file as `...corrupt.<timestamp>.json`,
    /// attempts to restore from `backupURL`, and returns an appropriate outcome.
    static func load(from url: URL, backupURL: URL) -> LibraryLoadOutcome {
        let fm = FileManager.default
        let hasPrimary = fm.fileExists(atPath: url.path(percentEncoded: false))
        let hasBackup = fm.fileExists(atPath: backupURL.path(percentEncoded: false))

        if !hasPrimary && !hasBackup {
            return .notFound
        }

        if hasPrimary {
            do {
                let data = try Data(contentsOf: url)
                let workspaces = try decodeWorkspaces(from: data)
                return .loaded(workspaces)
            } catch {
                let corruptURL: URL
                do {
                    corruptURL = try preserveCorruptFile(at: url)
                } catch {
                    return .unavailable(
                        errorDescription: "The download library could not be loaded or safely preserved. Opul will not save library changes: \(error.localizedDescription)"
                    )
                }
                if hasBackup {
                    if let backupData = try? Data(contentsOf: backupURL),
                       let restored = try? decodeWorkspaces(from: backupData) {
                        return .recoveredFromBackup(workspaces: restored, backupURL: backupURL, corruptURL: corruptURL)
                    }
                }
                return .preservedCorrupt(corruptURL: corruptURL, errorDescription: error.localizedDescription)
            }
        }

        // Primary does not exist, but backup does
        if let backupData = try? Data(contentsOf: backupURL),
           let restored = try? decodeWorkspaces(from: backupData) {
            return .recoveredFromBackup(workspaces: restored, backupURL: backupURL, corruptURL: nil)
        }

        do {
            let corruptURL = try preserveCorruptFile(at: backupURL)
            return .preservedCorrupt(
                corruptURL: corruptURL,
                errorDescription: "The only library backup was unreadable."
            )
        } catch {
            return .unavailable(
                errorDescription: "The download library backup could not be loaded or safely preserved. Opul will not save library changes: \(error.localizedDescription)"
            )
        }
    }

    /// Attempts to decode workspaces from either the versioned LibraryEnvelope or raw [Workspace] (0.1.0 format).
    static func decodeWorkspaces(from data: Data) throws -> [Workspace] {
        let decoder = JSONDecoder()
        if let envelope = try? decoder.decode(LibraryEnvelope.self, from: data) {
            return envelope.workspaces
        }
        return try decoder.decode([Workspace].self, from: data)
    }

    /// Atomically writes workspaces to `url` using a temporary file in `folder`,
    /// keeping `backupURL` updated with the previous valid version.
    static func save(
        workspaces: [Workspace],
        to url: URL,
        backupURL: URL,
        applicationSupportFolder folder: URL
    ) throws {
        let fm = FileManager.default
        try fm.createDirectory(at: folder, withIntermediateDirectories: true)

        let envelope = LibraryEnvelope(schemaVersion: LibraryEnvelope.currentVersion, workspaces: workspaces)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(envelope)

        let tempURL = folder.appending(path: "library.json.tmp.\(UUID().uuidString)", directoryHint: .notDirectory)
        defer { try? fm.removeItem(at: tempURL) }
        try data.write(to: tempURL, options: .atomic)
        try fm.setAttributes([.posixPermissions: 0o600], ofItemAtPath: tempURL.path(percentEncoded: false))

        let primaryPath = url.path(percentEncoded: false)
        if fm.fileExists(atPath: primaryPath) {
            if let existingData = try? Data(contentsOf: url),
               (try? decodeWorkspaces(from: existingData)) != nil {
                try replaceFileAtomically(with: existingData, at: backupURL, in: folder)
            }
        }

        if fm.fileExists(atPath: primaryPath) {
            _ = try fm.replaceItemAt(url, withItemAt: tempURL)
        } else {
            try fm.moveItem(at: tempURL, to: url)
        }
        try fm.setAttributes([.posixPermissions: 0o600], ofItemAtPath: primaryPath)
    }

    /// Ensures that if `url` exists and is valid, `backupURL` exists as a copy.
    static func ensureBackup(of url: URL, backupURL: URL) {
        let fm = FileManager.default
        let primaryPath = url.path(percentEncoded: false)
        let backupPath = backupURL.path(percentEncoded: false)
        guard fm.fileExists(atPath: primaryPath), !fm.fileExists(atPath: backupPath) else { return }
        guard let data = try? Data(contentsOf: url),
              (try? decodeWorkspaces(from: data)) != nil else { return }
        _ = try? fm.copyItem(at: url, to: backupURL)
        try? fm.setAttributes([.posixPermissions: 0o600], ofItemAtPath: backupPath)
    }

    /// Preserves a corrupted file by copying it to a unique timestamped file name.
    @discardableResult
    static func preserveCorruptFile(at url: URL) throws -> URL {
        let fm = FileManager.default
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        let timestamp = formatter.string(from: Date()).replacingOccurrences(of: ":", with: "-")
        let folder = url.deletingLastPathComponent()
        let filename = "\(url.deletingPathExtension().lastPathComponent).corrupt.\(timestamp).\(UUID().uuidString).json"
        let corruptURL = folder.appending(path: filename, directoryHint: .notDirectory)
        try fm.copyItem(at: url, to: corruptURL)
        try fm.setAttributes([.posixPermissions: 0o600], ofItemAtPath: corruptURL.path(percentEncoded: false))
        return corruptURL
    }

    private static func replaceFileAtomically(with data: Data, at url: URL, in folder: URL) throws {
        let fm = FileManager.default
        let temporaryURL = folder.appending(path: "\(url.lastPathComponent).tmp.\(UUID().uuidString)", directoryHint: .notDirectory)
        defer { try? fm.removeItem(at: temporaryURL) }
        try data.write(to: temporaryURL, options: .atomic)
        try fm.setAttributes([.posixPermissions: 0o600], ofItemAtPath: temporaryURL.path(percentEncoded: false))
        if fm.fileExists(atPath: url.path(percentEncoded: false)) {
            _ = try fm.replaceItemAt(url, withItemAt: temporaryURL)
        } else {
            try fm.moveItem(at: temporaryURL, to: url)
        }
    }
}
