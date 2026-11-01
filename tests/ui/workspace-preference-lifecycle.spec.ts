import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { mockDesktop, preferences } from "./fixture";
import type { PreferenceResponse } from "./fixture-options";
import {
  addFolder,
  additionLibrary,
  chooseMedia,
} from "./saved-folder-fixture";
import { breadcrumb, input, sidebar } from "./workspace-sidebar-fixture";

const savedPreferences = (page: Page) =>
  page.evaluate(() =>
    JSON.parse(sessionStorage.getItem("test-preferences") ?? "null")
  );

const release = async (page: Page, kind: PreferenceResponse) => {
  await expect
    .poll(() =>
      page.evaluate((key) => sessionStorage.getItem(key), `test-held-${kind}`)
    )
    .toBe("true");
  await page.evaluate((response) => {
    window.dispatchEvent(new Event(`test:release-${response}`));
  }, kind);
};

const openSettings = async (page: Page) => {
  await sidebar(page)
    .getByRole("button", { exact: true, name: "Settings" })
    .click();
  const settings = page.getByRole("dialog", { exact: true, name: "Settings" });
  await settings
    .getByRole("button", { exact: true, name: "Workspaces" })
    .click();

  return settings;
};

const selectCollapsed = async (page: Page) => {
  const settings = await openSettings(page);
  await settings
    .getByRole("button", {
      name: "Saved folders on startup: Remember previous state",
    })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Start collapsed" })
    .click();

  return settings;
};

for (const mode of ["remember", "collapsed"] as const) {
  for (const entry of ["sidebar", "search"] as const) {
    test(`${mode} delayed preferences preserve early toggles and ${entry} second-folder additions`, async ({
      page,
    }) => {
      const saved = additionLibrary(1);
      saved.workspaces[0].folders.push({
        id: "drafts",
        is_default: false,
        name: "Drafts",
        path: "/Users/test/Drafts",
      });
      saved.workspaces.push({
        ...structuredClone(saved.workspaces[0]),
        downloads: [],
        id: "other",
        name: "Other",
      });
      await mockDesktop(page, {
        folderPickerPath: "/Users/test/New reports",
        holdPreferenceResponses: ["read"],
        persist: true,
        preferences: {
          ...preferences,
          workspace_expansion: {
            media: false,
            other: true,
            "workspace-1": false,
          },
          workspace_expansion_startup: mode,
        },
        snapshot: saved,
      });
      await page.goto("/");

      const research = sidebar(page).getByRole("button", {
        name: "Toggle Research folders",
      });

      const media = sidebar(page).getByRole("button", {
        name: "Toggle Media folders",
      });

      const other = sidebar(page).getByRole("button", {
        name: "Toggle Other folders",
      });

      await research.click();
      await sidebar(page)
        .getByRole("button", { exact: true, name: "Research workspace" })
        .click();
      await input(page).fill("Report");

      if (entry === "search") {
        await chooseMedia(page);
      }

      await addFolder(page, entry);
      await expect(media).toHaveAttribute("aria-expanded", "true");
      await expect(research).toHaveAttribute("aria-expanded", "true");
      await expect
        .poll(() =>
          page.evaluate(() => sessionStorage.getItem("test-expansion-attempts"))
        )
        .toBeNull();
      await release(page, "read");
      await expect(other).toHaveAttribute(
        "aria-expanded",
        String(mode === "remember")
      );
      await expect(media).toHaveAttribute("aria-expanded", "true");
      await expect(research).toHaveAttribute("aria-expanded", "true");
      await expect(input(page)).toHaveValue("Report");
      await expect(breadcrumb(page)).toContainText(
        entry === "sidebar" ? "New reports" : "Research"
      );
      await expect
        .poll(() => savedPreferences(page))
        .toMatchObject({
          workspace_expansion: {
            media: true,
            other: true,
            "workspace-1": true,
          },
          workspace_expansion_startup: mode,
        });
      await page.reload();
      await release(page, "read");
      await expect(media).toHaveAttribute(
        "aria-expanded",
        String(mode === "remember")
      );
    });
  }
}

