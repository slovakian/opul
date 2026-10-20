import AppKit
import Foundation

enum InteractionSoundCategory: String, CaseIterable, Codable, Identifiable, Sendable {
    case success
    case error
    case warning
    case notification
    case tap

    var id: Self { self }

    var title: String {
        switch self {
        case .success: "Success"
        case .error: "Error"
        case .warning: "Warning"
        case .notification: "Notification"
        case .tap: "Tap"
        }
    }

    var explanation: String {
        switch self {
        case .success: "A download finishes successfully."
        case .error: "An active download fails."
        case .warning: "A download cannot start."
        case .notification: "Another noteworthy event occurs."
        case .tap: "Download and seeding controls are used."
        }
    }

    static func forDownloadTransition(from previousState: DownloadState?, to state: DownloadState) -> Self? {
        guard let previousState, previousState != state else { return nil }
        let completedStates: Set<DownloadState> = [.seeding, .finished]
        if completedStates.contains(state), !completedStates.contains(previousState) {
            return .success
        }
        if state == .failed, previousState != .failed {
            return .error
        }
        return nil
    }

    fileprivate var bundledFolderName: String { rawValue.capitalized }

    fileprivate var fallbackSoundNames: [String] {
        switch self {
        case .success: ["Glass", "Hero", "Purr"]
        case .error: ["Basso", "Funk", "Sosumi"]
        case .warning: ["Submarine", "Morse", "Blow"]
        case .notification: ["Ping", "Pop", "Tink", "Bottle"]
        case .tap: []
        }
    }
}

enum InteractionSoundCue: CaseIterable, Sendable {
    case resume
    case pause

    var category: InteractionSoundCategory { .tap }

    var bundledFilename: String {
        switch self {
        case .resume: "tap-cjmw2.wav"
        case .pause: "tap-smmhs.wav"
        }
    }
}

struct InteractionSoundPreferences: Equatable, Codable, Sendable {
    var isEnabled = true
    var volume = 0.8
    var enabledCategories = Set(InteractionSoundCategory.allCases)
    private var enabledCategoriesVersion = 1

    private enum CodingKeys: String, CodingKey {
        case isEnabled
        case volume
        case enabledCategories
        case enabledCategoriesVersion
    }

    init() {}

    init(from decoder: Decoder) throws {
        self.init()
        let values = try decoder.container(keyedBy: CodingKeys.self)
        isEnabled = try values.decodeIfPresent(Bool.self, forKey: .isEnabled) ?? isEnabled
        let decodedVolume = try values.decodeIfPresent(Double.self, forKey: .volume) ?? volume
        volume = min(max(decodedVolume, 0), 1)
        enabledCategories = try values.decodeIfPresent(
            Set<InteractionSoundCategory>.self,
            forKey: .enabledCategories
        ) ?? enabledCategories
        let decodedVersion = try values.decodeIfPresent(Int.self, forKey: .enabledCategoriesVersion) ?? 0
        if decodedVersion < 1 {
            enabledCategories.insert(.tap)
        }
        enabledCategoriesVersion = 1
    }

    func isEnabled(_ category: InteractionSoundCategory) -> Bool {
        isEnabled && enabledCategories.contains(category)
    }

    mutating func setEnabled(_ enabled: Bool, for category: InteractionSoundCategory) {
        if enabled {
            enabledCategories.insert(category)
        } else {
            enabledCategories.remove(category)
        }
    }
}

final class InteractionSoundEngine {
    private static let supportedExtensions = Set(["aif", "aiff", "caf", "m4a", "mp3", "wav"])

    private let randomSoundsByCategory: [InteractionSoundCategory: [URL]]
    private let fixedSoundsByCue: [InteractionSoundCue: URL]
    private var remainingSounds: [InteractionSoundCategory: [URL]] = [:]
    private var previousSound: [InteractionSoundCategory: URL] = [:]
    private var activeSounds: [NSSound] = []

