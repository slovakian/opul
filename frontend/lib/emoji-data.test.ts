import { afterEach, beforeEach, describe, expect, it } from "vite-plus/test";

import { resolveEmojiData } from "./emoji-data";

describe("resolveEmojiData", () => {
  beforeEach(() => {
    // Provide a minimal Canvas probe mock in Node test environment
    const fakeContext = {
      canvas: { height: 2, width: 2 },
      clearRect: () => {},
      fillStyle: "",
      fillText: () => {},
      font: "",
      getImageData: () => ({
        data: new Uint8ClampedArray([0, 0, 255, 255]),
      }),
      measureText: (text: string) => {
        // Flag simulation matching icon-picker.spec.ts line 425
        if (text === "🇪🇺") {
          return { width: 4 };
        }

        return { width: 2 };
      },
      textBaseline: "middle",
    };

    const mockDocument = {
      createElement: (tag: string) => {
        if (tag === "canvas") {
          return {
            getContext: () => fakeContext,
          };
        }

        return {};
      },
    };

    // SAFETY: mocking document in node environment for canvas probe
    Reflect.set(globalThis, "document", mockDocument);
  });

  afterEach(() => {
    Reflect.deleteProperty(globalThis, "document");
  });

  it("filters version ceilings and unsupported country flags", async () => {
    const data = await resolveEmojiData("en", { emojiVersion: 1 });

    expect(data.emojis.length).toBeGreaterThan(100);
    expect(
      data.emojis.filter(
        (emoji: { countryFlag?: boolean }) => emoji.countryFlag
      )
    ).toHaveLength(0);
    expect(
      Math.max(
        ...data.emojis.map((emoji: { version: number }) => emoji.version)
      )
    ).toBeLessThanOrEqual(1);
  });
});
