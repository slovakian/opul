import { describe, expect, it } from "vite-plus/test";

import { downloadMode, downloadTypes } from "./download-options";

describe("downloadOptions", () => {
  it("exports expected download types", () => {
    expect(downloadTypes).toEqual([
      { label: "Automatic", value: "Automatic" },
      { label: "File", value: "Direct" },
      { label: "Video or playlist", value: "Page" },
      { label: "Torrent", value: "Torrent" },
    ]);
  });

  describe("downloadMode", () => {
    it("detects torrent links from magnet URLs and .torrent files", () => {
      expect(downloadMode("magnet:?xt=urn:btih:123456")).toBe("Torrent");
      expect(downloadMode("https://example.com/files/archive.torrent")).toBe(
        "Torrent"
      );
    });

    it("detects direct downloads from common file extensions", () => {
      expect(downloadMode("https://example.com/file.zip")).toBe("Direct");
      expect(downloadMode("https://example.com/movie.mp4")).toBe("Direct");
      expect(downloadMode("https://example.com/installer.dmg")).toBe("Direct");
      expect(downloadMode("https://example.com/doc.pdf")).toBe("Direct");
      expect(downloadMode("https://example.com/archive.tar.gz")).toBe("Direct");
    });

    it("returns undefined for ambiguous URLs requiring uncertainty choices", () => {
      expect(downloadMode("https://example.com/watch?v=12345")).toBeUndefined();
      expect(
        downloadMode("https://example.com/ambiguous-page")
      ).toBeUndefined();
      expect(downloadMode("invalid url")).toBeUndefined();
    });
  });
});
