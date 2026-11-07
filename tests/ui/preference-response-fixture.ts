import type { Page } from "@playwright/test";

import type { Preferences } from "../../frontend/lib/model";
import type { DesktopOptions, PreferenceResponse } from "./fixture-options";

declare global {
  interface Window {
    testSettingsPreferences: (
      draft: Preferences,
      current: Preferences
    ) => Preferences;
    testMarkVideoSetupShown: (preferences: Preferences) => boolean;
    testPreferenceResponse: <T>(
      kind: PreferenceResponse,
      value: T
    ) => Promise<T>;
  }
}

export const mockPreferenceResponses = async (
  page: Page,
  options: DesktopOptions
) => {
  await page.addInitScript((mockOpts) => {
    const held = new Set(mockOpts.holdPreferenceResponses);
    let readFailure = mockOpts.failPreferencesRead;

    window.addEventListener("test:resume-preferences", () => {
      readFailure = false;
    });

    window.testSettingsPreferences = (draft, current) => ({
      ...draft,
      aggregate_views: current.aggregate_views,
      video_tools_setup_shown: current.video_tools_setup_shown,
      workspace_expansion: current.workspace_expansion,
    });

    window.testMarkVideoSetupShown = (preferences) => {
      const first = !preferences.video_tools_setup_shown;
      preferences.video_tools_setup_shown = true;
      sessionStorage.setItem("test-preferences", JSON.stringify(preferences));

      return first;
    };

    window.testPreferenceResponse = async (kind, value) => {
      if (kind === "read" && readFailure) {
        throw new Error("Preferences are unavailable");
      }

      // Capture the response at commit time, before allowing a later write to finish.
      const response = structuredClone(value);

      if (held.has(kind)) {
        if (kind !== "read") {
          held.delete(kind);
        }

        sessionStorage.setItem(`test-held-${kind}`, "true");
        // oxlint-disable-next-line promise/avoid-new
        await new Promise<void>((resolve) => {
          window.addEventListener(`test:release-${kind}`, () => resolve(), {
            once: true,
          });
        });
        held.delete(kind);
        sessionStorage.removeItem(`test-held-${kind}`);
      }

      return response;
    };
  }, options);
};
