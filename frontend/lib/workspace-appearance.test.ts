import { describe, expect, it } from "vite-plus/test";

import {
  canTreatEmoji,
  emojiTreatment,
  isFolderEmoji,
} from "./workspace-appearance";

describe("workspace-appearance helpers", () => {
  describe("isFolderEmoji", () => {
    it("recognizes folder emoji with and without variation selectors", () => {
      expect(isFolderEmoji("📁")).toBe(true);
      expect(isFolderEmoji("📁\uFE0E")).toBe(true);
      expect(isFolderEmoji("📁\uFE0F")).toBe(true);
      expect(isFolderEmoji("📂")).toBe(false);
      expect(isFolderEmoji("🐼")).toBe(false);
    });
  });

  describe("canTreatEmoji", () => {
    it("allows standard colorful emojis", () => {
      expect(canTreatEmoji("📁")).toBe(true);
      expect(canTreatEmoji("🐼")).toBe(true);
      expect(canTreatEmoji("🌈")).toBe(true);
      expect(canTreatEmoji("👩🏽‍💻")).toBe(true);
    });

    it("excludes monochrome or symbol emojis with little chroma", () => {
      expect(canTreatEmoji("⚪")).toBe(false);
      expect(canTreatEmoji("⚪\uFE0E")).toBe(false);
      expect(canTreatEmoji("⚪\uFE0F")).toBe(false);
      expect(canTreatEmoji("☠")).toBe(false);
      expect(canTreatEmoji("☠\uFE0F")).toBe(false);
      expect(canTreatEmoji("⬛")).toBe(false);
      expect(canTreatEmoji("🔘")).toBe(false);
    });
  });

  describe("emojiTreatment", () => {
    const autoFolderTreatment = {
      brightness: 100,
      hue: 45,
      preset_id: "gold",
      saturation: 120,
      version: 1,
    };

    it("returns null for untreatable emoji regardless of preference", () => {
      expect(
        emojiTreatment("⚪", {
          automatic_folder: autoFolderTreatment,
          preference: { kind: "Automatic" },
        })
      ).toBeNull();

      expect(
        emojiTreatment("⚪", {
          automatic_folder: autoFolderTreatment,
          preference: {
            brightness: 90,
            hue: 100,
            kind: "Custom",
            saturation: 150,
          },
        })
      ).toBeNull();
    });

    it("applies automatic treatment only for folder emoji", () => {
      expect(
        emojiTreatment("📁", {
          automatic_folder: autoFolderTreatment,
          preference: { kind: "Automatic" },
        })
      ).toEqual(autoFolderTreatment);

      expect(
        emojiTreatment("🐼", {
          automatic_folder: autoFolderTreatment,
          preference: { kind: "Automatic" },
        })
      ).toBeNull();
    });

    it("returns null for Original preference", () => {
      expect(
        emojiTreatment("📁", {
          automatic_folder: autoFolderTreatment,
          preference: { kind: "Original" },
        })
      ).toBeNull();
    });

    it("returns preset treatment for eligible emoji", () => {
      const preset = {
        brightness: 90,
        hue: 15,
        preset_id: "coral",
        saturation: 110,
        version: 1,
      };

      expect(
        emojiTreatment("🐼", {
          automatic_folder: autoFolderTreatment,
          preference: { kind: "Preset", treatment: preset },
        })
      ).toEqual(preset);
    });

    it("returns custom preference parameters for eligible emoji", () => {
      const custom = {
        brightness: 95,
        hue: 120,
        kind: "Custom" as const,
        saturation: 180,
      };

      expect(
        emojiTreatment("🐼", {
          automatic_folder: autoFolderTreatment,
          preference: custom,
        })
      ).toEqual(custom);
    });
  });
});
