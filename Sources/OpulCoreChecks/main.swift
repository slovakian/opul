import Foundation
import OpulCore

func expect(_ condition: @autoclosure () -> Bool, _ message: String) {
    if !condition() {
        fputs("FAIL: \(message)\n", stderr)
        exit(1)
    }
}

func guidedObservation(
    url: String,
    initiator: String,
    source: GuidedObservationSource,
    frameURL: String? = nil,
    mimeType: String? = nil,
    elementKey: String? = nil,
    headers: [GuidedHttpHeader] = []
) -> GuidedObservation {
    GuidedObservation(
        url: url,
        pageUrl: "https://app.invalid/watch",
        frameUrl: frameURL,
        title: "Fixture",
        initiator: initiator,
        mimeType: mimeType,
        statusCode: 200,
        contentLength: 2_000_000,
        observedAtMs: 1_700_000_000_000,
        source: source,
        isMainFrame: frameURL == nil,
        elementKey: elementKey,
        width: nil,
        height: nil,
        headers: headers
    )
}

func checkApplicationStateMachine() throws {
    let root = FileManager.default.temporaryDirectory
        .appending(path: "opul-state-checks-\(UUID().uuidString)", directoryHint: .isDirectory)
    defer { try? FileManager.default.removeItem(at: root) }
    let machine = try OpulStateMachine.open(config: OpulMachineConfig(
        applicationSupportFolder: root.path(percentEncoded: false),
        downloadsFolder: root.appending(path: "Downloads").path(percentEncoded: false),
        torrent: EngineConfig(
            sessionFolder: root.appending(path: "torrent").path(percentEncoded: false),
            listenPort: 0,
            randomizePort: true,
            enableDht: false,
            maxConnections: 8,
            maxDownloadBps: 0,
            maxUploadBps: 0
        ),
        media: MediaEngineConfig(
            sessionFolder: root.appending(path: "media").path(percentEncoded: false),
            resourcesFolder: nil,
            ytDlpPath: nil,
            ffmpegPath: nil,
            maxConcurrentDownloads: 1,
            maxDownloadBps: 0
        )
    ))
    let boot = machine.snapshot()
    expect(boot.session == .running, "state machine reconciles before open returns")

    let workspaceID = UUID().uuidString
    let folderID = UUID().uuidString
    let requestID = UUID().uuidString
    let request = MachineWorkspaceRequest(
        requestId: requestID,
        expectedRevision: nil,
        workspaceId: workspaceID,
        name: "State Machine Check",
        icon: MachineWorkspaceIcon(kind: .symbol, value: "checkmark.circle"),
        folderId: folderID,
        folderName: "Checks",
        folderPath: root.appending(path: "Checks").path(percentEncoded: false),
        defaultFormat: .best,
        fileOrganization: .automatic
    )
    let first = try machine.createWorkspace(request: request)
    let replay = try machine.createWorkspace(request: request)
    expect(first == replay, "command replay returns its durable receipt")
    expect(
        machine.snapshot().workspaces.contains { $0.id == workspaceID.lowercased() },
        "committed workspace appears in the projection"
    )
    _ = try machine.beginShutdown()
    expect(machine.snapshot().session == .stopped, "clean shutdown reaches stopped")
}

func checkInvalidStateMachineRecovery() throws {
    let root = FileManager.default.temporaryDirectory
        .appending(path: "opul-invalid-state-checks-\(UUID().uuidString)", directoryHint: .isDirectory)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }

    let stateURL = root.appending(path: "application-state.json")
    try "CORRUPTED_NON_JSON_CONTENT".data(using: .utf8)!.write(to: stateURL)

    let config = OpulMachineConfig(
        applicationSupportFolder: root.path(percentEncoded: false),
        downloadsFolder: root.appending(path: "Downloads").path(percentEncoded: false),
        torrent: EngineConfig(
            sessionFolder: root.appending(path: "torrent").path(percentEncoded: false),
            listenPort: 0,
            randomizePort: true,
            enableDht: false,
            maxConnections: 8,
            maxDownloadBps: 0,
            maxUploadBps: 0
        ),
        media: MediaEngineConfig(
            sessionFolder: root.appending(path: "media").path(percentEncoded: false),
            resourcesFolder: nil,
            ytDlpPath: nil,
            ffmpegPath: nil,
            maxConcurrentDownloads: 1,
            maxDownloadBps: 0
        )
    )

    let machine = try OpulStateMachine.open(config: config)
    var snapshot = machine.snapshot()
    expect(snapshot.session == .recoveryRequired, "corrupt state opens in recoveryRequired")
    expect(!snapshot.recoveryNotices.isEmpty, "recovery notices present for corrupt state")

    _ = try machine.acknowledgeRecovery()
    snapshot = machine.snapshot()
    expect(snapshot.session == .running, "acknowledging recovery transitions to running")
    expect(snapshot.recoveryNotices.isEmpty, "recovery notices cleared after acknowledgement")
    expect(!snapshot.workspaces.isEmpty, "fresh workspace is available after recovery")

    _ = try machine.beginShutdown()
}