    init() {
        randomSoundsByCategory = Dictionary(uniqueKeysWithValues: InteractionSoundCategory.allCases.map { category in
            let bundledSounds = Self.bundledSounds(for: category)
            let sounds = bundledSounds.isEmpty ? Self.systemSounds(for: category) : bundledSounds
            return (category, sounds)
        })
        fixedSoundsByCue = Dictionary(uniqueKeysWithValues: InteractionSoundCue.allCases.compactMap { cue in
            guard let sound = Self.bundledSound(named: cue.bundledFilename, for: cue.category) else {
                return nil
            }
            return (cue, sound)
        })
    }

    func playRandom(_ category: InteractionSoundCategory, preferences: InteractionSoundPreferences) {
        guard preferences.isEnabled(category), let url = nextSound(for: category) else { return }
        play(url, volume: preferences.volume)
    }

    func play(_ cue: InteractionSoundCue, preferences: InteractionSoundPreferences) {
        guard preferences.isEnabled(cue.category), let url = fixedSoundsByCue[cue] else { return }
        play(url, volume: preferences.volume)
    }

    private func play(_ url: URL, volume: Double) {
        activeSounds.removeAll { !$0.isPlaying }
        guard let sound = NSSound(contentsOf: url, byReference: true) else { return }
        sound.volume = Float(min(max(volume, 0), 1))
        activeSounds.append(sound)
        sound.play()
    }

    private func nextSound(for category: InteractionSoundCategory) -> URL? {
        guard let sounds = randomSoundsByCategory[category], !sounds.isEmpty else { return nil }
        if remainingSounds[category]?.isEmpty != false {
            var shuffled = sounds.shuffled()
            if shuffled.count > 1, shuffled.first == previousSound[category] {
                shuffled.swapAt(0, Int.random(in: 1..<shuffled.count))
            }
            remainingSounds[category] = shuffled
        }
        let sound = remainingSounds[category]?.removeFirst()
        previousSound[category] = sound
        return sound
    }

    private static func bundledSounds(for category: InteractionSoundCategory) -> [URL] {
        let roots = [Bundle.opulResources.resourceURL, Bundle.main.resourceURL].compactMap { $0 }
        for root in roots {
            let folder = root
                .appending(path: "Sounds", directoryHint: .isDirectory)
                .appending(path: category.bundledFolderName, directoryHint: .isDirectory)
            guard let contents = try? FileManager.default.contentsOfDirectory(
                at: folder,
                includingPropertiesForKeys: nil,
                options: [.skipsHiddenFiles]
            ) else {
                continue
            }
            let sounds = contents
                .filter { supportedExtensions.contains($0.pathExtension.lowercased()) }
                .sorted { $0.lastPathComponent.localizedStandardCompare($1.lastPathComponent) == .orderedAscending }
            if !sounds.isEmpty {
                return sounds
            }
        }
        return []
    }

    private static func bundledSound(named filename: String, for category: InteractionSoundCategory) -> URL? {
        let roots = [Bundle.opulResources.resourceURL, Bundle.main.resourceURL].compactMap { $0 }
        for root in roots {
            let sound = root
                .appending(path: "Sounds", directoryHint: .isDirectory)
                .appending(path: category.bundledFolderName, directoryHint: .isDirectory)
                .appending(path: filename, directoryHint: .notDirectory)
            if FileManager.default.fileExists(atPath: sound.path(percentEncoded: false)) {
                return sound
            }
        }
        return nil
    }

    private static func systemSounds(for category: InteractionSoundCategory) -> [URL] {
        let folder = URL(fileURLWithPath: "/System/Library/Sounds", isDirectory: true)
        return category.fallbackSoundNames.compactMap { name in
            let url = folder.appending(path: "\(name).aiff", directoryHint: .notDirectory)
            return FileManager.default.fileExists(atPath: url.path) ? url : nil
        }
    }
}