test("failed reads remain usable and explicit Retry preserves early intent and restores untouched choices", async ({
  page,
}) => {
  const saved = additionLibrary(2);
  saved.workspaces[0].folders.push({
    id: "drafts",
    is_default: false,
    name: "Drafts",
    path: "/Users/test/Drafts",
  });
  await mockDesktop(page, {
    failPreferencesRead: true,
    persist: true,
    preferences: {
      ...preferences,
      workspace_expansion: { media: false, "workspace-1": true },
    },
    snapshot: saved,
  });
  await page.goto("/");

  const error = page
    .getByRole("alert")
    .filter({ hasText: "Couldn't load preferences" });

  await expect(error).toBeVisible();

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await media.click();
  await sidebar(page)
    .getByRole("button", { exact: true, name: "Media workspace" })
    .click();
  await input(page).fill("Report");
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-expansion-attempts"))
    )
    .toBeNull();
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:resume-preferences"))
  );
  await error.getByRole("button", { exact: true, name: "Retry" }).click();
  await expect(error).toHaveCount(0);
  await expect(media).toHaveAttribute("aria-expanded", "true");
  await expect(
    sidebar(page).getByRole("button", { name: "Toggle Research folders" })
  ).toHaveAttribute("aria-expanded", "true");
  await expect(input(page)).toHaveValue("Report");
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({
      workspace_expansion: { media: true, "workspace-1": true },
    });
});

test("a delayed display response cannot undo newer startup settings or expansion", async ({
  page,
}) => {
  await mockDesktop(page, {
    holdPreferenceResponses: ["display"],
    persist: true,
    snapshot: additionLibrary(2),
  });
  await page.goto("/");
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  const settings = await selectCollapsed(page);
  await settings
    .getByRole("button", { exact: true, name: "Save changes" })
    .click();
  await expect(settings).toHaveCount(0);

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await media.click();
  await release(page, "display");
  await expect(media).toHaveAttribute("aria-expanded", "true");
  const reopened = await openSettings(page);
  await expect(
    reopened.getByRole("button", {
      name: "Saved folders on startup: Start collapsed",
    })
  ).toBeVisible();
  await reopened
    .getByRole("button", { exact: true, name: "Save changes" })
    .click();
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({
      aggregate_views: { all: { layout: "Table" } },
      workspace_expansion: { media: true },
      workspace_expansion_startup: "collapsed",
    });
  await page.reload();
  await expect(media).toHaveAttribute("aria-expanded", "false");
  await expect(
    page.getByRole("button", { exact: true, name: "Table view" })
  ).toHaveAttribute("aria-pressed", "true");
});

test("a delayed Settings response preserves newer expansion and display choices", async ({
  page,
}) => {
  await mockDesktop(page, {
    holdPreferenceResponses: ["settings"],
    persist: true,
    snapshot: additionLibrary(2),
  });
  await page.goto("/");
  const settings = await selectCollapsed(page);
  await settings
    .getByRole("button", { exact: true, name: "Save changes" })
    .click();
  await expect(
    settings.getByRole("button", { exact: true, name: "Saving…" })
  ).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(settings).toHaveCount(0);

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await media.click();
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await release(page, "settings");
  await expect(media).toHaveAttribute("aria-expanded", "true");
  const reopened = await openSettings(page);
  await expect(
    reopened.getByRole("button", {
      name: "Saved folders on startup: Start collapsed",
    })
  ).toBeVisible();
  await reopened.getByRole("button", { exact: true, name: "Cancel" }).click();
  await expect(
    page.getByRole("button", { exact: true, name: "Table view" })
  ).toHaveAttribute("aria-pressed", "true");
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({
      aggregate_views: { all: { layout: "Table" } },
      workspace_expansion: { media: true },
      workspace_expansion_startup: "collapsed",
    });
});

test("a delayed expansion acknowledgement preserves newer Settings and display preferences", async ({
  page,
}) => {
  await mockDesktop(page, {
    holdPreferenceResponses: ["expansion"],
    persist: true,
    snapshot: additionLibrary(2),
  });
  await page.goto("/");

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await media.click();
  const settings = await selectCollapsed(page);
  await settings
    .getByRole("button", { exact: true, name: "Save changes" })
    .click();
  await expect(settings).toHaveCount(0);
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await media.click();
  await release(page, "expansion");
  await expect(media).toHaveAttribute("aria-expanded", "false");
  const reopened = await openSettings(page);
  await expect(
    reopened.getByRole("button", {
      name: "Saved folders on startup: Start collapsed",
    })
  ).toBeVisible();
  await reopened.getByRole("button", { exact: true, name: "Cancel" }).click();
  await expect(
    page.getByRole("button", { exact: true, name: "Table view" })
  ).toHaveAttribute("aria-pressed", "true");
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({
      aggregate_views: { all: { layout: "Table" } },
      workspace_expansion: { media: false },
      workspace_expansion_startup: "collapsed",
    });
});
