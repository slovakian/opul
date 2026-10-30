import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { mockDesktop, snapshot, preferences } from "./fixture";

const library = () => {
  const value = structuredClone(snapshot);
  value.workspaces[0].folders.push({
    id: "archive",
    is_default: false,
    name: "Archive",
    path: "/Users/test/Archive",
  });
  value.workspaces.push({
    ...structuredClone(value.workspaces[0]),
    downloads: [],
    folders: [
      {
        id: "films",
        is_default: true,
        name: "Films",
        path: "/Users/test/Films",
      },
    ],
    id: "media",
    name: "Media",
  });

  return value;
};

const navigate = async (page: Page, name: string) => {
  await page
    .getByRole("navigation", { name: "Workspace navigation" })
    .getByRole("button", { exact: true, name })
    .click();
};

const input = (page: Page) =>
  page.getByRole("searchbox", { name: "Search downloads or paste a link" });

const destination = (page: Page, name: string) =>
  page
    .locator("main > header")
    .getByRole("button", { exact: true, name: `Save location: ${name}` });

test("context defaults and cross-workspace overrides survive downloads and reset on navigation", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true, snapshot: library() });
  await page.goto("/");
  await expect(destination(page, "Choose…")).toBeVisible();
  await navigate(page, "Downloads");
  await expect(destination(page, "Downloads")).toBeVisible();
  await navigate(page, "Archive");
  await expect(destination(page, "Archive")).toBeVisible();
  await input(page).fill("search text");
  await destination(page, "Archive").click();
  const popover = page.locator('[data-slot="popover-content"]');
  await expect(popover.getByText("Media", { exact: true })).toBeVisible();
  await page.screenshot({ path: "test-results/destination-popover.png" });
  await popover
    .getByRole("button", { exact: true, name: "Films (Default)" })
    .click();
  await expect(destination(page, "Media › Films")).toBeVisible();
  await input(page).fill("https://example.com/first.zip");
  await input(page).press("Enter");
  await expect(input(page)).toHaveValue("");
  await expect(destination(page, "Media › Films")).toBeVisible();
  await input(page).fill("https://example.com/second.zip");
  await page
    .getByRole("button", { exact: true, name: "Download link" })
    .click();
  await expect(input(page)).toHaveValue("");

  const saved = await page.evaluate(() =>
    JSON.parse(sessionStorage.getItem("test-snapshot") ?? "null")
  );

  expect(saved.workspaces[1].downloads).toHaveLength(2);
  expect(
    saved.workspaces[1].downloads.map(
      (item: { folder_id: string }) => item.folder_id
    )
  ).toEqual(["films", "films"]);
  await navigate(page, "Downloads");
  await expect(destination(page, "Downloads")).toBeVisible();
  await navigate(page, "Archive");
  await expect(destination(page, "Archive")).toBeVisible();
  await navigate(page, "All downloads");
  await expect(destination(page, "Choose…")).toBeVisible();
});

