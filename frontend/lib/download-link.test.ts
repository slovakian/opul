import { describe, expect, it } from "vite-plus/test";

import { isDownloadLink } from "./download-link";

describe("isDownloadLink", () => {
  it("recognizes valid web download links", () => {
    expect(isDownloadLink("https://example.com/file.zip")).toBe(true);
    expect(isDownloadLink("http://example.com/video.mp4")).toBe(true);
    expect(
      isDownloadLink("https://sub.domain.org/path/resource?query=1#frag")
    ).toBe(true);
  });

  it("recognizes valid magnet links with hex and base32 hashes", () => {
    expect(
      isDownloadLink(
        "magnet:?xt=urn:btih:da39a3ee5e6b4b0d3255bfef95601890afd80709"
      )
    ).toBe(true);
    expect(
      isDownloadLink(
        "magnet:?xt=urn:btih:N423XZ2P3JNZ27756R7PMMW3G4S5R67A&dn=test"
      )
    ).toBe(true);
  });

  it("rejects invalid magnet links without valid xt hash", () => {
    expect(isDownloadLink("magnet:?dn=test")).toBe(false);
    expect(isDownloadLink("magnet:?xt=urn:other:123")).toBe(false);
  });

  it("rejects non-links, search queries, and whitespace-containing strings", () => {
    expect(isDownloadLink("")).toBe(false);
    expect(isDownloadLink("not a link")).toBe(false);
    expect(isDownloadLink("https://example.com/has space")).toBe(false);
    expect(isDownloadLink("ftp://example.com/file.zip")).toBe(false);
    expect(isDownloadLink("just searching for downloads")).toBe(false);
  });

  it("handles leading and trailing whitespace by trimming", () => {
    expect(isDownloadLink("  https://example.com/file.zip  ")).toBe(true);
  });
});
