import AppKit
import OpulCore
import SwiftUI
@preconcurrency import WebKit

struct GuidedExtractionSheet: View {
    @Environment(\.dismiss) private var dismiss
    let library: DownloadLibrary

    @State private var browser = GuidedBrowserModel()
    @State private var address = ""
    @State private var selectedKind: GuidedMediaKind?

    var body: some View {
        VStack(spacing: 0) {
            navigationBar
            Divider()
            HSplitView {
                GuidedWebView(model: browser)
                    .frame(minWidth: 600, minHeight: 500)
                resultList
                    .frame(minWidth: 280, idealWidth: 340, maxWidth: 420)
            }
            Divider()
            HStack {
                Text("Play or reveal media on the page. Opul detects video, audio, streams, subtitles, and images loaded by the browser.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Done") { dismiss() }
                    .keyboardShortcut(.defaultAction)
            }
            .padding(12)
        }
        .frame(minWidth: 940, minHeight: 620)
        .onAppear {
            if address.isEmpty {
                address = browser.address
            }
        }
        .task {
            while !Task.isCancelled {
                do {
                    try await Task.sleep(for: .seconds(15))
                } catch {
                    return
                }
                browser.refreshCandidates()
            }
        }
    }

    private var navigationBar: some View {
        HStack(spacing: 8) {
            Button("Back", systemImage: "chevron.left") { browser.goBack() }
                .labelStyle(.iconOnly)
                .disabled(!browser.canGoBack)
            Button("Forward", systemImage: "chevron.right") { browser.goForward() }
                .labelStyle(.iconOnly)
                .disabled(!browser.canGoForward)
            Button("Reload", systemImage: "arrow.clockwise") { browser.reload() }
                .labelStyle(.iconOnly)

            TextField("Website address", text: $address)
                .textFieldStyle(.roundedBorder)
                .onSubmit { browser.load(address) }

            Button("Go") { browser.load(address) }
                .buttonStyle(.borderedProminent)
            Button("Clear Results") { browser.clearCandidates() }
                .disabled(browser.candidates.isEmpty)
            Button("Clear Login Data") {
                Task {
                    await browser.clearWebsiteData()
                    library.clearGuidedLoginData()
                }
            }
        }
        .padding(12)
        .onChange(of: browser.address) { _, value in
            address = value
        }
    }

    private var resultList: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Detected Media")
                    .font(.headline)
                Spacer()
                if browser.isLoading {
                    ProgressView()
                        .controlSize(.small)
                }
                Text(filteredCandidates.count.formatted())
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            }
            .padding(12)

            Picker("Kind", selection: $selectedKind) {
                Text("All").tag(nil as GuidedMediaKind?)
                ForEach(visibleKinds, id: \.self) { kind in
                    Text(kind.title).tag(kind as GuidedMediaKind?)
                }
            }
            .pickerStyle(.segmented)
            .padding(.horizontal, 12)
            .padding(.bottom, 8)

            Divider()

            if filteredCandidates.isEmpty {
                ContentUnavailableView(
                    "No Media Yet",
                    systemImage: "waveform.path.ecg",
                    description: Text("Browse the page and start playback to expose its media requests.")
                )
            } else {
                List(filteredCandidates, id: \.id) { candidate in
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(candidate.kind.title, systemImage: candidate.kind.symbolName)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Spacer()
                            Button("Download") {
                                add(candidate)
                            }
                            .buttonStyle(.borderedProminent)
                            .controlSize(.small)
                            .disabled(candidate.isExpired)
                        }
                        Text(candidate.title)
                            .lineLimit(2)
                        Text(candidate.url)
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                            .lineLimit(2)
                            .truncationMode(.middle)
                            .textSelection(.enabled)
                        HStack(spacing: 8) {
                            Text(candidate.reason)
                            if candidate.occurrences > 1 {
                                Text("Seen \(candidate.occurrences)×")
                            }
                            if candidate.isExpired {
                                Text("Expired — play again")
                                    .foregroundStyle(.orange)
                            }
                        }
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    }
                    .padding(.vertical, 4)
                }
                .listStyle(.inset)
            }

            if let error = browser.errorMessage {
                Text(error)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .padding(12)
            }
        }
    }

    private var visibleKinds: [GuidedMediaKind] {
        [.video, .audio, .stream, .subtitle, .image]
    }

    private var filteredCandidates: [GuidedMediaCandidate] {
        guard let selectedKind else { return browser.candidates }
        return browser.candidates.filter { $0.kind == selectedKind }
    }

    private func add(_ candidate: GuidedMediaCandidate) {
        Task {
            let cookies = await browser.netscapeCookieJar()
            let error = library.addGuidedDownload(
                candidate: candidate,
                session: browser.extractionSessionForDownload,
                cookies: cookies,
                userAgent: browser.userAgent
            )
            browser.reportDownloadError(error)
        }
    }
}
typealias GuidedMediaCandidate = GuidedCandidate

