import { AxeBuilder } from "@axe-core/playwright";
import type { Page } from "@playwright/test";
import { expect, test } from "@playwright/test";

import { mockDesktop, snapshot } from "./fixture";

const input = (page: Page) =>
  page.getByRole("searchbox", { name: "Search downloads or paste a link" });

test("options open without text, preserve drafts on Escape, and expose workspace formats", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "Download options" })
    .click();
  await expect(
    page.getByRole("button", { exact: true, name: "Download type: Automatic" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      exact: true,
      name: "Video format: Best available",
    })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { exact: true, name: "Open .torrent" })
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await input(page).fill("draft search");
  await page
    .getByRole("button", { exact: true, name: "Download options" })
    .click();
  await page.keyboard.press("Escape");
  await expect(input(page)).toHaveValue("draft search");
});

test("ambiguous links require a neutral type choice and cancel preserves the draft", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await input(page).fill("https://example.com/watch?id=123");
  await input(page).press("Enter");

  const dialog = page.getByRole("dialog", {
    exact: true,
    name: "Choose download type",
  });

  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByRole("button", { exact: true, name: "Download type: Choose" })
  ).toBeVisible();
  await expect(
    dialog.getByRole("button", { exact: true, name: "Download" })
  ).toBeDisabled();
  await page.keyboard.press("Escape");
  await expect(input(page)).toHaveValue("https://example.com/watch?id=123");
  await expect(input(page)).toBeFocused();
});

const options = (page: Page) =>
  page.getByRole("button", { exact: true, name: "Download options" });

const choose = async (page: Page, label: string, value: string) => {
  await page
    .getByRole("button", { name: new RegExp(`^${label}:`, "u") })
    .click();
  await page.getByRole("button", { exact: true, name: value }).click();
};

const command = (page: Page) =>
  page.evaluate(() =>
    JSON.parse(sessionStorage.getItem("test-command") ?? "null")
  );

for (const [label, mode] of [
  ["File", "Direct"],
  ["Video or playlist", "Page"],
  ["Torrent", "Torrent"],
]) {
  test(`${label} override reaches the backend and resets on success`, async ({
    page,
  }) => {
    await mockDesktop(page, { failFirstAdd: true });
    await page.goto("/");
    await input(page).fill("https://example.com/ambiguous");
    await options(page).click();
    await choose(page, "Download type", label);
    await choose(page, "Video format", "MP3 audio");
    await page.keyboard.press("Escape");
    await expect(options(page)).toHaveAttribute("data-active", "true");
    await expect(options(page)).toHaveAttribute("title", `${label}; MP3 audio`);
    await input(page).press("Enter");
    await expect(
      page.getByText("Couldn't add download. Try again.", { exact: true })
    ).toBeVisible();
    await expect(input(page)).toHaveValue("https://example.com/ambiguous");
    await expect(options(page)).toHaveAttribute("data-active", "true");
    await input(page).press("Enter");
    await expect(input(page)).toHaveValue("");
    expect(await command(page)).toMatchObject({
      format: "AudioMp3",
      kind: "add_url",
      mode,
    });
    await expect(options(page)).toHaveAttribute("data-active", "false");
  });
}

test("workspace default saves explicitly, survives reload, and initializes the next format", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true });
  await page.goto("/");
  await options(page).click();
  await choose(page, "Video format", "720p");
  expect(await command(page)).toBeNull();
  await page
    .getByRole("button", { exact: true, name: "Set as workspace default" })
    .click();
  await expect(options(page)).toHaveAttribute("data-active", "false");
  expect(await command(page)).toMatchObject({
    format: "P720",
    kind: "format",
    workspace_id: "workspace-1",
  });
  await page.reload();
  await options(page).click();
  await expect(
    page.getByRole("button", { exact: true, name: "Video format: 720p" })
  ).toBeVisible();
});

for (const path of [null, "/Users/test/sample.torrent"]) {
  test(`native torrent picker preserves draft on ${path ? "import" : "cancel"}`, async ({
    page,
  }) => {
    await mockDesktop(page, { torrentPickerPath: path });
    await page.goto("/");
    await input(page).fill("https://example.com/unfinished.zip");
    await options(page).click();
    await page
      .getByRole("button", { exact: true, name: "Open .torrent" })
      .click();
    await expect(input(page)).toHaveValue("https://example.com/unfinished.zip");
    await expect(
      page.getByRole("button", { exact: true, name: "Download link" })
    ).toBeEnabled();

    const picker = await page.evaluate(() =>
      JSON.parse(sessionStorage.getItem("test-picker") ?? "null")
    );

    expect(picker).toMatchObject({
      options: {
        filters: [{ extensions: ["torrent"], name: "Torrent" }],
        multiple: false,
      },
    });

    if (path) {
      await expect
        .poll(() => command(page))
        .toMatchObject({
          folder_id: "folder-1",
          kind: "add_torrent_file",
          path,
          workspace_id: "workspace-1",
        });
      await expect(
        page.locator('[data-slot="toast"]').filter({ hasText: "new-file.zip" })
      ).toBeVisible();
    } else {
      expect(await command(page)).toBeNull();
    }
  });
}

