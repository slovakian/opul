import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { mockDesktop, preferences } from "./fixture";
import { library, replaceSnapshot, sidebar } from "./workspace-sidebar-fixture";

const twoBranches = () => {
  const saved = library();
  saved.workspaces[0].folders.push({
    id: "drafts",
    is_default: false,
    name: "Drafts",
    path: "/Users/test/Drafts",
  });

  return saved;
};

const attempts = (page: Page) =>
  page.evaluate(() =>
    JSON.parse(sessionStorage.getItem("test-expansion-attempts") ?? "[]")
  );

const savedPreferences = (page: Page) =>
  page.evaluate(() =>
    JSON.parse(sessionStorage.getItem("test-preferences") ?? "null")
  );

test("restores independent remembered choices without writing defaults and saves across relaunch", async ({
  page,
}) => {
  await mockDesktop(page, {
    persist: true,
    preferences: {
      ...preferences,
      workspace_expansion: { media: true, "workspace-1": false },
    },
    snapshot: library(),
  });
  await page.goto("/");

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await expect(media).toHaveAttribute("aria-expanded", "true");
  await expect(
    sidebar(page).getByRole("button", { exact: true, name: "Films" })
  ).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(() => sessionStorage.getItem("test-expansion-attempts"))
    )
    .toBeNull();
  await media.click();
  await expect
    .poll(() =>
      page.evaluate(() =>
        JSON.parse(sessionStorage.getItem("test-preferences") ?? "null")
      )
    )
    .toMatchObject({
      workspace_expansion: { media: false, "workspace-1": false },
    });
  await page.reload();
  await expect(media).toHaveAttribute("aria-expanded", "false");
});

test("restores both branches independently and retains dormant one-folder choices without default writes", async ({
  page,
}) => {
  const saved = twoBranches();
  await mockDesktop(page, {
    persist: true,
    preferences: {
      ...preferences,
      workspace_expansion: { media: false, "workspace-1": true },
    },
    snapshot: saved,
  });
  await page.goto("/");

  const first = sidebar(page).getByRole("button", {
    name: `Toggle ${saved.workspaces[0].name} folders`,
  });

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await expect(first).toHaveAttribute("aria-expanded", "true");
  await expect(media).toHaveAttribute("aria-expanded", "false");
  saved.workspaces[0].folders.pop();
  saved.sequence += 1;
  await replaceSnapshot(page, saved);
  await expect(first).toHaveCount(0);
  await media.click();
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({
      workspace_expansion: { media: true, "workspace-1": true },
    });
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
  await page.reload();
  await expect(first).toHaveCount(0);
  await expect(media).toHaveAttribute("aria-expanded", "true");
});

test("unremembered branches stay collapsed through polling and navigation without creating choices", async ({
  page,
}) => {
  const saved = twoBranches();
  await mockDesktop(page, { persist: true, snapshot: saved });
  await page.goto("/");
  await expect(
    sidebar(page).getByRole("button", { name: "Toggle Media folders" })
  ).toHaveAttribute("aria-expanded", "false");
  await sidebar(page)
    .getByRole("button", { exact: true, name: "Media workspace" })
    .click();
  saved.sequence += 1;
  saved.workspaces[1].name = "Renamed Media";
  await replaceSnapshot(page, saved);
  await expect(
    sidebar(page).getByRole("button", { name: "Toggle Renamed Media folders" })
  ).toHaveAttribute("aria-expanded", "false");
  await expect.poll(() => attempts(page)).toEqual([]);
  await page.reload();
  await expect(
    sidebar(page).getByRole("button", { name: "Toggle Renamed Media folders" })
  ).toHaveAttribute("aria-expanded", "false");
  await expect.poll(() => attempts(page)).toEqual([]);
});

test("serializes rapid choices and acknowledges only the saved choice", async ({
  page,
}) => {
  const saved = twoBranches();
  await mockDesktop(page, {
    holdFirstExpansion: true,
    persist: true,
    snapshot: saved,
  });
  await page.goto("/");

  const first = sidebar(page).getByRole("button", {
    name: `Toggle ${saved.workspaces[0].name} folders`,
  });

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await media.click();
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
  await media.click();
  await first.click();
  await expect(media).toHaveAttribute("aria-expanded", "false");
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:release-expansion"))
  );
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({
      workspace_expansion: { media: false, "workspace-1": true },
    });
  await expect
    .poll(() => attempts(page))
    .toEqual([{ media: true }, { media: false, "workspace-1": true }]);
  await expect(media).toHaveAttribute("aria-expanded", "false");
  await expect(first).toHaveAttribute("aria-expanded", "true");
  await page.reload();
  await expect(media).toHaveAttribute("aria-expanded", "false");
  await expect(first).toHaveAttribute("aria-expanded", "true");
});

