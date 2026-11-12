import { describe, expect, it } from "vite-plus/test";

import {
  ERROR_DISPLAY_MAX,
  truncateError,
  truncateFilename,
  truncateMiddle,
  truncateUrl,
} from "./truncate";

const longTurnstile =
  "https://vimeo.com/257170085?turnstile=0.ABcdefGhIjKlMnOpQrStUvWxYz0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_extra_token_padding_to_make_this_enormous_like_the_bug_report";

describe("truncateMiddle", () => {
  it("returns short strings unchanged", () => {
    expect(truncateMiddle("hello", 10)).toBe("hello");
  });

  it("keeps head and tail around an ellipsis", () => {
    expect(truncateMiddle("abcdefghijklmnopqrstuvwxyz", 11)).toBe(
      "abcde…vwxyz"
    );
  });
});

describe("truncateUrl", () => {
  it("strips a huge query when origin and path fit", () => {
    expect(truncateUrl(longTurnstile, 64)).toBe("https://vimeo.com/257170085…");
  });

  it("middle-ellipsizes when the path itself is long", () => {
    const url =
      "https://example.com/very/long/path/segment/another/segment/file.mp4";

    const result = truncateUrl(url, 40);

    expect(result.length).toBeLessThanOrEqual(40);
    expect(result).toContain("…");
    expect(result.startsWith("https")).toBe(true);
    expect(result.endsWith(".mp4")).toBe(true);
  });

  it("leaves short URLs alone", () => {
    expect(truncateUrl("https://vimeo.com/257170085")).toBe(
      "https://vimeo.com/257170085"
    );
  });
});

describe("truncateFilename", () => {
  it("treats URL-shaped titles like URLs", () => {
    expect(truncateFilename(longTurnstile, 64)).toBe(
      "https://vimeo.com/257170085…"
    );
  });

  it("prefers a leading ellipsis plus leaf for long paths", () => {
    expect(
      truncateFilename("/Users/jason/Movies/very-long-directory/clip.mp4", 20)
    ).toBe("…clip.mp4");
  });

  it("middle-ellipsizes long bare filenames", () => {
    const name = "super-long-download-title-without-slashes.mkv";
    const result = truncateFilename(name, 24);

    expect(result.length).toBeLessThanOrEqual(24);
    expect(result).toContain("…");
  });
});

describe("truncateError", () => {
  it("uses only the first line", () => {
    expect(truncateError("Timed out\nstack frame one\nstack frame two")).toBe(
      "Timed out"
    );
  });

  it("strips huge query blobs from embedded URLs", () => {
    const message = `Download failed for ${longTurnstile}`;
    const result = truncateError(message, 120);

    expect(result).toContain("https://vimeo.com/257170085…");
    expect(result).not.toContain("turnstile=");
    expect(result.length).toBeLessThanOrEqual(120);
  });

  it("end-ellipsizes past the display max", () => {
    const message = "x".repeat(ERROR_DISPLAY_MAX + 40);
    const result = truncateError(message);

    expect(result.length).toBe(ERROR_DISPLAY_MAX);
    expect(result.endsWith("…")).toBe(true);
  });
});
