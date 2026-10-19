import AppKit

/// Receives Edit ▸ Paste and ⌘V for one window.
///
/// The standard Paste item walks the responder chain and stops at the first
/// view that implements `paste:`. A SwiftUI hosting view can be that view
/// while the search field is not focused, so the command never reaches the
/// workspace. This router owns the Paste menu item and the key equivalent,
/// forwards them into a focused text field, and otherwise starts a download.
@MainActor
public final class WorkspacePasteCatcher: NSObject {
    public var isEnabled = false
    public var onPaste: () -> Void = {}
    public weak var window: NSWindow?

    public func focusedTextResponder() -> NSResponder? {
        guard let responder = window?.firstResponder else { return nil }
        if responder is NSText || responder is NSTextField {
            return responder
        }
        return nil
    }

    public func receivePaste(_ sender: Any?) {
        if let text = focusedTextResponder() {
            _ = text.perform(#selector(NSText.paste(_:)), with: sender)
            return
        }
        guard isEnabled else { return }
        onPaste()
    }
}

@MainActor
public final class WorkspacePasteRouter: NSObject {
    public static let shared = WorkspacePasteRouter()

    private final class CatcherBox {
        weak var catcher: WorkspacePasteCatcher?
        init(_ catcher: WorkspacePasteCatcher) { self.catcher = catcher }
    }

    private var catchers: [ObjectIdentifier: CatcherBox] = [:]
    private var monitor: Any?
    private var observesMenus = false

    public func sync(_ catcher: WorkspacePasteCatcher, window: NSWindow?) {
        let stale = catchers.keys.filter { key in
            guard let existing = catchers[key]?.catcher else { return true }
            return existing === catcher
        }
        for key in stale {
            catchers.removeValue(forKey: key)
        }
        if let window {
            catchers[ObjectIdentifier(window)] = CatcherBox(catcher)
            catcher.window = window
        } else {
            catcher.window = nil
        }
        installMonitorIfNeeded()
        installMenuItem()
    }

    public func handleKeyDown(_ event: NSEvent) -> NSEvent? {
        consumePasteShortcut(event) ? nil : event
    }

    private func consumePasteShortcut(_ event: NSEvent) -> Bool {
        guard isPasteShortcut(event) else { return false }
        guard let catcher = activeCatcher() else { return false }
        guard catcher.focusedTextResponder() == nil, catcher.isEnabled else { return false }
        catcher.onPaste()
        return true
    }

    @objc public func paste(_ sender: Any?) {
        if let catcher = activeCatcher() {
            catcher.receivePaste(sender)
            return
        }
        guard let responder = NSApp.keyWindow?.firstResponder,
              responder.responds(to: #selector(NSText.paste(_:))) else {
            return
        }
        _ = responder.perform(#selector(NSText.paste(_:)), with: sender)
    }

    @objc public func validateMenuItem(_ menuItem: NSMenuItem) -> Bool {
        guard isPasteItem(menuItem) else { return true }
        if let catcher = activeCatcher() {
            if catcher.focusedTextResponder() != nil {
                return pasteboardHasTextualData
            }
            return catcher.isEnabled
        }
        return NSApp.keyWindow?.firstResponder?.responds(to: #selector(NSText.paste(_:))) == true
    }

    private var pasteboardHasTextualData: Bool {
        let types = NSPasteboard.general.types ?? []
        return types.contains(.string) || types.contains(.rtf) || types.contains(.URL) || types.contains(.fileURL)
    }

    private func catcher(for window: NSWindow?) -> WorkspacePasteCatcher? {
        guard let window else { return nil }
        return catchers[ObjectIdentifier(window)]?.catcher
    }

    /// The key window's catcher. A paste check cannot always become key, so a
    /// sole registered window is used when AppKit has no key window. A key
    /// sheet is a different window and does not fall through to the workspace.
    private func activeCatcher() -> WorkspacePasteCatcher? {
        if let window = NSApp.keyWindow {
            return catcher(for: window)
        }
        let live = catchers.values.compactMap(\.catcher).filter { $0.window != nil }
        guard live.count == 1 else { return nil }
        return live[0]
    }

    private func installMonitorIfNeeded() {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self else { return event }
            nonisolated(unsafe) let keyEvent = event
            let consume = MainActor.assumeIsolated {
                self.consumePasteShortcut(keyEvent)
            }
            return consume ? nil : event
        }
        guard !observesMenus else { return }
        observesMenus = true
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(menuDidBeginTracking(_:)),
            name: NSMenu.didBeginTrackingNotification,
            object: nil
        )
    }

    @objc private func menuDidBeginTracking(_ notification: Notification) {
        installMenuItem()
    }

    private func installMenuItem() {
        guard let item = pasteMenuItem() else { return }
        item.target = self
        item.action = #selector(paste(_:))
    }

    private func pasteMenuItem() -> NSMenuItem? {
        guard let main = NSApp.mainMenu else { return nil }
        for top in main.items {
            guard let submenu = top.submenu else { continue }
            if let item = submenu.items.first(where: isPasteItem) {
                return item
            }
        }
        return nil
    }

    private func isPasteItem(_ item: NSMenuItem) -> Bool {
        item.action == #selector(NSText.paste(_:)) || item.action == #selector(paste(_:))
    }

    private func isPasteShortcut(_ event: NSEvent) -> Bool {
        guard event.type == .keyDown, !event.isARepeat else { return false }
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask).subtracting(.capsLock)
        guard flags == .command else { return false }
        return event.charactersIgnoringModifiers?.lowercased() == "v"
    }
}