test("aggregate confirmation cancels safely, remembers last used and saves the automatic workspace", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true, snapshot: library() });
  await page.goto("/");
  await input(page).fill("https://example.com/first.zip");
  await input(page).press("Enter");

  const dialog = page.getByRole("dialog", {
    exact: true,
    name: "Choose download destination",
  });

  await expect(dialog).toBeVisible();
  await page.screenshot({ path: "test-results/destination-dialog.png" });
  await expect(
    dialog.getByRole("button", {
      exact: true,
      name: "Download workspace: Downloads",
    })
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(input(page)).toHaveValue("https://example.com/first.zip");
  await expect(input(page)).toBeFocused();
  await input(page).press("Enter");
  await dialog
    .getByRole("button", { exact: true, name: "Download workspace: Downloads" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Media" })
    .click();
  await dialog.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(dialog).toHaveCount(0);
  await navigate(page, "Completed");
  await input(page).fill("https://example.com/second.zip");
  await input(page).press("Enter");
  await expect(
    dialog.getByRole("button", {
      exact: true,
      name: "Download workspace: Media",
    })
  ).toBeVisible();
  await dialog
    .getByRole("checkbox", {
      name: "Use this workspace automatically next time",
    })
    .check();
  await dialog.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(dialog).toHaveCount(0);
  await page.reload();
  await expect(destination(page, "Media › Films")).toBeVisible();
  await page.keyboard.press("Meta+,");
  const settings = page.getByRole("dialog", { exact: true, name: "Settings" });
  await settings
    .getByRole("button", { exact: true, name: "Downloads" })
    .click();
  await expect(
    settings.getByRole("button", {
      exact: true,
      name: "Aggregate destination: Use a default workspace",
    })
  ).toBeVisible();
  await settings
    .getByRole("button", {
      exact: true,
      name: "Aggregate destination: Use a default workspace",
    })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Ask each time" })
    .click();
  await settings
    .getByRole("button", { exact: true, name: "Save changes" })
    .click();
  await expect(destination(page, "Choose…")).toBeVisible();
  await page.reload();
  await expect(destination(page, "Choose…")).toBeVisible();
});

test("new locations target the selected workspace and keep an accessible compact header", async ({
  page,
}) => {
  await mockDesktop(page, {
    folderPickerPath: "/Users/test/Movies",
    persist: true,
    snapshot: library(),
  });
  await page.goto("/");
  await destination(page, "Choose…").click();
  await page
    .getByRole("button", { exact: true, name: "Films (Default)" })
    .click();
  await destination(page, "Media › Films").click();
  await page
    .getByRole("button", { exact: true, name: "Add new location…" })
    .click();
  await expect(destination(page, "Media › Movies")).toBeVisible();

  const saved = await page.evaluate(() =>
    JSON.parse(sessionStorage.getItem("test-snapshot") ?? "null")
  );

  expect(saved.workspaces[1].folders[1].path).toBe("/Users/test/Movies");
  expect(saved.workspaces[0].folders).toHaveLength(2);
  await destination(page, "Media › Movies").click();
  await expect(
    page
      .locator('[data-slot="popover-content"]')
      .getByRole("button", { exact: true, name: "Movies" })
  ).toHaveAttribute("data-active");
  await page.keyboard.press("Escape");

  const accessibility = await new AxeBuilder({ page })
    .include("main > header")
    .analyze();

  expect(accessibility.violations).toEqual([]);
  await page.setViewportSize({ height: 800, width: 720 });
  await page.screenshot({ path: "test-results/destination-720.png" });
});

test("single workspace defaults stay visible in empty, search, and link states", async ({
  page,
}) => {
  await mockDesktop(page, { empty: true });
  await page.goto("/");
  await expect(destination(page, "Downloads › Downloads")).toBeVisible();
  await input(page).fill("search text");
  await expect(destination(page, "Downloads › Downloads")).toBeVisible();
  await input(page).fill("https://example.com/new.zip");
  await input(page).press("Enter");
  await expect(
    page.getByRole("dialog", {
      exact: true,
      name: "Choose download destination",
    })
  ).toHaveCount(0);
  await expect(input(page)).toHaveValue("");
});

test("Settings can choose an automatic workspace without altering view defaults", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true, snapshot: library() });
  await page.goto("/");
  await page.keyboard.press("Meta+,");
  const settings = page.getByRole("dialog", { exact: true, name: "Settings" });
  await settings
    .getByRole("button", { exact: true, name: "Downloads" })
    .click();
  await settings
    .getByRole("button", {
      exact: true,
      name: "Aggregate destination: Ask each time",
    })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Use a default workspace" })
    .click();
  await settings
    .getByRole("button", { exact: true, name: "Default workspace: Downloads" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Media" })
    .click();
  await settings
    .getByRole("button", { exact: true, name: "Save changes" })
    .click();
  await expect(destination(page, "Media › Films")).toBeVisible();
  await input(page).fill("https://example.com/new.zip");
  await input(page).press("Enter");
  await expect(input(page)).toHaveValue("");
  await expect(
    page.getByRole("dialog", {
      exact: true,
      name: "Choose download destination",
    })
  ).toHaveCount(0);
  await navigate(page, "Archive");
  await expect(destination(page, "Archive")).toBeVisible();
  await navigate(page, "In progress");
  await expect(destination(page, "Media › Films")).toBeVisible();
  await page.reload();
  await expect(destination(page, "Media › Films")).toBeVisible();
});

