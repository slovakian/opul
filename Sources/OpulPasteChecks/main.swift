import AppKit
import OpulClipboard
import SwiftUI

func expect(_ condition: @autoclosure () -> Bool, _ message: String) {
    if !condition() {
        fputs("FAIL: \(message)\n", stderr)
        exit(1)
    }
}

let magnet = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567"
let video = "https://example.com/watch/123"
let remoteTorrent = "https://cdn.example.com/a.torrent?token=1"
func downloads(
    _ items: [PasteboardItem],
    fileExists: @escaping (String) -> Bool = { _ in false }
) -> [ClipboardDownload] {
    clipboardDownloads(in: items, fileExists: fileExists)
}

expect(downloads([PasteboardItem(string: magnet)]) == [.address(magnet)], "magnet link")
expect(downloads([PasteboardItem(string: "  \(video)  ")]) == [.address(video)], "web link is trimmed")
expect(downloads([PasteboardItem(string: "<\(video)>")]) == [.address(video)], "angle-bracket web link")
expect(downloads([PasteboardItem(string: video, url: video)]) == [.address(video)], "url and string are one download")
expect(
    downloads([PasteboardItem(string: "\(video)\nnot a link\n\(magnet)")]) == [.address(video), .address(magnet)],
    "several links on separate lines"
)
expect(downloads([PasteboardItem(string: remoteTorrent)]) == [.address(remoteTorrent)], "remote torrent url stays an address")
expect(downloads([PasteboardItem(string: "www.youtube.com/watch?v=abc")]) == [.address("www.youtube.com/watch?v=abc")], "www link")
expect(downloads([PasteboardItem(string: "youtu.be/abc")]) == [.address("youtu.be/abc")], "short video link")
expect(downloads([PasteboardItem(string: "opul://add?url=\(video)")]) == [.address("opul://add?url=\(video)")], "opul link")
expect(downloads([PasteboardItem(string: "hello")]) == [], "plain text is not a download")
expect(downloads([PasteboardItem(string: "   \n")]) == [], "blank paste")
expect(
    downloads([PasteboardItem(fileURL: URL(fileURLWithPath: "/tmp/movie.mp4").absoluteString)]) == [],
    "non-torrent file is ignored"
)

let show = URL(fileURLWithPath: "/Users/me/My Show.torrent")
expect(
    downloads([
        PasteboardItem(string: "My Show.torrent", fileURL: show.absoluteString),
    ]) == [.torrentFile("/Users/me/My Show.torrent")],
    "copied torrent file uses its path, not the file name"
)
expect(
    downloads([PasteboardItem(string: "/Users/me/missing.torrent")]) == [.torrentFile("/Users/me/missing.torrent")],
    "absolute torrent path"
)
expect(downloads([PasteboardItem(string: "notes.torrent")]) == [], "bare torrent file name without a path")
expect(
    downloads([PasteboardItem(string: "/tmp/notes.torrent")], fileExists: { $0 == "notes.torrent" }) == [.torrentFile("/tmp/notes.torrent")],
    "absolute path does not consult fileExists"
)

let homeTorrent = ("~/Downloads/a.torrent" as NSString).expandingTildeInPath
expect(
    downloads([PasteboardItem(string: "~/Downloads/a.torrent")]) == [.torrentFile(homeTorrent)],
    "tilde torrent path"
)
expect(
    downloads([
        PasteboardItem(string: video),
        PasteboardItem(url: video),
    ]) == [.address(video)],
    "duplicate links across items"
)
expect(
    downloads([
        PasteboardItem(string: magnet),
        PasteboardItem(fileURL: show.absoluteString),
    ]) == [.address(magnet), .torrentFile("/Users/me/My Show.torrent")],
    "magnet and torrent file together"
)

let board = NSPasteboard(name: NSPasteboard.Name("dev.opul.paste-checks"))
board.clearContents()
expect(board.setString(magnet, forType: .string), "write magnet")
expect(
    clipboardDownloads(in: pasteboardItems(on: board), fileExists: { _ in false }) == [.address(magnet)],
    "magnet read back from a pasteboard, items=\(pasteboardItems(on: board))"
)

board.clearContents()
expect(board.writeObjects([show as NSURL]), "write torrent file url")
expect(
    clipboardDownloads(in: pasteboardItems(on: board), fileExists: { _ in false }) == [.torrentFile("/Users/me/My Show.torrent")],
    "finder copy read back from a pasteboard, items=\(pasteboardItems(on: board))"
)

board.clearContents()
let remoteURL = URL(string: remoteTorrent)!
expect(board.writeObjects([remoteURL as NSURL]), "write remote torrent url")
expect(
    clipboardDownloads(in: pasteboardItems(on: board), fileExists: { _ in false }) == [.address(remoteTorrent)],
    "remote torrent read back from a pasteboard, items=\(pasteboardItems(on: board))"
)
board.clearContents()

