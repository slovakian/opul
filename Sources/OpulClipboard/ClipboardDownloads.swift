import AppKit
import Foundation

public struct PasteboardItem: Equatable, Sendable {
    public var string: String?
    public var url: String?
    public var fileURL: String?

    public init(string: String? = nil, url: String? = nil, fileURL: String? = nil) {
        self.string = string
        self.url = url
        self.fileURL = fileURL
    }
}

public enum ClipboardDownload: Equatable, Sendable {
    /// Magnet link, web page, or remote `.torrent` URL. Pass to `paste(address:)`.
    case address(String)
    /// Local `.torrent` file. Pass to `addTorrentFile(at:)`.
    case torrentFile(String)
}

public func isDownloadableAddress(
    _ address: String,
    fileExists: (String) -> Bool = { FileManager.default.fileExists(atPath: $0) }
) -> Bool {
    let lower = address.lowercased()
    if lower.hasPrefix("magnet:") || lower.hasPrefix("http://") || lower.hasPrefix("https://") || lower.hasPrefix("opul://") {
        return true
    }
    if lower.hasSuffix(".torrent") {
        return true
    }
    if fileExists(address) {
        return true
    }
    if lower.hasPrefix("www.") || lower.contains("youtube.com/") || lower.contains("youtu.be/") || lower.contains("vimeo.com/") || lower.contains("bilibili.com/") || lower.contains("twitch.tv/") {
        return true
    }
    if lower.contains("/") && (lower.contains(".com") || lower.contains(".org") || lower.contains(".net") || lower.contains(".io") || lower.contains(".co") || lower.contains(".cc") || lower.contains(".tv")) {
        return true
    }
    return false
}

public func clipboardDownloads(
    in items: [PasteboardItem],
    fileExists: (String) -> Bool = { FileManager.default.fileExists(atPath: $0) }
) -> [ClipboardDownload] {
    var found: [ClipboardDownload] = []
    var seen = Set<String>()
    for item in items {
        for download in classify(item, fileExists: fileExists) {
            let key = dedupeKey(download)
            guard seen.insert(key).inserted else { continue }
            found.append(download)
        }
    }
    return found
}

public func pasteboardItems(on pasteboard: NSPasteboard) -> [PasteboardItem] {
    let parsed = (pasteboard.pasteboardItems ?? []).map { item in
        PasteboardItem(
            string: item.string(forType: .string),
            url: item.string(forType: .URL),
            fileURL: item.string(forType: .fileURL)
        )
    }
    let useful = parsed.filter { item in
        item.string?.isEmpty == false || item.url?.isEmpty == false || item.fileURL?.isEmpty == false
    }
    if useful.contains(where: { $0.fileURL?.isEmpty == false }) {
        return useful
    }
    if let paths = pasteboard.propertyList(forType: NSPasteboard.PasteboardType("NSFilenamesPboardType")) as? [String],
       !paths.isEmpty {
        let files = paths.map { path in
            PasteboardItem(fileURL: URL(fileURLWithPath: path).absoluteString)
        }
        return useful.isEmpty ? files : files + useful
    }
    if !useful.isEmpty {
        return useful
    }
    if let urls = pasteboard.readObjects(forClasses: [NSURL.self], options: nil) as? [URL], !urls.isEmpty {
        return urls.map { url in
            if url.isFileURL {
                PasteboardItem(fileURL: url.absoluteString)
            } else {
                PasteboardItem(url: url.absoluteString)
            }
        }
    }
    if let string = pasteboard.string(forType: .string), !string.isEmpty {
        return [PasteboardItem(string: string)]
    }
    return []
}

private func classify(
    _ item: PasteboardItem,
    fileExists: (String) -> Bool
) -> [ClipboardDownload] {
    if let path = localTorrentPath(item.fileURL) ?? fileTorrentPath(item.url),
       path.hasPrefix("/") || fileExists(path) {
        return [.torrentFile(path)]
    }

    var downloads: [ClipboardDownload] = []
    if let url = item.url {
        append(url, fileExists: fileExists, to: &downloads)
    }
    if let string = item.string {
        for line in string.split(whereSeparator: \.isNewline) {
            append(String(line), fileExists: fileExists, to: &downloads)
        }
    }
    return downloads
}

private func fileTorrentPath(_ raw: String?) -> String? {
    guard let raw, let url = URL(string: raw), url.isFileURL else { return nil }
    return localTorrentPath(raw)
}

private func append(
    _ raw: String,
    fileExists: (String) -> Bool,
    to downloads: inout [ClipboardDownload]
) {
    let address = unwrap(raw)
    guard !address.isEmpty else { return }
    if let path = localTorrentPath(address) {
        if path.hasPrefix("/") || fileExists(path) {
            downloads.append(.torrentFile(path))
        }
        return
    }
    guard isDownloadableAddress(address, fileExists: { _ in false }) else { return }
    downloads.append(.address(address))
}

private func unwrap(_ raw: String) -> String {
    let address = raw.trimmingCharacters(in: .whitespacesAndNewlines)
    guard address.count >= 2, address.first == "<", address.last == ">" else {
        return address
    }
    let inner = String(address.dropFirst().dropLast()).trimmingCharacters(in: .whitespacesAndNewlines)
    return inner.isEmpty ? address : inner
}

private func localTorrentPath(_ raw: String?) -> String? {
    guard let raw else { return nil }
    let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else { return nil }
    if let url = URL(string: trimmed), let scheme = url.scheme?.lowercased(), !scheme.isEmpty {
        guard scheme == "file" else { return nil }
        let path = url.path
        guard path.lowercased().hasSuffix(".torrent") else { return nil }
        return path
    }
    if trimmed.lowercased().hasPrefix("file:") {
        return nil
    }
    let path = trimmed.hasPrefix("~") ? (trimmed as NSString).expandingTildeInPath : trimmed
    guard path.lowercased().hasSuffix(".torrent") else { return nil }
    return path
}

private func dedupeKey(_ download: ClipboardDownload) -> String {
    switch download {
    case .address(let address):
        "a\u{0}\(address)"
    case .torrentFile(let path):
        "f\u{0}\(URL(fileURLWithPath: path).standardizedFileURL.path.lowercased())"
    }
}