test("an unavailable configured workspace falls back to prompting", async ({
  page,
}) => {
  await mockDesktop(page, {
    preferences: {
      ...preferences,
      aggregate_destination: { kind: "workspace", workspace_id: "deleted" },
    },
    snapshot: library(),
  });
  await page.goto("/");
  await expect(destination(page, "Choose…")).toBeVisible();
  await input(page).fill("https://example.com/new.zip");
  await input(page).press("Enter");

  const dialog = page.getByRole("dialog", {
    exact: true,
    name: "Choose download destination",
  });

  await expect(
    dialog.getByRole("button", {
      exact: true,
      name: "Download workspace: Downloads",
    })
  ).toBeVisible();
  await dialog
    .getByRole("button", { exact: true, name: "Save location: Downloads" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Archive" })
    .click();

  const accessibility = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(accessibility.violations).toEqual([]);
  await dialog.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(destination(page, "Downloads › Archive")).toBeVisible();
});

test("failed preference saves keep the confirmation and draft available for retry", async ({
  page,
}) => {
  await mockDesktop(page, {
    failFirstPreferences: true,
    persist: true,
    snapshot: library(),
  });
  await page.goto("/");
  await input(page).fill("https://example.com/new.zip");
  await input(page).press("Enter");

  const dialog = page.getByRole("dialog", {
    exact: true,
    name: "Choose download destination",
  });

  await dialog
    .getByRole("checkbox", {
      name: "Use this workspace automatically next time",
    })
    .check();
  await dialog.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(
    page.getByText("Couldn't write preferences. Try again.", { exact: true })
  ).toBeVisible();
  await expect(dialog).toBeVisible();
  await expect(
    page.locator('input[aria-label="Search downloads or paste a link"]')
  ).toHaveValue("https://example.com/new.zip");
  expect(
    await page.evaluate(() => sessionStorage.getItem("test-snapshot"))
  ).toBeNull();
  await dialog.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(input(page)).toHaveValue("");
});

test("failed additions retain the selected folder in the confirmation", async ({
  page,
}) => {
  await mockDesktop(page, {
    failFirstAdd: true,
    persist: true,
    snapshot: library(),
  });
  await page.goto("/");
  await input(page).fill("https://example.com/new.zip");
  await input(page).press("Enter");

  const dialog = page.getByRole("dialog", {
    exact: true,
    name: "Choose download destination",
  });

  await dialog
    .getByRole("button", { exact: true, name: "Download workspace: Downloads" })
    .click();
  await page
    .locator('[data-slot="popover-content"]')
    .getByRole("button", { exact: true, name: "Media" })
    .click();
  await dialog.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(
    page.getByText("Couldn't add download. Try again.", { exact: true })
  ).toBeVisible();
  await expect(
    dialog.getByRole("button", {
      exact: true,
      name: "Download workspace: Media",
    })
  ).toBeVisible();
  await expect(
    page.locator('input[aria-label="Search downloads or paste a link"]')
  ).toHaveValue("https://example.com/new.zip");
  await dialog.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(destination(page, "Media › Films")).toBeVisible();
});

for (const picker of [
  { folderPickerPath: null },
  { folderPickerError: "Folder access unavailable" },
]) {
  test(`native picker cancel or failure preserves the destination: ${JSON.stringify(picker)}`, async ({
    page,
  }) => {
    await mockDesktop(page, { persist: true, snapshot: library(), ...picker });
    await page.goto("/");
    await input(page).fill("keep this draft");
    await destination(page, "Choose…").click();
    await page
      .getByRole("button", { exact: true, name: "Add new location…" })
      .click();
    await expect(destination(page, "Choose…")).toBeVisible();
    await expect(input(page)).toHaveValue("keep this draft");

    if ("folderPickerError" in picker) {
      await expect(
        page.getByText("Folder access unavailable", { exact: true })
      ).toBeVisible();
    }

    expect(
      await page.evaluate(() => sessionStorage.getItem("test-snapshot"))
    ).toBeNull();
  });
}

test("a preferences read failure leaves contextual downloads and aggregate confirmation usable", async ({
  page,
}) => {
  await mockDesktop(page, {
    failPreferencesRead: true,
    persist: true,
    snapshot: library(),
  });
  await page.goto("/");
  await navigate(page, "Archive");
  await expect(destination(page, "Archive")).toBeEnabled();
  await input(page).fill("https://example.com/context.zip");
  await input(page).press("Enter");
  await expect(input(page)).toHaveValue("");
  await navigate(page, "All downloads");
  await expect(destination(page, "Choose…")).toBeEnabled();
  await input(page).fill("https://example.com/aggregate.zip");
  await input(page).press("Enter");

  const dialog = page.getByRole("dialog", {
    exact: true,
    name: "Choose download destination",
  });

  await expect(
    dialog.getByRole("checkbox", {
      name: "Use this workspace automatically next time",
    })
  ).toBeDisabled();
  await dialog.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(input(page)).toHaveValue("");
});
