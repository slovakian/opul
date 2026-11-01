import type { Page } from "@playwright/test";

import type {
  Preferences,
  WorkspaceExpansion,
  WorkspaceExpansionChanges,
} from "../../frontend/lib/model";
import type { DesktopOptions } from "./fixture-options";

declare global {
  interface Window {
    testSaveWorkspaceExpansion: (
      choices: WorkspaceExpansionChanges,
      preferences: Preferences
    ) => Promise<WorkspaceExpansion>;
  }
}

export const mockWorkspaceExpansion = async (
  page: Page,
  options: DesktopOptions
) => {
  await page.addInitScript((mockOpts) => {
    let expansionAttempts = 0;

    window.testSaveWorkspaceExpansion = async (choices, currentPreferences) => {
      expansionAttempts += 1;

      const attempts = JSON.parse(
        sessionStorage.getItem("test-expansion-attempts") ?? "[]"
      );

      attempts.push(choices);
      sessionStorage.setItem(
        "test-expansion-attempts",
        JSON.stringify(attempts)
      );

      if (mockOpts.holdFirstExpansion && expansionAttempts === 1) {
        // oxlint-disable-next-line promise/avoid-new
        await new Promise<void>((resolve) => {
          window.addEventListener("test:release-expansion", () => resolve(), {
            once: true,
          });
        });
      }

      if (
        expansionAttempts <=
        (mockOpts.failExpansionAttempts ??
          (mockOpts.failFirstExpansion ? 1 : 0))
      ) {
        throw new Error("Couldn't write workspace expansion. Try again.");
      }

      const expansion = new Map(
        Object.entries(currentPreferences.workspace_expansion)
      );

      for (const [id, expanded] of Object.entries(choices)) {
        if (expanded === null) {
          expansion.delete(id);
        } else {
          expansion.set(id, expanded);
        }
      }

      currentPreferences.workspace_expansion = Object.fromEntries(expansion);

      if (mockOpts.persist) {
        sessionStorage.setItem(
          "test-preferences",
          JSON.stringify(currentPreferences)
        );
      }

      return structuredClone(currentPreferences.workspace_expansion);
    };
  }, options);
};