test("failed writes keep session choices, do not retry on polling, and explicitly retry the latest pending choice", async ({
  page,
}) => {
  const saved = twoBranches();
  await mockDesktop(page, {
    failFirstExpansion: true,
    holdFirstExpansion: true,
    persist: true,
    preferences: {
      ...preferences,
      workspace_expansion: { media: false, "workspace-1": false },
    },
    snapshot: saved,
  });
  await page.goto("/");

  const first = sidebar(page).getByRole("button", {
    name: `Toggle ${saved.workspaces[0].name} folders`,
  });

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await media.click();
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
  await media.click();
  await first.click();
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:release-expansion"))
  );

  const error = page
    .getByRole("alert")
    .filter({ hasText: "Couldn't save workspace expansion" });

  await expect(error).toBeVisible();
  await expect(first).toHaveAttribute("aria-expanded", "true");
  await expect(media).toHaveAttribute("aria-expanded", "false");
  await expect.poll(() => savedPreferences(page)).toBeNull();
  saved.sequence += 1;
  saved.workspaces[1].name = "Renamed Media";
  await replaceSnapshot(page, saved);
  await expect(
    sidebar(page).getByRole("button", { name: "Toggle Renamed Media folders" })
  ).toHaveAttribute("aria-expanded", "false");
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
  await error.getByRole("button", { exact: true, name: "Retry" }).click();
  await expect(error).toHaveCount(0);
  await expect
    .poll(() => attempts(page))
    .toEqual([{ media: true }, { "workspace-1": true }]);
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({
      workspace_expansion: { media: false, "workspace-1": true },
    });
  await page.reload();
  await expect(first).toHaveAttribute("aria-expanded", "true");
  await expect(
    sidebar(page).getByRole("button", { name: "Toggle Renamed Media folders" })
  ).toHaveAttribute("aria-expanded", "false");
});

test("returning to the persisted choice clears the failure without a redundant write and a later change can save", async ({
  page,
}) => {
  await mockDesktop(page, {
    failFirstExpansion: true,
    persist: true,
    snapshot: library(),
  });
  await page.goto("/");

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await media.click();

  const error = page
    .getByRole("alert")
    .filter({ hasText: "Couldn't save workspace expansion" });

  await expect(error).toBeVisible();
  await expect(media).toHaveAttribute("aria-expanded", "true");
  await media.click();
  await expect(error).toHaveCount(0);
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
  await media.click();
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({ workspace_expansion: { media: true } });
  await expect
    .poll(() => attempts(page))
    .toEqual([{ media: true }, { media: true }]);
});

test("Settings drafts and display saves preserve live expansion on relaunch", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true, snapshot: library() });
  await page.goto("/");

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await media.click();
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({ workspace_expansion: { media: true } });
  await sidebar(page)
    .getByRole("button", { exact: true, name: "Settings" })
    .click();
  await page
    .getByRole("dialog", { exact: true, name: "Settings" })
    .getByRole("button", { exact: true, name: "Save changes" })
    .click();
  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await expect
    .poll(() => savedPreferences(page))
    .toMatchObject({
      aggregate_views: { all: { layout: "Table" } },
      workspace_expansion: { media: true },
    });
  await page.reload();
  await expect(media).toHaveAttribute("aria-expanded", "true");
  await expect.poll(() => attempts(page)).toEqual([{ media: true }]);
});

test("startup policy changes apply on the next launch without changing the session", async ({
  page,
}) => {
  await mockDesktop(page, {
    persist: true,
    preferences: {
      ...preferences,
      workspace_expansion: { media: true },
      workspace_expansion_startup: "collapsed",
    },
    snapshot: twoBranches(),
  });
  await page.goto("/");

  const media = sidebar(page).getByRole("button", {
    name: "Toggle Media folders",
  });

  await expect(media).toHaveAttribute("aria-expanded", "false");

  await sidebar(page)
    .getByRole("button", { exact: true, name: "Settings" })
    .click();
  const settings = page.getByRole("dialog", { exact: true, name: "Settings" });
  await settings
    .getByRole("button", { exact: true, name: "Workspaces" })
    .click();
  await settings
    .getByRole("button", {
      name: "Saved folders on startup: Start collapsed",
    })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Remember previous state" })
    .click();
  await settings
    .getByRole("button", { exact: true, name: "Save changes" })
    .click();

  await expect(media).toHaveAttribute("aria-expanded", "false");
  await page.reload();
  await expect(media).toHaveAttribute("aria-expanded", "true");

  await sidebar(page)
    .getByRole("button", { exact: true, name: "Settings" })
    .click();
  const reopened = page.getByRole("dialog", { exact: true, name: "Settings" });
  await reopened
    .getByRole("button", { exact: true, name: "Workspaces" })
    .click();
  await reopened
    .getByRole("button", {
      name: "Saved folders on startup: Remember previous state",
    })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Start collapsed" })
    .click();
  await reopened
    .getByRole("button", { exact: true, name: "Save changes" })
    .click();

  await expect(media).toHaveAttribute("aria-expanded", "true");
  await page.reload();
  await expect(media).toHaveAttribute("aria-expanded", "false");
});
