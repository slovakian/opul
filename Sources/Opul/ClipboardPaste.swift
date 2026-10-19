import AppKit
import OpulClipboard
import SwiftUI

struct WorkspacePasteMonitor: NSViewRepresentable {
    var isEnabled: Bool
    let onPaste: () -> Void

    func makeCoordinator() -> WorkspacePasteCatcher {
        WorkspacePasteCatcher()
    }

    func makeNSView(context: Context) -> PasteMonitorAnchor {
        let view = PasteMonitorAnchor()
        view.onMove = { window in
            WorkspacePasteRouter.shared.sync(context.coordinator, window: window)
        }
        bind(context.coordinator)
        return view
    }

    func updateNSView(_ view: PasteMonitorAnchor, context: Context) {
        bind(context.coordinator)
        WorkspacePasteRouter.shared.sync(context.coordinator, window: view.window)
    }

    private func bind(_ catcher: WorkspacePasteCatcher) {
        catcher.isEnabled = isEnabled
        catcher.onPaste = onPaste
    }
}

final class PasteMonitorAnchor: NSView {
    var onMove: ((NSWindow?) -> Void)?

    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    override var acceptsFirstResponder: Bool { false }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        onMove?(window)
    }

    override func viewWillMove(toWindow newWindow: NSWindow?) {
        if newWindow == nil {
            onMove?(nil)
        }
        super.viewWillMove(toWindow: newWindow)
    }
}