for (const [label, mode] of [
  ["File", "Direct"],
  ["Video or playlist", "Page"],
  ["Torrent", "Torrent"],
]) {
  test(`uncertainty choice ${label} submits the original URL`, async ({
    page,
  }) => {
    await mockDesktop(page);
    await page.goto("/");
    await input(page).fill("https://example.com/unknown");
    await input(page).press("Enter");
    await choose(page, "Download type", label);
    await page
      .getByRole("dialog", { name: "Choose download type" })
      .getByRole("button", { exact: true, name: "Download" })
      .click();
    await expect(input(page)).toHaveValue("");
    expect(await command(page)).toMatchObject({
      kind: "add_url",
      mode,
      url: "https://example.com/unknown",
    });
  });
}

test("all formats are selectable and workspace defaults follow navigation", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces.push({
    ...structuredClone(library.workspaces[0]),
    default_format: "BestMp4",
    downloads: [],
    id: "media",
    name: "Media",
  });
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "Downloads workspace" })
    .click();
  await options(page).click();

  for (const label of [
    "Best MP4",
    "1080p",
    "720p",
    "Original file",
    "MP3 audio",
    "M4A audio",
    "Best available",
  ]) {
    // Each choice updates the same popover before the next interaction.
    // oxlint-disable-next-line no-await-in-loop
    await choose(page, "Video format", label);
    // oxlint-disable-next-line no-await-in-loop
    await expect(
      page.getByRole("button", { exact: true, name: `Video format: ${label}` })
    ).toBeVisible();
  }

  await choose(page, "Video format", "720p");
  await choose(page, "Video format", "Workspace default");
  await expect(options(page)).toHaveAttribute("data-active", "false");
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { exact: true, name: "Media workspace" })
    .click();
  await options(page).click();
  await expect(
    page.getByRole("button", { exact: true, name: "Video format: Best MP4" })
  ).toBeVisible();
});

test("aggregate torrent import asks for destination and preserves the draft on completion", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces.push({
    ...structuredClone(library.workspaces[0]),
    downloads: [],
    id: "media",
    name: "Media",
  });
  await mockDesktop(page, {
    holdAdd: true,
    snapshot: library,
    torrentPickerPath: "/Users/test/sample.torrent",
  });
  await page.goto("/");
  await input(page).fill("draft search");
  await options(page).click();
  await page
    .getByRole("button", { exact: true, name: "Open .torrent" })
    .click();

  const dialog = page.getByRole("dialog", {
    exact: true,
    name: "Choose download destination",
  });

  await choose(page, "Download workspace", "Media");
  await dialog.getByRole("button", { exact: true, name: "Download" }).click();
  await expect(
    dialog.getByRole("button", { exact: true, name: "Adding…" })
  ).toBeDisabled();
  await expect(page.locator('input[type="search"]')).toHaveValue(
    "draft search"
  );
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:release-add"))
  );
  await expect(dialog).toHaveCount(0);
  await expect(input(page)).toHaveValue("draft search");
  expect(await command(page)).toMatchObject({
    kind: "add_torrent_file",
    workspace_id: "media",
  });
});

test("options and uncertainty controls fit the minimum window and remain accessible", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await mockDesktop(page);
  await page.setViewportSize({ height: 800, width: 720 });
  await page.goto("/");
  await options(page).click();
  await page.screenshot({ path: "test-results/download-options-720.png" });

  const optionsAccessibility = await new AxeBuilder({ page })
    .include('[data-slot="popover-content"]')
    .analyze();

  expect(optionsAccessibility.violations).toEqual([]);

  const bounds = await page
    .locator('[data-slot="popover-content"]')
    .boundingBox();

  expect(bounds).not.toBeNull();
  expect(bounds && bounds.x + bounds.width).toBeLessThanOrEqual(720);
  await page.keyboard.press("Escape");
  await input(page).fill("https://example.com/unknown");
  await input(page).press("Enter");
  await page.screenshot({ path: "test-results/download-type-720.png" });

  const dialogAccessibility = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(dialogAccessibility.violations).toEqual([]);
  expect(errors).toEqual([]);
});

test("aggregate destination confirmation retains the explicit video format", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces.push({
    ...structuredClone(library.workspaces[0]),
    default_format: "BestMp4",
    downloads: [],
    id: "media",
    name: "Media",
  });
  await mockDesktop(page, { snapshot: library });
  await page.goto("/");
  await input(page).fill("https://example.com/watch");
  await options(page).click();
  await choose(page, "Download type", "Video or playlist");
  await choose(page, "Video format", "MP3 audio");
  await page.keyboard.press("Escape");
  await input(page).press("Enter");
  await choose(page, "Download workspace", "Media");
  await page
    .getByRole("dialog", { exact: true, name: "Choose download destination" })
    .getByRole("button", { exact: true, name: "Download" })
    .click();
  await expect(input(page)).toHaveValue("");
  expect(await command(page)).toMatchObject({
    format: "AudioMp3",
    kind: "add_url",
    mode: "Page",
    workspace_id: "media",
  });
});