private struct GuidedBasicAuthorization {
    let origin: String
    let pathPrefix: String
    let value: String
}

private struct GuidedNavigationContext {
    let frameURL: URL?
    let isMainFrame: Bool
    let headers: [GuidedHttpHeader]
}

@Observable
final class GuidedBrowserModel: NSObject, WKNavigationDelegate, WKUIDelegate, WKScriptMessageHandler {
    private static let initialAddress = "https://"
    private static let websiteDataStore = WKWebsiteDataStore(
        forIdentifier: UUID(uuidString: "8E932C76-5547-46CA-9B71-6988DE4582EF")!
    )

    @ObservationIgnored private let extractionSession = GuidedExtractionSession()
    @ObservationIgnored private(set) lazy var webView: WKWebView = makeWebView()
    @ObservationIgnored private var popupWindows: [ObjectIdentifier: GuidedPopupWindowController] = [:]
    @ObservationIgnored private var basicAuthorizations: [GuidedBasicAuthorization] = []
    @ObservationIgnored private var navigationContexts: [String: [GuidedNavigationContext]] = [:]
    private(set) var address = initialAddress
    private(set) var canGoBack = false
    private(set) var canGoForward = false
    private(set) var isLoading = false
    private(set) var candidates: [GuidedMediaCandidate] = []
    private(set) var errorMessage: String?
    private(set) var userAgent: String?
    var extractionSessionForDownload: GuidedExtractionSession { extractionSession }

