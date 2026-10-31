import { describe, expect, it } from "vite-plus/test";

import palette from "../../src/application/folder-treatments.json";
import { createColorSuggestions } from "./color-suggestions";

describe("createColorSuggestions", () => {
  const defaultAppearance = {
    automatic_folder: palette[0],
    preference: { kind: "Automatic" as const },
  };

  it("produces deterministic folder suggestions for the same workspace ID", () => {
    const suggestions1 = createColorSuggestions({
      appearance: defaultAppearance,
      workspaceId: "workspace-1",
    });

    const suggestions2 = createColorSuggestions({
      appearance: defaultAppearance,
      workspaceId: "workspace-1",
    });

    expect(suggestions1.folder).toEqual(suggestions2.folder);
    expect(suggestions1.folder).toHaveLength(12);
    expect(
      new Set(suggestions1.folder.map((item) => item.preset_id)).size
    ).toBe(12);
  });

  it("prioritizes saved preset at the front of suggestions", () => {
    const savedPreset = palette.at(5);

    if (!savedPreset) {
      throw new Error("Expected preset at index 5");
    }

    const suggestions = createColorSuggestions({
      appearance: {
        automatic_folder: palette[0],
        preference: { kind: "Preset", treatment: savedPreset },
      },
      workspaceId: "workspace-1",
    });

    expect(suggestions.folder[0]).toEqual(savedPreset);
    expect(suggestions.other[0]).toEqual(savedPreset);
  });

  it("includes all 12 treatments in suggestions without duplicates", () => {
    const suggestions = createColorSuggestions({
      appearance: defaultAppearance,
      workspaceId: "workspace-2",
    });

    expect(suggestions.folder).toHaveLength(palette.length);
    expect(suggestions.other).toHaveLength(palette.length);
  });
});
