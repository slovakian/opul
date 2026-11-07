import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import type { VideoToolsSnapshot } from "../../frontend/lib/video-tools";
import { mockDesktop, snapshot } from "./fixture";

const initial: VideoToolsSnapshot = {
  keep_up_to_date: true,
  nightly_available: false,
  published_size: 23_400_000,
  sequence: 1,
  state: { kind: "not_set_up" },
  update_available: false,
  use_nightly: false,
};

const settingUp: VideoToolsSnapshot = {
  ...initial,
  state: { kind: "setting_up", steps: [] },
};

const input = (page: Page) =>
  page.getByRole("searchbox", { name: "Search downloads or paste a link" });

const consent = (page: Page) =>
  page.getByRole("dialog", { name: "Video downloads need video tools" });

const submitVideo = async (page: Page) => {
  await page
    .getByRole("button", { exact: true, name: "Download options" })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Download type: Automatic" })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Video or playlist" })
    .click();
  await page.keyboard.press("Escape");
  await input(page).fill("https://example.com/playlist");
  await page
    .getByRole("button", { exact: true, name: "Download link" })
    .click();
};

const push = async (page: Page, video: VideoToolsSnapshot) => {
  await page.evaluate((value) => {
    sessionStorage.setItem("test-video-tools", JSON.stringify(value));
    window.dispatchEvent(new Event("test:video-tools"));
  }, video);
};

const showLibrary = async (page: Page) => {
  await page.getByRole("button", { exact: true, name: "Clear input" }).click();
};

test("video consent displays the size, drops Not now and Escape, and passes axe", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true, videoTools: initial });
  await page.goto("/");
  await submitVideo(page);
  await expect(consent(page)).toContainText("23.4 MB");

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  await consent(page)
    .getByRole("button", { exact: true, name: "Not now" })
    .click();
  await expect(input(page)).toHaveValue("");
  expect(
    await page.evaluate(() => sessionStorage.getItem("test-add-attempts"))
  ).toBeNull();
  await submitVideo(page);
  await expect(consent(page)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(consent(page)).toHaveCount(0);
  expect(
    await page.evaluate(() => sessionStorage.getItem("test-add-attempts"))
  ).toBeNull();
});

test("Download creates one waiting playlist and keeps its folder and format across restart", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  library.workspaces[0].folders.push({
    id: "films",
    is_default: false,
    name: "Films",
    path: "/Users/test/Films",
  });
  library.workspaces[0].default_format = "AudioM4a";
  await mockDesktop(page, {
    persist: true,
    snapshot: library,
    videoTools: initial,
  });
  await page.goto("/");
  await page
    .locator("main > header")
    .getByRole("button", { name: /^Save location:/u })
    .click();
  await page.getByRole("button", { exact: true, name: "Films" }).click();
  await submitVideo(page);
  await consent(page)
    .getByRole("button", { exact: true, name: "Download" })
    .click();
  await expect(consent(page)).toHaveCount(0);
  await expect(
    page.getByText("Waiting for video tools…", { exact: true })
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      JSON.parse(sessionStorage.getItem("test-command") ?? "null")
    )
  ).toMatchObject({
    folder_id: "films",
    format: "AudioM4a",
    kind: "add_url",
    mode: "Page",
  });
  await page.reload();
  await expect(page.getByText("Waiting video", { exact: true })).toHaveCount(1);
  await expect(
    page.getByText("Waiting for video tools…", { exact: true })
  ).toBeVisible();
  await page.screenshot({ path: "test-results/waiting-video.png" });

  const audit = await new AxeBuilder({ page })
    .exclude("[data-base-ui-focus-guard]")
    .analyze();

  expect(audit.violations).toEqual([]);
  await push(page, {
    ...initial,
    sequence: 10,
    state: {
      installed: { python_version: "3.13.8", runtime: 1, yt_dlp: "2026.10.26" },
      kind: "ready",
      last_checked: null,
      update_flag: { kind: "none" },
    },
  });
  await expect(
    page.getByText("Waiting for video tools…", { exact: true })
  ).toHaveCount(0);
  await expect(page.getByText("Waiting video", { exact: true })).toHaveCount(1);
});

