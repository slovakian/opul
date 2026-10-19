import Foundation

enum StorageChecks {
    private static func expect(_ condition: @autoclosure () -> Bool, _ message: String) {
        if !condition() {
            fputs("FAIL: \(message)\n", stderr)
            exit(1)
        }
    }

    static func run() {
        print("==> Running Opul storage and resilience checks")
        testNormal010To10Upgrade()
        testUnknownFields()
        testIncompatibleSchema()
        testUnknownEnumCaseIsIncompatible()
        testTruncatedJSONWithBackupRecovery()
        testInterruptedWritesAndAtomicPermissions()
        testCorruptFileNeverSilentlyReplaced()
        testInteractionSoundPreferences()
        testInteractionSoundCues()
        testInteractionSoundTransitions()
        testDefaultDownloadArrangement()
        print("✓ All storage and resilience checks passed!")
    }

    // MARK: - 1. Normal 0.1.0 to 1.0 Upgrade Fixture
    static func testNormal010To10Upgrade() {
        let tempDir = FileManager.default.temporaryDirectory.appending(path: "opul-storage-check-010-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let libraryURL = tempDir.appending(path: "library.json")
        let backupURL = tempDir.appending(path: "library.json.bak")

        // Real 0.1.0 format fixture: a raw array without schemaVersion.
        let fixture010 = """
        [
          {
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "General Downloads",
            "icon": {
              "symbol": {
                "_0": "arrow.down.circle"
              }
            },
            "savedFolders": [
              {
                "id": "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE",
                "name": "Downloads",
                "path": "/Users/test/Downloads",
                "isDefault": true
              }
            ],
            "defaultFormat": "original",
            "view": {
              "layout": "list",
              "sort": "order",
              "grouping": "none"
            },
            "downloads": [
              {
                "id": "99999999-8888-7777-6666-555555555555",
                "name": "Sample File",
                "address": "https://example.com/sample.pdf",
                "state": "finished",
                "source": "fileURL",
                "folderID": "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE",
                "totalSize": 1048576,
                "bytesDownloaded": 1048576,
                "progress": 1.0
              }
            ]
          }
        ]
        """
        try! fixture010.data(using: .utf8)!.write(to: libraryURL)

        // Load 0.1.0 format
        let outcome = LibraryStorage.load(from: libraryURL, backupURL: backupURL)
        guard case .loaded(let workspaces) = outcome else {
            expect(false, "0.1.0 library should load cleanly")
            return
        }
        expect(workspaces.count == 1, "expected 1 workspace from 0.1.0 fixture")
        expect(workspaces[0].name == "General Downloads", "workspace name mismatch")
        expect(workspaces[0].downloads.count == 1, "expected 1 download")
        expect(workspaces[0].downloads[0].name == "Sample File", "download name mismatch")
        expect(workspaces[0].downloads[0].state == .finished, "download state mismatch")

        // Save as 1.0 (versioned envelope)
        try! LibraryStorage.save(workspaces: workspaces, to: libraryURL, backupURL: backupURL, applicationSupportFolder: tempDir)

        // Verify upgraded format has schemaVersion: 1
        let upgradedData = try! Data(contentsOf: libraryURL)
        let envelope = try! JSONDecoder().decode(LibraryEnvelope.self, from: upgradedData)
        expect(envelope.schemaVersion == 1, "expected schemaVersion 1 in upgraded file")
        expect(envelope.workspaces.count == 1, "expected 1 workspace in upgraded envelope")

        // Re-load and verify compatibility
        let reloadedOutcome = LibraryStorage.load(from: libraryURL, backupURL: backupURL)
        guard case .loaded(let reloaded) = reloadedOutcome else {
            expect(false, "upgraded 1.0 envelope should load cleanly")
            return
        }
        expect(reloaded == workspaces, "reloaded workspaces must match")
        print("  ✓ Normal 0.1.0 to 1.0 upgrade check passed")
    }

    // MARK: - 2. Unknown Fields Fixture
    static func testUnknownFields() {
        let tempDir = FileManager.default.temporaryDirectory.appending(path: "opul-storage-check-unknown-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let libraryURL = tempDir.appending(path: "library.json")
        let backupURL = tempDir.appending(path: "library.json.bak")

        let fixtureUnknownFields = """
        {
          "schemaVersion": 1,
          "unknownRootField": "future_version_flag",
          "futureFeatureFlags": { "p2pSharing": true },
          "workspaces": [
            {
              "id": "11111111-2222-3333-4444-555555555555",
              "name": "Future Workspace",
              "extraWorkspaceMetadata": "test",
              "icon": { "symbol": { "_0": "folder" } },
              "savedFolders": [
                {
                  "id": "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE",
                  "name": "Downloads",
                  "path": "/tmp",
                  "isDefault": true,
                  "folderColor": "#FF0000"
                }
              ],
              "defaultFormat": "best",
              "view": {
                "layout": "list",
                "sort": "name",
                "grouping": "none",
                "columnWidths": [100, 200]
              },
              "downloads": [
                {
                  "id": "99999999-8888-7777-6666-555555555555",
                  "name": "Download with Future Props",
                  "address": "https://example.com/file",
                  "state": "queued",
                  "source": "fileURL",
                  "folderID": "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE",
                  "tags": ["audio", "priority"],
                  "cloudSyncStatus": "synced"
                }
              ]
            }
          ]
        }
        """
        try! fixtureUnknownFields.data(using: .utf8)!.write(to: libraryURL)

        let outcome = LibraryStorage.load(from: libraryURL, backupURL: backupURL)
        guard case .loaded(let workspaces) = outcome else {
            expect(false, "library with unknown fields should load cleanly")
            return
        }
        expect(workspaces.count == 1, "expected 1 workspace")
        expect(workspaces[0].name == "Future Workspace", "name mismatch")
        expect(workspaces[0].downloads[0].state == .queued, "known state should survive unknown sibling fields")
        expect(workspaces[0].downloads[0].source == .fileURL, "known source should survive unknown sibling fields")
        print("  ✓ Unknown fields check passed")
    }

    // MARK: - 3. Incompatible Schema Fixture
    static func testIncompatibleSchema() {
        let tempDir = FileManager.default.temporaryDirectory.appending(path: "opul-storage-check-incompat-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let libraryURL = tempDir.appending(path: "library.json")
        let backupURL = tempDir.appending(path: "library.json.bak")

        let incompatibleJSON = """
        {
          "schemaVersion": 2,
          "workspaces": []
        }
        """
        try! incompatibleJSON.data(using: .utf8)!.write(to: libraryURL)

        let outcome = LibraryStorage.load(from: libraryURL, backupURL: backupURL)
        guard case .preservedCorrupt(let corruptURL, _) = outcome else {
            expect(false, "incompatible schema should result in .preservedCorrupt")
            return
        }
        expect(FileManager.default.fileExists(atPath: corruptURL.path(percentEncoded: false)), "future-schema file must be preserved")
        expect(FileManager.default.fileExists(atPath: libraryURL.path(percentEncoded: false)), "original file must remain untouched")
        expect((try? LibraryStorage.decodeWorkspaces(from: Data(contentsOf: libraryURL))) == nil, "future schema must not be decoded by this version")
        print("  ✓ Future schema preservation check passed")
    }

    static func testUnknownEnumCaseIsIncompatible() {
        let tempDir = FileManager.default.temporaryDirectory.appending(path: "opul-storage-check-enum-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let libraryURL = tempDir.appending(path: "library.json")
        let backupURL = tempDir.appending(path: "library.json.bak")
        let incompatibleJSON = """
        {
          "schemaVersion": 1,
          "workspaces": [{
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "Future Workspace",
            "icon": { "symbol": { "_0": "folder" } },
            "savedFolders": [],
            "defaultFormat": "future-format",
            "view": { "layout": "list", "sort": "order", "grouping": "none" },
            "downloads": []
          }]
        }
        """
        try! incompatibleJSON.data(using: .utf8)!.write(to: libraryURL)

        let outcome = LibraryStorage.load(from: libraryURL, backupURL: backupURL)
        guard case .preservedCorrupt(let corruptURL, _) = outcome else {
            expect(false, "unknown enum case should preserve the incompatible library")
            return
        }
        expect(FileManager.default.fileExists(atPath: corruptURL.path(percentEncoded: false)), "unknown-enum library must be preserved")
        print("  ✓ Unknown enum incompatibility check passed")
    }

    // MARK: - 4. Truncated JSON with Backup Recovery Fixture
    static func testTruncatedJSONWithBackupRecovery() {
        let tempDir = FileManager.default.temporaryDirectory.appending(path: "opul-storage-check-truncated-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let libraryURL = tempDir.appending(path: "library.json")
        let backupURL = tempDir.appending(path: "library.json.bak")

        // Save good backup
        let goodWorkspace = Workspace(
            name: "Backed Up Workspace",
            icon: .symbol("star"),
            savedFolders: [],
            defaultFormat: .best,
            view: WorkspaceViewSettings(),
            downloads: []
        )
        let envelope = LibraryEnvelope(schemaVersion: 1, workspaces: [goodWorkspace])
        let goodData = try! JSONEncoder().encode(envelope)
        try! goodData.write(to: backupURL)

        // Write truncated JSON to primary file
        let truncatedJSON = "[{\"id\": \"11111111-2222-3333-4444-555555555555\", \"name\": \"Truncat"
        try! truncatedJSON.data(using: .utf8)!.write(to: libraryURL)

        let outcome = LibraryStorage.load(from: libraryURL, backupURL: backupURL)
        guard case .recoveredFromBackup(let restored, _, let preservedURL) = outcome,
              let corruptURL = preservedURL else {
            expect(false, "truncated JSON should recover from backup")
            return
        }
        expect(restored.count == 1, "expected 1 restored workspace")
        expect(restored[0].name == "Backed Up Workspace", "restored name mismatch")
        expect(FileManager.default.fileExists(atPath: corruptURL.path(percentEncoded: false)), "corrupt file must be preserved")
        let corruptData = try! Data(contentsOf: corruptURL)
        expect(String(data: corruptData, encoding: .utf8) == truncatedJSON, "preserved corrupt data must match")
        print("  ✓ Truncated JSON with backup recovery check passed")
    }

    // MARK: - 5. Interrupted Writes and Atomic Permissions
    static func testInterruptedWritesAndAtomicPermissions() {
        let tempDir = FileManager.default.temporaryDirectory.appending(path: "opul-storage-check-atomic-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let libraryURL = tempDir.appending(path: "library.json")
        let backupURL = tempDir.appending(path: "library.json.bak")

        let ws1 = Workspace(name: "Version 1", icon: .symbol("folder"), savedFolders: [], defaultFormat: .original, view: WorkspaceViewSettings(), downloads: [])
        try! LibraryStorage.save(workspaces: [ws1], to: libraryURL, backupURL: backupURL, applicationSupportFolder: tempDir)

        let attrs1 = try! FileManager.default.attributesOfItem(atPath: libraryURL.path(percentEncoded: false))
        let perms1 = (attrs1[.posixPermissions] as? NSNumber)?.intValue ?? 0
        expect(perms1 == 0o600, "library permissions should be 0600 (got \(String(format: "%o", perms1)))")

        let ws2 = Workspace(name: "Version 2", icon: .symbol("folder"), savedFolders: [], defaultFormat: .original, view: WorkspaceViewSettings(), downloads: [])
        try! LibraryStorage.save(workspaces: [ws2], to: libraryURL, backupURL: backupURL, applicationSupportFolder: tempDir)

        // Verify primary has ws2 and backup has ws1
        let primaryLoaded = try! LibraryStorage.decodeWorkspaces(from: Data(contentsOf: libraryURL))
        expect(primaryLoaded[0].name == "Version 2", "primary file should have version 2")

        let backupLoaded = try! LibraryStorage.decodeWorkspaces(from: Data(contentsOf: backupURL))
        expect(backupLoaded[0].name == "Version 1", "backup file should have version 1")

        print("  ✓ Interrupted writes and atomic permissions check passed")
    }

    // MARK: - 6. Corrupt File Never Silently Replaced
    static func testCorruptFileNeverSilentlyReplaced() {
        let tempDir = FileManager.default.temporaryDirectory.appending(path: "opul-storage-check-never-replace-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let libraryURL = tempDir.appending(path: "library.json")
        let backupURL = tempDir.appending(path: "library.json.bak")

        let corruptedData = "INVALID_CORRUPTED_NON_JSON_CONTENT".data(using: .utf8)!
        try! corruptedData.write(to: libraryURL)

        let outcome = LibraryStorage.load(from: libraryURL, backupURL: backupURL)
        guard case .preservedCorrupt(let corruptURL, _) = outcome else {
            expect(false, "expected .preservedCorrupt")
            return
        }

        // Verify the original file still exists and contains the corrupted content
        let currentPrimary = try! Data(contentsOf: libraryURL)
        expect(currentPrimary == corruptedData, "primary corrupted file must not be silently replaced or erased")

        // Verify the preserved copy also exists
        let preservedCopy = try! Data(contentsOf: corruptURL)
        expect(preservedCopy == corruptedData, "preserved copy must match corrupted data")

        print("  ✓ Corrupt file never silently replaced check passed")
    }

    static func testInteractionSoundPreferences() {
        let legacyData = "{}".data(using: .utf8)!
        let legacyPreferences = try! JSONDecoder().decode(TorrentPreferences.self, from: legacyData)
        expect(legacyPreferences.interactionSounds.isEnabled, "sound effects should default to enabled")
        expect(legacyPreferences.interactionSounds.volume == 0.8, "sound volume should default to 80%")
        expect(
            legacyPreferences.interactionSounds.enabledCategories == Set(InteractionSoundCategory.allCases),
            "all sound categories should be enabled by default"
        )

        let preTapData = """
        {
          "isEnabled": true,
          "volume": 0.8,
          "enabledCategories": ["success", "error", "warning", "notification"]
        }
        """.data(using: .utf8)!
        let migratedPreferences = try! JSONDecoder().decode(InteractionSoundPreferences.self, from: preTapData)
        expect(
            migratedPreferences.enabledCategories.contains(.tap),
            "the Tap category should be enabled when migrating existing preferences"
        )

        var preferences = TorrentPreferences()
        preferences.interactionSounds.isEnabled = false
        preferences.interactionSounds.volume = 0.6
        preferences.interactionSounds.setEnabled(false, for: .notification)
        preferences.interactionSounds.setEnabled(false, for: .tap)
        let encoded = try! JSONEncoder().encode(preferences)
        let decoded = try! JSONDecoder().decode(TorrentPreferences.self, from: encoded)
        expect(!decoded.interactionSounds.isEnabled, "master sound preference should round-trip")
        expect(decoded.interactionSounds.volume == 0.6, "sound volume should round-trip")
        expect(
            !decoded.interactionSounds.enabledCategories.contains(.notification),
            "category sound preference should round-trip"
        )
        expect(
            !decoded.interactionSounds.enabledCategories.contains(.tap),
            "an explicit Tap category opt-out should round-trip"
        )
        print("  ✓ Interaction sound preference check passed")
    }

    static func testInteractionSoundCues() {
        expect(InteractionSoundCue.resume.category == .tap, "resume should use the Tap category")
        expect(InteractionSoundCue.resume.bundledFilename == "tap-cjmw2.wav", "resume should use tap-cjmw2")
        expect(InteractionSoundCue.pause.category == .tap, "pause should use the Tap category")
        expect(InteractionSoundCue.pause.bundledFilename == "tap-smmhs.wav", "pause should use tap-smmhs")
        print("  ✓ Interaction sound cue check passed")
    }

    static func testInteractionSoundTransitions() {
        expect(
            InteractionSoundCategory.forDownloadTransition(from: .fetching, to: .finished) == .success,
            "finishing a media download should use a success sound"
        )
        expect(
            InteractionSoundCategory.forDownloadTransition(from: .fetching, to: .seeding) == .success,
            "a torrent entering seeding should use a success sound"
        )
        expect(
            InteractionSoundCategory.forDownloadTransition(from: .seeding, to: .finished) == nil,
            "stopping a completed torrent should not repeat its success sound"
        )
        expect(
            InteractionSoundCategory.forDownloadTransition(from: .processing, to: .failed) == .error,
            "an active download failure should use an error sound"
        )
        expect(
            InteractionSoundCategory.forDownloadTransition(from: nil, to: .finished) == nil,
            "restoring a completed download should not play a sound"
        )
        print("  ✓ Interaction sound transition check passed")
    }

    static func testDefaultDownloadArrangement() {
        let folder = SavedFolder(name: "Downloads", path: "/tmp", isDefault: true)
        let first = Download(name: "First", address: "https://example.com/first", folderID: folder.id)
        let newest = Download(name: "Newest", address: "https://example.com/newest", folderID: folder.id)
        let settings = WorkspaceViewSettings()
        let arrangement = DownloadArrangement.make(
            downloads: [first, newest],
            folders: [folder],
            settings: settings
        )

        expect(settings.grouping == .none, "downloads should be ungrouped by default")
        expect(settings.sort == .order, "downloads should use date-added order by default")
        expect(!arrangement.showsHeaders, "the default download view should not show group headers")
        expect(arrangement.lines.map(\.id) == [newest.id, first.id], "newest downloads should appear first")
        print("  ✓ Default download arrangement check passed")
    }
}