    func load(_ value: String) {
        var normalized = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return }
        if !normalized.contains("://") {
            normalized = "https://" + normalized
        }
        guard let url = URL(string: normalized), ["http", "https"].contains(url.scheme?.lowercased()) else {
            errorMessage = "Enter a valid HTTP or HTTPS address."
            return
        }
        errorMessage = nil
        webView.load(URLRequest(url: url))
    }

    func goBack() { webView.goBack() }
    func goForward() { webView.goForward() }
    func reload() { webView.reload() }
    func clearCandidates() {
        extractionSession.clear()
        candidates.removeAll()
        errorMessage = nil
    }

    func clearWebsiteData() async {
        let openPopups = Array(popupWindows.values)
        popupWindows.removeAll()
        openPopups.forEach { $0.close() }
        basicAuthorizations.removeAll()
        navigationContexts.removeAll()
        await withCheckedContinuation { continuation in
            Self.websiteDataStore.removeData(
                ofTypes: WKWebsiteDataStore.allWebsiteDataTypes(),
                modifiedSince: .distantPast
            ) {
                continuation.resume()
            }
        }
        clearCandidates()
        webView.loadHTMLString("", baseURL: nil)
        address = Self.initialAddress
    }

    func netscapeCookieJar() async -> String {
        let cookies = await withCheckedContinuation { continuation in
            webView.configuration.websiteDataStore.httpCookieStore.getAllCookies { cookies in
                continuation.resume(returning: cookies)
            }
        }
        let header = "# Netscape HTTP Cookie File\n# Created by Opul guided extraction"
        let rows = cookies.map { cookie in
            let httpOnly = cookie.isHTTPOnly ? "#HttpOnly_" : ""
            let includeSubdomains = cookie.domain.hasPrefix(".") ? "TRUE" : "FALSE"
            let secure = cookie.isSecure ? "TRUE" : "FALSE"
            let expires = Int(cookie.expiresDate?.timeIntervalSince1970 ?? 0)
            return "\(httpOnly)\(Self.cookieField(cookie.domain))\t\(includeSubdomains)\t\(Self.cookieField(cookie.path))\t\(secure)\t\(expires)\t\(Self.cookieField(cookie.name))\t\(Self.cookieField(cookie.value))"
        }
        return ([header] + rows).joined(separator: "\n")
    }

    func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation?) {
        if webView === self.webView {
            isLoading = true
            errorMessage = nil
            updateNavigationState(webView)
        }
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation?) {
        if webView === self.webView {
            isLoading = false
            updateNavigationState(webView)
            webView.evaluateJavaScript("navigator.userAgent") { [weak self] value, _ in
                self?.userAgent = value as? String
            }
        }
    }

    func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy) -> Void
    ) {
        if navigationAction.targetFrame != nil {
            let context = GuidedNavigationContext(
                frameURL: navigationAction.sourceFrame.request.url,
                isMainFrame: navigationAction.targetFrame?.isMainFrame ?? false,
                headers: Self.headers(for: navigationAction.request)
            )
            rememberNavigationContext(context, for: navigationAction.request.url)
            observe(
                request: navigationAction.request,
                frameURL: context.frameURL,
                isMainFrame: context.isMainFrame,
                source: .navigation,
                initiator: "navigation"
            )
        }
        decisionHandler(.allow)
    }

    func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationResponse: WKNavigationResponse,
        decisionHandler: @escaping @MainActor @Sendable (WKNavigationResponsePolicy) -> Void
    ) {
        if let url = navigationResponse.response.url {
            let response = navigationResponse.response as? HTTPURLResponse
            let context = takeNavigationContext(
                for: url,
                isMainFrame: navigationResponse.isForMainFrame
            )
            observe(
                url: url,
                frameURL: context?.frameURL,
                isMainFrame: navigationResponse.isForMainFrame,
                source: .navigation,
                initiator: "navigation-response",
                mimeType: navigationResponse.response.mimeType,
                statusCode: response.map { UInt32(clamping: $0.statusCode) },
                contentLength: navigationResponse.response.expectedContentLength >= 0
                    ? UInt64(navigationResponse.response.expectedContentLength)
                    : nil,
                headers: context?.headers ?? []
            )
        }
        decisionHandler(.allow)
    }

    func webView(
        _ webView: WKWebView,
        didFail navigation: WKNavigation?,
        withError error: Error
    ) {
        if webView === self.webView {
            isLoading = false
            errorMessage = error.localizedDescription
            updateNavigationState(webView)
        }
    }

    func webView(
        _ webView: WKWebView,
        createWebViewWith configuration: WKWebViewConfiguration,
        for navigationAction: WKNavigationAction,
        windowFeatures: WKWindowFeatures
    ) -> WKWebView? {
        if navigationAction.targetFrame == nil {
            guard popupWindows.count < 4 else {
                errorMessage = "Opul blocked additional popups after four guided browser windows."
                return nil
            }
            observe(
                request: navigationAction.request,
                frameURL: navigationAction.sourceFrame.request.url,
                isMainFrame: false,
                source: .popup,
                initiator: "popup"
            )
            let popup = WKWebView(frame: .zero, configuration: configuration)
            popup.navigationDelegate = self
            popup.uiDelegate = self
            popup.allowsBackForwardNavigationGestures = true
            let identifier = ObjectIdentifier(popup)
            let controller = GuidedPopupWindowController(webView: popup) { [weak self] in
                self?.popupWindows.removeValue(forKey: identifier)
            }
            popupWindows[identifier] = controller
            controller.showWindow(nil)
            return popup
        }
        return nil
    }

    func webViewDidClose(_ webView: WKWebView) {
        popupWindows[ObjectIdentifier(webView)]?.close()
    }

    func webView(
        _ webView: WKWebView,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping @MainActor @Sendable (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        let method = challenge.protectionSpace.authenticationMethod
        guard [NSURLAuthenticationMethodHTTPBasic, NSURLAuthenticationMethodHTTPDigest]
            .contains(method) else {
            completionHandler(.performDefaultHandling, nil)
            return
        }
        if challenge.previousFailureCount > 0,
           let origin = Self.originKey(for: challenge.protectionSpace) {
            let challengedPath = challenge.failureResponse?.url
                .map(Self.authenticationPathPrefix(for:))
            basicAuthorizations.removeAll {
                $0.origin == origin
                    && (challengedPath == nil || $0.pathPrefix == challengedPath)
            }
        }
        if challenge.previousFailureCount == 0,
           let credential = challenge.proposedCredential,
           credential.password?.isEmpty == false {
            rememberBasicAuthorization(
                credential,
                for: challenge.protectionSpace,
                challengedURL: challenge.failureResponse?.url
            )
            completionHandler(.useCredential, credential)
            return
        }
        guard let window = webView.window else {
            completionHandler(.performDefaultHandling, nil)
            return
        }
        let alert = NSAlert()
        alert.messageText = "Sign in to \(challenge.protectionSpace.host)"
        alert.informativeText = challenge.protectionSpace.realm ?? "This page requires HTTP authentication."
        alert.addButton(withTitle: "Sign In")
        alert.addButton(withTitle: "Cancel")
        let username = NSTextField(string: challenge.proposedCredential?.user ?? "")
        username.placeholderString = "Username"
        let password = NSSecureTextField(string: "")
        password.placeholderString = "Password"
        let fields = NSStackView(views: [username, password])
        fields.orientation = .vertical
        fields.spacing = 8
        fields.frame = NSRect(x: 0, y: 0, width: 320, height: 54)
        alert.accessoryView = fields
        alert.beginSheetModal(for: window) { response in
            guard response == .alertFirstButtonReturn else {
                completionHandler(.cancelAuthenticationChallenge, nil)
                return
            }
            let credential = URLCredential(
                user: username.stringValue,
                password: password.stringValue,
                persistence: .forSession
            )
            self.rememberBasicAuthorization(
                credential,
                for: challenge.protectionSpace,
                challengedURL: challenge.failureResponse?.url
            )
            completionHandler(.useCredential, credential)
        }
    }

    func webView(
        _ webView: WKWebView,
        didFailProvisionalNavigation navigation: WKNavigation?,
        withError error: Error
    ) {
        if webView === self.webView {
            isLoading = false
            errorMessage = error.localizedDescription
            updateNavigationState(webView)
        }
    }

    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.name == "opulMedia" else {
            return
        }
        let bodies: [[String: Any]]
        if let batch = message.body as? [[String: Any]] {
            bodies = batch
        } else if let item = message.body as? [String: Any] {
            bodies = [item]
        } else {
            return
        }
        let observations = bodies.prefix(256).compactMap {
            observation(from: $0, frameInfo: message.frameInfo)
        }
        guard !observations.isEmpty else { return }
        candidates = extractionSession.observeBatch(observations: observations)
        updateLimitNotice()
    }

    private func observation(
        from body: [String: Any],
        frameInfo: WKFrameInfo
    ) -> GuidedObservation? {
        guard let rawURL = body["url"] as? String, let url = URL(string: rawURL) else {
            return nil
        }
        let source: GuidedObservationSource = switch body["source"] as? String {
        case "performance": .performance
        default: .dom
        }
        let headers = basicAuthorization(for: url).map {
            [GuidedHttpHeader(name: "Authorization", value: $0)]
        } ?? []
        return GuidedObservation(
            url: url.absoluteString,
            pageUrl: body["pageURL"] as? String ?? frameInfo.request.url?.absoluteString ?? address,
            frameUrl: frameInfo.request.url?.absoluteString,
            title: body["title"] as? String ?? webView.title ?? "",
            initiator: body["initiator"] as? String ?? "",
            mimeType: body["mimeType"] as? String,
            statusCode: Self.uint32(body["statusCode"]),
            contentLength: Self.uint64(body["contentLength"]),
            observedAtMs: UInt64(Date().timeIntervalSince1970 * 1_000),
            source: source,
            isMainFrame: frameInfo.isMainFrame,
            elementKey: body["elementKey"] as? String,
            width: Self.uint32(body["width"]),
            height: Self.uint32(body["height"]),
            headers: headers
        )
    }

    private func makeWebView() -> WKWebView {
        let controller = WKUserContentController()
        controller.add(
            WeakScriptMessageHandler(self),
            contentWorld: .defaultClient,
            name: "opulMedia"
        )
        controller.addUserScript(
            WKUserScript(
                source: Self.detectionScript,
                injectionTime: .atDocumentEnd,
                forMainFrameOnly: false,
                in: .defaultClient
            )
        )
        let configuration = WKWebViewConfiguration()
        configuration.userContentController = controller
        configuration.websiteDataStore = Self.websiteDataStore
        configuration.mediaTypesRequiringUserActionForPlayback = []
        configuration.allowsAirPlayForMediaPlayback = true
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
        let view = WKWebView(frame: .zero, configuration: configuration)
        view.navigationDelegate = self
        view.uiDelegate = self
        view.allowsBackForwardNavigationGestures = true
        return view
    }

    private func updateNavigationState(_ webView: WKWebView) {
        address = webView.url?.absoluteString ?? address
        canGoBack = webView.canGoBack
        canGoForward = webView.canGoForward
    }

    private func observe(
        request: URLRequest,
        frameURL: URL?,
        isMainFrame: Bool,
        source: GuidedObservationSource,
        initiator: String
    ) {
        guard let url = request.url else { return }
        let headers = Self.headers(for: request)
        observe(
            url: url,
            frameURL: frameURL,
            isMainFrame: isMainFrame,
            source: source,
            initiator: initiator,
            headers: headers
        )
    }

    private func observe(
        url: URL,
        pageURL: String? = nil,
        frameURL: URL?,
        isMainFrame: Bool,
        source: GuidedObservationSource,
        initiator: String,
        title: String? = nil,
        mimeType: String? = nil,
        statusCode: UInt32? = nil,
        contentLength: UInt64? = nil,
        elementKey: String? = nil,
        width: UInt32? = nil,
        height: UInt32? = nil,
        headers: [GuidedHttpHeader]
    ) {
        var headers = headers
        if !headers.contains(where: { $0.name.caseInsensitiveCompare("Authorization") == .orderedSame }),
           let authorization = basicAuthorization(for: url) {
            headers.append(GuidedHttpHeader(name: "Authorization", value: authorization))
        }
        candidates = extractionSession.observe(
            observation: GuidedObservation(
                url: url.absoluteString,
                pageUrl: pageURL ?? webView.url?.absoluteString ?? address,
                frameUrl: frameURL?.absoluteString,
                title: title ?? webView.title ?? "",
                initiator: initiator,
                mimeType: mimeType,
                statusCode: statusCode,
                contentLength: contentLength,
                observedAtMs: UInt64(Date().timeIntervalSince1970 * 1_000),
                source: source,
                isMainFrame: isMainFrame,
                elementKey: elementKey,
                width: width,
                height: height,
                headers: headers
            )
        )
        updateLimitNotice()
    }

    func refreshCandidates() {
        candidates = extractionSession.candidates()
    }

    func reportDownloadError(_ error: String?) {
        errorMessage = error
    }

    private func updateLimitNotice() {
        if extractionSession.limitReached() {
            errorMessage = "Opul is keeping the 2,000 highest-ranked media candidates from this page."
        }
    }

    private static func uint32(_ value: Any?) -> UInt32? {
        guard let number = value as? NSNumber, number.doubleValue >= 0 else { return nil }
        return UInt32(clamping: number.uint64Value)
    }

    private static func headers(for request: URLRequest) -> [GuidedHttpHeader] {
        (request.allHTTPHeaderFields ?? [:]).map {
            GuidedHttpHeader(name: $0.key, value: $0.value)
        }
    }

    private func rememberNavigationContext(_ context: GuidedNavigationContext, for url: URL?) {
        guard let key = url?.absoluteString else { return }
        if navigationContexts.count >= 128,
           navigationContexts[key] == nil,
           let oldestKey = navigationContexts.keys.first {
            navigationContexts.removeValue(forKey: oldestKey)
        }
        var contexts = navigationContexts[key] ?? []
        if contexts.count >= 8 {
            contexts.removeFirst()
        }
        contexts.append(context)
        navigationContexts[key] = contexts
    }

    private func takeNavigationContext(for url: URL, isMainFrame: Bool) -> GuidedNavigationContext? {
        let key = url.absoluteString
        guard var contexts = navigationContexts[key], !contexts.isEmpty else { return nil }
        let index = contexts.firstIndex { $0.isMainFrame == isMainFrame } ?? contexts.startIndex
        let context = contexts.remove(at: index)
        if contexts.isEmpty {
            navigationContexts.removeValue(forKey: key)
        } else {
            navigationContexts[key] = contexts
        }
        return context
    }

    private static func uint64(_ value: Any?) -> UInt64? {
        guard let number = value as? NSNumber, number.doubleValue >= 0 else { return nil }
        return number.uint64Value
    }

    private static func cookieField(_ value: String) -> String {
        value.components(separatedBy: .newlines).joined().replacingOccurrences(of: "\t", with: "")
    }

    private func rememberBasicAuthorization(
        _ credential: URLCredential,
        for protectionSpace: URLProtectionSpace,
        challengedURL: URL?
    ) {
        guard protectionSpace.authenticationMethod == NSURLAuthenticationMethodHTTPBasic,
              !protectionSpace.isProxy(),
              let username = credential.user,
              let password = credential.password,
              let origin = Self.originKey(for: protectionSpace),
              let challengedURL,
              Self.originKey(for: challengedURL) == origin,
              let encoded = "\(username):\(password)".data(using: .utf8)?.base64EncodedString() else {
            return
        }
        let pathPrefix = Self.authenticationPathPrefix(for: challengedURL)
        basicAuthorizations.removeAll {
            $0.origin == origin && $0.pathPrefix == pathPrefix
        }
        basicAuthorizations.append(
            GuidedBasicAuthorization(
                origin: origin,
                pathPrefix: pathPrefix,
                value: "Basic \(encoded)"
            )
        )
    }

    private func basicAuthorization(for url: URL) -> String? {
        guard let origin = Self.originKey(for: url) else { return nil }
        return basicAuthorizations
            .filter { authorization in
                authorization.origin == origin
                    && Self.path(url.path, isWithin: authorization.pathPrefix)
            }
            .max { $0.pathPrefix.count < $1.pathPrefix.count }?
            .value
    }

    private static func path(_ path: String, isWithin prefix: String) -> Bool {
        path == prefix
            || (path.hasPrefix(prefix)
                && (prefix.hasSuffix("/") || path.dropFirst(prefix.count).first == "/"))
    }

    private static func authenticationPathPrefix(for url: URL) -> String {
        let path = url.path.isEmpty ? "/" : url.path
        if path.hasSuffix("/") {
            return path
        }
        guard let slash = path.lastIndex(of: "/") else { return "/" }
        return String(path[...slash])
    }

    private static func originKey(for protectionSpace: URLProtectionSpace) -> String? {
        guard !protectionSpace.isProxy(),
              let scheme = protectionSpace.protocol?.lowercased() else {
            return nil
        }
        return originKey(scheme: scheme, host: protectionSpace.host, port: protectionSpace.port)
    }

    private static func originKey(for url: URL) -> String? {
        guard let scheme = url.scheme?.lowercased(), let host = url.host else { return nil }
        return originKey(scheme: scheme, host: host, port: url.port ?? 0)
    }

    private static func originKey(scheme: String, host: String, port: Int) -> String {
        let resolvedPort = port > 0 ? port : (scheme == "https" ? 443 : 80)
        return "\(scheme)://\(host.lowercased()):\(resolvedPort)"
    }

    private static let detectionScript = #"""
    (() => {
      if (window.__opulMediaObserver) return;
      const seen = new Set();
      const seenOrder = [];
      let pending = [];
      let flushScheduled = false;
      const elementIDs = new WeakMap();
      let nextElementID = 1;
      const elementKey = (node) => {
        if (!node) return null;
        let value = elementIDs.get(node);
        if (!value) {
          value = String(nextElementID++);
          elementIDs.set(node, value);
        }
        return value;
      };
      const flush = () => {
        flushScheduled = false;
        if (!pending.length) return;
        window.webkit.messageHandlers.opulMedia.postMessage(pending.splice(0, pending.length));
      };
      const enqueue = (observation) => {
        pending.push(observation);
        if (pending.length >= 128) {
          flush();
          return;
        }
        if (!flushScheduled) {
          flushScheduled = true;
          setTimeout(flush, 50);
        }
      };
      const emit = (rawURL, initiator, source, node, details = {}) => {
        if (!rawURL) return;
        let absolute;
        try { absolute = new URL(rawURL, document.baseURI).href; } catch (_) { return; }
        if (!absolute.startsWith('http://') && !absolute.startsWith('https://')) return;
        const key = `${source}|${initiator}|${elementKey(node) || ''}|${absolute}`;
        if (seen.has(key)) return;
        seen.add(key);
        seenOrder.push(key);
        if (seenOrder.length > 4000) seen.delete(seenOrder.shift());
        enqueue({
          url: absolute,
          initiator: initiator || '',
          source,
          pageURL: document.URL,
          title: document.title || '',
          elementKey: elementKey(node),
          width: details.width || null,
          height: details.height || null,
          mimeType: details.mimeType || null,
          contentLength: details.contentLength || null,
          statusCode: details.statusCode || null
        });
      };
      const scanElement = (node) => {
        if (!node || typeof node.tagName !== 'string') return;
        const tag = node.tagName.toLowerCase();
        const parentTag = node.parentElement?.tagName?.toLowerCase();
        const initiator = tag === 'source' && (parentTag === 'video' || parentTag === 'audio')
          ? parentTag
          : tag;
        const width = node.videoWidth || node.naturalWidth || node.clientWidth || 0;
        const height = node.videoHeight || node.naturalHeight || node.clientHeight || 0;
        emit(node.currentSrc || node.src, initiator, 'dom', node, {
          width,
          height,
          mimeType: node.type || null
        });
      };
      const scan = () => {
        document.querySelectorAll('video,audio,source,track,img').forEach(scanElement);
      };
      window.__opulMediaObserver = new MutationObserver(scan);
      window.__opulMediaObserver.observe(document.documentElement, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ['src', 'srcset']
      });
      if (typeof PerformanceObserver !== 'undefined') {
        try {
          const observer = new PerformanceObserver((list) => {
            list.getEntries().forEach((entry) => {
              emit(entry.name, entry.initiatorType || 'resource', 'performance', null, {
                contentLength: entry.transferSize || entry.encodedBodySize || 0,
                statusCode: entry.responseStatus || 0
              });
            });
          });
          observer.observe({type: 'resource', buffered: true});
          window.__opulPerformanceObserver = observer;
        } catch (_) {}
      }
      document.addEventListener('loadstart', (event) => scanElement(event.target), true);
      document.addEventListener('loadedmetadata', (event) => scanElement(event.target), true);
      performance.getEntriesByType('resource').forEach((entry) => {
        emit(entry.name, entry.initiatorType || 'resource', 'performance', null, {
          contentLength: entry.transferSize || entry.encodedBodySize || 0,
          statusCode: entry.responseStatus || 0
        });
      });
      setInterval(scan, 1000);
      scan();
    })();
    """#
}

private final class GuidedPopupWindowController: NSWindowController, NSWindowDelegate {
    private let onClose: () -> Void

    init(webView: WKWebView, onClose: @escaping () -> Void) {
        self.onClose = onClose
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 900, height: 620),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.contentView = webView
        window.title = "Guided Extraction Popup"
        super.init(window: window)
        window.delegate = self
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func windowWillClose(_ notification: Notification) {
        onClose()
    }
}

private final class WeakScriptMessageHandler: NSObject, WKScriptMessageHandler {
    weak var target: (any WKScriptMessageHandler)?

    init(_ target: any WKScriptMessageHandler) {
        self.target = target
    }

    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        target?.userContentController(userContentController, didReceive: message)
    }
}

private struct GuidedWebView: NSViewRepresentable {
    let model: GuidedBrowserModel

    func makeNSView(context: Context) -> WKWebView { model.webView }
    func updateNSView(_ nsView: WKWebView, context: Context) {}
}

private extension GuidedMediaKind {
    var title: String {
        switch self {
        case .video: "Video"
        case .audio: "Audio"
        case .stream: "Stream"
        case .subtitle: "Subtitle"
        case .image: "Image"
        case .unknown: "Other"
        }
    }

    var symbolName: String {
        switch self {
        case .video: "film"
        case .audio: "waveform"
        case .stream: "dot.radiowaves.left.and.right"
        case .subtitle: "captions.bubble"
        case .image: "photo"
        case .unknown: "doc"
        }
    }
}