test("Setting up accepts without another consent dialog and Not working offers projected Try again", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true, videoTools: settingUp });
  await page.goto("/");
  await submitVideo(page);
  await expect(input(page)).toHaveValue("");
  await expect(consent(page)).toHaveCount(0);
  await expect(
    page.getByText("Waiting for video tools…", { exact: true })
  ).toBeVisible();

  const inProgress = page
    .getByRole("navigation", { name: "Workspace navigation" })
    .getByRole("button", { name: /^In progress/u });

  await expect(inProgress.locator("..")).toContainText("2");
  await inProgress.click();
  await expect(
    page.getByText("Waiting for video tools…", { exact: true })
  ).toBeVisible();
  await push(page, {
    ...initial,
    sequence: 8,
    state: {
      details: "Connection lost",
      kind: "not_working",
      reason: "download",
    },
  });
  await expect(
    page.getByText("Video tools aren't working", { exact: true })
  ).toBeVisible();

  const audit = await new AxeBuilder({ page })
    .exclude("[data-base-ui-focus-guard]")
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.getByRole("button", { exact: true, name: "Try again" }).click();
  await expect(
    page.getByText("Waiting for video tools…", { exact: true })
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      JSON.parse(sessionStorage.getItem("test-video-tools-operation") ?? "null")
    )
  ).toEqual({ kind: "retry" });
});

test("Don't show this again persists Turned off and dims Video without setup prompts", async ({
  page,
}) => {
  await mockDesktop(page, { persist: true, videoTools: initial });
  await page.goto("/");
  await submitVideo(page);
  await consent(page)
    .getByRole("checkbox", { name: "Don't show this again" })
    .check();
  await consent(page)
    .getByRole("button", { exact: true, name: "Not now" })
    .click();
  await page.reload();
  await input(page).fill("https://example.com/unknown");
  await page
    .getByRole("button", { exact: true, name: "Download link" })
    .click();
  const dialog = page.getByRole("dialog", { name: "Choose download type" });
  await expect(dialog).toContainText(
    "Needs video tools. Turn on in Settings › Tools."
  );
  await dialog
    .getByRole("button", { exact: true, name: "Download type: Choose" })
    .click();
  await expect(
    page.getByRole("button", { exact: true, name: "Video or playlist" })
  ).toBeDisabled();

  const audit = await new AxeBuilder({ page })
    .exclude("[data-base-ui-focus-guard]")
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await showLibrary(page);
  expect(
    await page.evaluate(() => sessionStorage.getItem("test-add-attempts"))
  ).toBeNull();
  await expect(consent(page)).toHaveCount(0);
});

test("a retried video in Turned off waits and offers projected Set up", async ({
  page,
}) => {
  const library = structuredClone(snapshot);
  const [download] = library.workspaces[0].downloads;
  download.source = "VideoPage";
  download.state = "Failed";
  download.capabilities.can_pause = false;
  download.capabilities.can_retry = true;
  await mockDesktop(page, {
    snapshot: library,
    videoTools: { ...initial, state: { kind: "turned_off" } },
  });
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: `Retry ${download.name}` })
    .click();
  await expect(
    page.getByText("Needs video tools", { exact: true })
  ).toBeVisible();
  await expect(consent(page)).toHaveCount(0);

  const audit = await new AxeBuilder({ page })
    .exclude("[data-base-ui-focus-guard]")
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.getByRole("button", { exact: true, name: "Set up" }).click();
  await expect(
    page.getByText("Waiting for video tools…", { exact: true })
  ).toBeVisible();
});

test("a rejected waiting Download keeps consent open and does not start setup", async ({
  page,
}) => {
  await mockDesktop(page, { failFirstAdd: true, videoTools: initial });
  await page.goto("/");
  await submitVideo(page);
  await consent(page)
    .getByRole("button", { exact: true, name: "Download" })
    .click();
  await expect(
    page.getByText("Couldn't add download. Try again.", { exact: true })
  ).toBeVisible();
  await expect(consent(page)).toBeVisible();
  expect(
    await page.evaluate(() => window.testVideoTools.snapshot().state.kind)
  ).toBe("not_set_up");
  await consent(page)
    .getByRole("button", { exact: true, name: "Download" })
    .click();
  await expect(consent(page)).toHaveCount(0);
  await expect(
    page.getByRole("button", { exact: true, name: "Waiting video" })
  ).toHaveCount(1);
  expect(
    await page.evaluate(() => window.testVideoTools.snapshot().state.kind)
  ).toBe("setting_up");
});