do {
    try checkApplicationStateMachine()
    try checkInvalidStateMachineRecovery()
    let empty = try Session().summarize()
    expect(empty.count == 0, "empty count")
    expect(empty.total == 0, "empty total")
    expect(empty.mean == 0, "empty mean")

    let session = Session()
    session.record(value: 2)
    session.record(value: 4)
    let summary = try session.summarize()
    expect(summary.count == 2, "recorded count")
    expect(summary.total == 6, "recorded total")
    expect(summary.mean == 3, "recorded mean")

    let overflow = Session()
    overflow.record(value: Int64.max)
    overflow.record(value: 1)
    do {
        _ = try overflow.summarize()
        expect(false, "overflow should throw")
    } catch let error as CoreError {
        expect(error == .TotalOverflow, "overflow case")
    }

    expect(
        classifyGuidedUrl(url: "https://cdn.invalid/master.m3u8", initiator: "fetch") == .stream,
        "HLS stream classification"
    )
    expect(
        classifyGuidedUrl(url: "https://cdn.invalid/captions.vtt", initiator: "track") == .subtitle,
        "subtitle classification"
    )
    expect(
        classifyGuidedUrl(url: "https://cdn.invalid/cover.webp", initiator: "img") == .image,
        "image classification"
    )

    let extraction = GuidedExtractionSession()
    var candidates = extraction.observe(observation: guidedObservation(
        url: "https://cdn.invalid/movie.mp4",
        initiator: "video",
        source: .dom,
        elementKey: "video-1"
    ))
    expect(candidates.count == 1 && candidates[0].directDownload, "direct media candidate")
    candidates = extraction.observe(observation: guidedObservation(
        url: "https://cdn.invalid/hls/master.m3u8",
        initiator: "fetch",
        source: .performance
    ))
    candidates = extraction.observe(observation: guidedObservation(
        url: "https://cdn.invalid/dash/manifest.mpd",
        initiator: "fetch",
        source: .performance
    ))
    expect(candidates.filter { $0.kind == .stream }.count == 2, "HLS and DASH candidates")
    let beforeSegment = candidates.count
    candidates = extraction.observe(observation: guidedObservation(
        url: "https://cdn.invalid/hls/chunk-12.ts",
        initiator: "fetch",
        source: .performance
    ))
    expect(candidates.count == beforeSegment, "HLS segment collapsing")

    candidates = extraction.observe(observation: guidedObservation(
        url: "https://cdn.invalid/private/video",
        initiator: "video",
        source: .dom,
        frameURL: "https://embed.invalid/player",
        mimeType: "video/mp4",
        headers: [
            GuidedHttpHeader(name: "Authorization", value: "Bearer fixture"),
            GuidedHttpHeader(name: "Cookie", value: "not-forwarded-as-a-header")
        ]
    ))
    let iframe = candidates.first { $0.url.contains("/private/video") }
    expect(iframe?.referer == "https://embed.invalid/player", "iframe referrer")
    expect(iframe?.frameUrl == "https://embed.invalid/player", "iframe frame URL")

    candidates = extraction.observe(observation: guidedObservation(
        url: "https://cdn.invalid/popup-audio",
        initiator: "popup",
        source: .popup,
        mimeType: "audio/mpeg"
    ))
    expect(candidates.contains { $0.kind == .audio && $0.reason.contains("Popup") }, "popup candidate")

    let expiring = GuidedExtractionSession()
    let firstSigned = expiring.observe(observation: guidedObservation(
        url: "https://cdn.invalid/live/master.m3u8?token=old&expires=4102444800",
        initiator: "fetch",
        source: .performance
    ))
    let refreshed = expiring.observe(observation: guidedObservation(
        url: "https://cdn.invalid/live/master.m3u8?token=new&expires=4102444900",
        initiator: "fetch",
        source: .performance
    ))
    expect(refreshed.count == 1, "expiring manifest deduplication")
    expect(refreshed[0].id == firstSigned[0].id && refreshed[0].url.contains("token=new"), "signed URL refresh")

    let sessionFolder = FileManager.default.temporaryDirectory
        .appending(path: "opul-core-checks-\(UUID().uuidString)", directoryHint: .isDirectory)
    defer { try? FileManager.default.removeItem(at: sessionFolder) }
    let media = try MediaEngine(config: MediaEngineConfig(
        sessionFolder: sessionFolder.path(percentEncoded: false),
        resourcesFolder: nil,
        ytDlpPath: nil,
        ffmpegPath: nil,
        maxConcurrentDownloads: 2,
        maxDownloadBps: 0
    ))
    let tools = media.toolStatus()
    expect(!tools.message.isEmpty, "media tool diagnostic")
    expect(media.snapshots().isEmpty, "empty media queue")

    if let smokeURL = ProcessInfo.processInfo.environment["OPUL_MEDIA_SMOKE_URL"] {
        let probe = try media.probeUrl(url: smokeURL)
        expect(probe.items.count == 1, "smoke probe item")
        let transferID = try media.addDownload(
            item: probe.items[0],
            outputFolder: sessionFolder.path(percentEncoded: false),
            options: MediaDownloadOptions(
                format: .best,
                contentLayout: .original,
                downloadSubtitles: false,
                subtitleLanguages: [],
                embedSubtitles: false,
                writeThumbnail: false,
                writeMetadata: false,
                filenameTemplate: "smoke.%(ext)s",
                referer: smokeURL,
                userAgent: nil
            )
        )
        var completed: MediaTransferSnapshot?
        for _ in 0..<100 {
            if let snapshot = media.snapshots().first(where: { $0.id == transferID }),
               snapshot.state == .finished || snapshot.state == .failed {
                completed = snapshot
                break
            }
            try await Task.sleep(for: .milliseconds(100))
        }
        expect(completed?.state == .finished, "smoke download finished: \(completed?.errorMessage ?? "timed out")")
        expect(completed?.outputPath.map(FileManager.default.fileExists(atPath:)) == true, "smoke output exists")
    }

    print("OpulCore checks passed")
} catch {
    fputs("FAIL: \(error)\n", stderr)
    exit(1)
}