let app = NSApplication.shared
app.setActivationPolicy(.regular)

let window = NSWindow(
    contentRect: NSRect(x: 0, y: 0, width: 320, height: 200),
    styleMask: [.titled],
    backing: .buffered,
    defer: false
)
let content = NSView(frame: NSRect(x: 0, y: 0, width: 320, height: 200))
window.contentView = content
window.setFrameOrigin(NSPoint(x: 40, y: 40))
window.orderFront(nil)
expect(window.contentView === content, "test window has a content view")

let mainMenu = NSMenu()
let editItem = NSMenuItem(title: "Edit", action: nil, keyEquivalent: "")
let editMenu = NSMenu(title: "Edit")
let pasteItem = NSMenuItem(title: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
pasteItem.keyEquivalentModifierMask = .command
editMenu.addItem(pasteItem)
editItem.submenu = editMenu
mainMenu.addItem(editItem)
app.mainMenu = mainMenu

let router = WorkspacePasteRouter()
let catcher = WorkspacePasteCatcher()
var downloadsStarted = 0
catcher.isEnabled = true
catcher.onPaste = { downloadsStarted += 1 }
router.sync(catcher, window: window)
expect(pasteItem.target === router, "paste menu item targets the workspace router")
expect(pasteItem.action == #selector(WorkspacePasteRouter.paste(_:)), "paste menu item calls the router")

let plain = NSView(frame: NSRect(x: 0, y: 0, width: 20, height: 20))
content.addSubview(plain)
expect(window.makeFirstResponder(plain), "focus a non-text view")
downloadsStarted = 0
expect(NSApp.sendAction(#selector(WorkspacePasteRouter.paste(_:)), to: router, from: pasteItem), "send paste")
expect(downloadsStarted == 1, "paste starts a download when no text field is focused")

final class RecordingTextView: NSTextView {
    var pasteCount = 0
    override func paste(_ sender: Any?) {
        pasteCount += 1
    }
}

let text = RecordingTextView(frame: NSRect(x: 0, y: 40, width: 200, height: 40))
content.addSubview(text)
expect(window.makeFirstResponder(text), "focus the text view")
expect(window.firstResponder === text, "text view is first responder")
downloadsStarted = 0
expect(NSApp.sendAction(#selector(WorkspacePasteRouter.paste(_:)), to: router, from: pasteItem), "send paste to text")
expect(text.pasteCount == 1, "focused text field receives the paste")
expect(downloadsStarted == 0, "focused text field does not start a download")

expect(window.makeFirstResponder(plain), "return focus to the list")
catcher.isEnabled = false
downloadsStarted = 0
expect(NSApp.sendAction(#selector(WorkspacePasteRouter.paste(_:)), to: router, from: pasteItem), "send paste with no workspace")
expect(downloadsStarted == 0, "paste does nothing when no workspace is selected")
expect(!router.validateMenuItem(pasteItem), "paste menu is disabled without a workspace")
catcher.isEnabled = true
expect(router.validateMenuItem(pasteItem), "paste menu is enabled for a workspace")

let host = NSHostingView(rootView: Text("Downloads"))
host.frame = NSRect(x: 0, y: 80, width: 120, height: 40)
content.addSubview(host)
expect(window.makeFirstResponder(host), "focus a SwiftUI hosting view")
downloadsStarted = 0
guard let pasteEvent = NSEvent.keyEvent(
    with: .keyDown,
    location: .zero,
    modifierFlags: .command,
    timestamp: 0,
    windowNumber: window.windowNumber,
    context: nil,
    characters: "v",
    charactersIgnoringModifiers: "v",
    isARepeat: false,
    keyCode: 9
) else {
    fputs("FAIL: could not build a paste key event\n", stderr)
    exit(1)
}
expect(router.handleKeyDown(pasteEvent) == nil, "command-v is consumed")
expect(downloadsStarted == 1, "command-v starts a download when a SwiftUI view is focused, responder=\(String(describing: window.firstResponder))")

downloadsStarted = 0
expect(window.makeFirstResponder(text), "focus text before a key event")
expect(router.handleKeyDown(pasteEvent) != nil, "command-v is left for the text field")
expect(downloadsStarted == 0, "command-v does not download while typing")

let shifted = NSEvent.keyEvent(
    with: .keyDown,
    location: .zero,
    modifierFlags: [.command, .shift],
    timestamp: 0,
    windowNumber: window.windowNumber,
    context: nil,
    characters: "v",
    charactersIgnoringModifiers: "v",
    isARepeat: false,
    keyCode: 9
)
expect(window.makeFirstResponder(plain), "focus the list again")
catcher.isEnabled = true
downloadsStarted = 0
expect(router.handleKeyDown(shifted!) != nil, "command-shift-v is not a download paste")
expect(downloadsStarted == 0, "paste and match style does not start a download")

window.orderOut(nil)
window.close()
router.sync(catcher, window: nil)
print("Opul paste checks passed")
