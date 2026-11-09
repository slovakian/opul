import { describe, expect, it } from "vite-plus/test";

import { colorTreatmentFilter } from "./color-treatment";
import { downloadFolderColor, downloadFolderColors } from "./download-folder";

const SAMPLE_SIZE = 600;

const sampleIds = Array.from(
  { length: SAMPLE_SIZE },
  (_, index) => `download-${index}`
);

describe("downloadFolderColor", () => {
  it("returns the same color for the same download", () => {
    expect(downloadFolderColor("download-42")).toBe(
      downloadFolderColor("download-42")
    );
  });

  it("spreads downloads across every color, including untinted blue", () => {
    const counts = new Map<string, number>();

    for (const id of sampleIds) {
      const color = downloadFolderColor(id);
      counts.set(color.id, (counts.get(color.id) ?? 0) + 1);
    }

    const fairShare = SAMPLE_SIZE / downloadFolderColors.length;

    expect([...counts.keys()].toSorted()).toEqual(
      downloadFolderColors.map((color) => color.id).toSorted()
    );

    for (const count of counts.values()) {
      expect(count).toBeGreaterThan(fairShare / 2);
      expect(count).toBeLessThan(fairShare * 2);
    }
  });

  it("keeps blue as the artwork's own color", () => {
    expect(downloadFolderColors.find((color) => color.id === "blue")).toEqual({
      id: "blue",
      treatment: null,
    });
  });
});

describe("colorTreatmentFilter", () => {
  it("builds a CSS filter in hue, saturation, brightness order", () => {
    expect(
      colorTreatmentFilter({ brightness: 94, hue: 49, saturation: 400 })
    ).toBe("hue-rotate(49deg) saturate(400%) brightness(94%)");
  });
});
