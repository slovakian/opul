import type { Page } from "@playwright/test";

import type { DesktopOptions, PreferenceResponse } from "./fixture-options";

declare global {
  interface Window {
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
