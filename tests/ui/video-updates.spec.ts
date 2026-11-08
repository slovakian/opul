import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import type { VideoToolsSnapshot } from "../../frontend/lib/video-tools";
import { mockDesktop, snapshot } from "./fixture";
import { baseDownload } from "./fixture-data";

const available: VideoToolsSnapshot = {
  keep_up_to_date: false,
  nightly_available: false,
  published_size: null,
  sequence: 1,
  state: {
    installed: { python_version: "3.13.18", runtime: 1, yt_dlp: "2026.10.26" },
    kind: "ready",
    last_checked: 1_793_000_000,
    update_flag: { kind: "available", version: "2026.11.01" },
  },
  update_available: true,
  use_nightly: false,
};

const push = async (page: Page, video: VideoToolsSnapshot) => {
  await page.evaluate((value) => {
    sessionStorage.setItem("test-video-tools", JSON.stringify(value));
    window.dispatchEvent(new Event("test:video-tools"));
  }, video);
};

const openSettings = async (page: Page) => {
  await page.getByRole("button", { exact: true, name: "Settings" }).click();

  return page.getByRole("dialog", { exact: true, name: "Settings" });
};

test("consent notification appears once across reloads and another version notifies again", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true, videoTools: available });
  await page.goto("/");

  const notification = page
    .locator('[data-slot="toast"]')
    .filter({ hasText: "Video tools update available" });

  await expect(notification).toBeVisible();
  await expect(
    notification.getByRole("button", { exact: true, name: "Update" })
  ).toBeVisible();
  const audit = await new AxeBuilder({ page }).analyze();

  expect(audit.violations).toEqual([]);
  await page.screenshot({ path: "test-results/video-update-notification.png" });
  await page.reload();
  await expect(
    page.getByRole("button", { exact: true, name: "Settings" })
  ).toBeVisible();
  await expect(notification).toHaveCount(0);
  const settings = await openSettings(page);
  await expect(
    settings.getByText("Update available", { exact: true })
  ).toBeVisible();
  await settings.getByRole("button", { exact: true, name: "Tools" }).click();
  await expect(
    settings.getByRole("button", { exact: true, name: "Update" })
  ).toBeVisible();
  await page.screenshot({ path: "test-results/video-update-available.png" });
  await settings.getByRole("button", { exact: true, name: "Done" }).click();
  await push(page, { ...available, sequence: 2 });
  await expect(notification).toHaveCount(0);

  if (available.state.kind !== "ready") {
    throw new Error("Expected Ready fixture");
  }

  await push(page, {
    ...available,
    sequence: 3,
    state: {
      ...available.state,
      update_flag: { kind: "available", version: "2026.11.02" },
    },
  });
  await expect(
    notification.getByText("2026.11.02 is available.")
  ).toBeVisible();
});

test("notification Update starts installation with progress confined to Settings", async ({
  page,
}) => {
  await mockDesktop(page, { videoTools: available });
  await page.goto("/");

  const notification = page
    .locator('[data-slot="toast"]')
    .filter({ hasText: "Video tools update available" });

  await notification
    .getByRole("button", { exact: true, name: "Update" })
    .click();
  await expect(notification).toHaveCount(0);
  await expect(page.getByText(/Updating to/u)).toHaveCount(0);
  const settings = await openSettings(page);
  await expect(
    settings.getByText("Update available", { exact: true })
  ).toHaveCount(0);
  await settings.getByRole("button", { exact: true, name: "Tools" }).click();
  await expect(
    settings.getByText("Updating to 2026.11.01…", { exact: true })
  ).toBeVisible();
  await settings
    .getByText("Updating to 2026.11.01…", { exact: true })
    .scrollIntoViewIfNeeded();
  await expect(settings.getByText("Ready", { exact: true })).toBeVisible();
  await expect(settings.getByText(/^2026\.10\.26/u)).toBeVisible();

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.screenshot({ path: "test-results/video-update-progress.png" });
});

test("automatic installation and ineligible states do not show consent notifications", async ({
  page,
}) => {
  await mockDesktop(page, {
    videoTools: { ...available, keep_up_to_date: true },
  });
  await page.goto("/");
  await expect(
    page.getByRole("button", { exact: true, name: "Settings" })
  ).toBeVisible();
  await expect(
    page.getByText("Video tools update available", { exact: true })
  ).toHaveCount(0);
  await push(page, {
    ...available,
    sequence: 2,
    state: { kind: "turned_off" },
  });
  await expect(
    page.getByText("Video tools update available", { exact: true })
  ).toHaveCount(0);
});

test("failed video keeps the original error and offers Update in its list and details", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces[0].layout = "List";
  library.workspaces[0].downloads = [
    {
      ...baseDownload,
      capabilities: { ...baseDownload.capabilities, can_pause: false },
      error_message: "ERROR: original yt-dlp error text",
      id: "failed-video",
      name: "Fixture video",
      needs_video_tools: true,
      source: "VideoPage",
      state: "Failed",
    },
    {
      ...baseDownload,
      error_message: "Direct transfer failed",
      id: "failed-direct",
      name: "Direct file",
      state: "Failed",
    },
  ];
  await mockDesktop(page, { snapshot: library, videoTools: available });
  await page.goto("/");
  const item = page.getByRole("listitem").filter({ hasText: "Fixture video" });
  await expect(
    item.getByText("ERROR: original yt-dlp error text", { exact: true })
  ).toBeVisible();
  await expect(
    item.getByText("An update may fix this", { exact: true })
  ).toBeVisible();
  await expect(
    page
      .getByRole("listitem")
      .filter({ hasText: "Direct file" })
      .getByText("An update may fix this")
  ).toHaveCount(0);
  await item
    .getByRole("button", { exact: true, name: "Fixture video" })
    .click();
  await expect(
    page
      .getByRole("alert")
      .filter({ hasText: "ERROR: original yt-dlp error text" })
  ).toBeVisible();

  const audit = await new AxeBuilder({ page })
    .include('[data-slot="video-tools-update-advice"]')
    .include(".inspector")
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.screenshot({ path: "test-results/video-update-advice.png" });
  await item.getByRole("button", { exact: true, name: "Update" }).click();
  await expect(
    page.getByText("An update may fix this", { exact: true })
  ).toHaveCount(0);
  await expect(
    item.getByText("ERROR: original yt-dlp error text", { exact: true })
  ).toBeVisible();
});
