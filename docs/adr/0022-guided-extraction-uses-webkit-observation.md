# 0022: Guided extraction uses WebKit observation

Status: accepted

Some sites expose media only after navigation, authentication, or playback. Opul therefore includes a user-driven WebKit browser that observes media elements and browser resource timing entries while the person interacts with the page.

The browser reports bounded DOM, Resource Timing, navigation, and popup observations to Rust for validation, classification, deduplication, expiry handling, and ranking as video, audio, adaptive stream, subtitle, image, or unknown content. The public candidate omits request credentials. When the person chooses a candidate, Swift sends its ID, the WebKit user agent, and the browser cookie jar; Rust resolves the private headers and current URL from the same extraction session.

The browser uses a dedicated persistent WebKit website-data store so guided sign-ins survive navigation and app relaunches without sharing the app's default store. Cookies copied into the downloader are local to Opul, protected with owner-only file permissions, and filtered by normal domain/path/secure rules for direct requests. A Clear Login Data action removes WebKit state and queued downloader credential copies. Popups use WebKit's supplied configuration in retained auxiliary guided-browser windows, preserving their browsing context without silently opening the system browser.

Public WebKit does not expose Downie's private general subresource request/response interception API. Opul uses the facts public APIs provide and keeps those limits explicit. Basic HTTP credentials are session- and path-scoped for direct handoff. Digest remains inside WebKit. A guided selection that requires `yt-dlp` and carries Authorization or Origin is rejected because passing those headers globally could disclose them to unrelated media hosts; cookie-authenticated streams remain supported.

This is a clean-room implementation based on observed product behavior. It does not reuse Downie code, private frameworks, site rules, or bundled resources.
