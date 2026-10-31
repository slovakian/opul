import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { mockDesktop, snapshot } from "./fixture";

const library = () => {
  const value = structuredClone(snapshot);
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
    icon: { kind: "Emoji", value: "👩🏽‍💻" },
    id: "media",
    name: "Media",
  });

  return value;
};

const input = (page: Page) =>
  page.getByRole("searchbox", { name: "Search downloads or paste a link" });

const button = (page: Page) =>
  page.getByRole("button", { exact: true, name: "Download link" });

const notice = (page: Page) =>
  page.locator('[data-slot="toast"]').filter({ hasText: "new-file.zip" });

const navigate = async (page: Page, name: string) => {
  await page
    .getByRole("navigation", { name: "Workspace navigation" })
    .getByRole("button", { exact: true, name })
    .click();
};

const chooseMedia = async (page: Page) => {
  await page
    .locator("main > header")
    .getByRole("button", { name: /^Save location:/u })
    .click();
  await page
    .getByRole("button", { exact: true, name: "Films (Default)" })
    .click();
};

const release = async (page: Page) => {
  await page.evaluate(() =>
    window.dispatchEvent(new Event("test:release-add"))
  );
};

const attempts = (page: Page) =>
  page.evaluate(() => sessionStorage.getItem("test-add-attempts"));

test("submission adds in place and the workspace toast opens the new inspector", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await mockDesktop(page, { persist: true, snapshot: library() });
  await page.goto("/");
  await navigate(page, "Completed");
  await chooseMedia(page);
  await input(page).fill("  https://example.com/new.zip  ");

  await input(page).press("Enter");

  await expect(input(page)).toHaveValue("");
  await expect(
    page.locator("main > header").getByText("Completed", { exact: true })
  ).toBeVisible();
  await expect(
    page.getByRole("complementary", { name: "Download details" })
  ).toHaveCount(0);
  await expect(notice(page)).toBeVisible();
  await expect(notice(page).locator('[data-slot="emoji-artwork"]')).toHaveText(
    "👩🏽‍💻"
  );
  await expect(notice(page)).toContainText("Media");
  await expect(notice(page)).toContainText("Films");
  await page.screenshot({
    path: "test-results/submission-toast.png",
  });

  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="toast-viewport"]')
    .analyze();

  expect(accessibility.violations).toEqual([]);
  await notice(page).getByRole("button", { exact: true, name: "View" }).focus();
  await page.keyboard.press("Enter");

  const inspector = page.getByRole("complementary", {
    name: "Download details",
  });

  await expect(
    inspector.getByRole("heading", { name: "new-file.zip" })
  ).toBeVisible();
  await expect(inspector).toContainText("Films");
  await expect(
    page.locator("main > header").getByText("Completed", { exact: true })
  ).toBeVisible();
  await inspector.getByRole("button", { name: "Close details" }).click();
  await expect(
    page.getByText("The creative process.mp4", { exact: true })
  ).toBeVisible();
  expect(await attempts(page)).toBe("1");
  expect(errors).toEqual([]);
});

test("pending submissions reject duplicates and preserve newer input", async ({
  page,
}) => {
  const newer = "next search";
  await mockDesktop(page, { holdAdd: true, persist: true });
  await page.goto("/");
  await input(page).fill("https://example.com/first.zip");
  // Submit twice in the same event turn, before React can disable the button.
  await page
    .getByRole("form", { name: "Search or add a download" })
    .evaluate((form) => {
      if (!(form instanceof HTMLFormElement)) {
        throw new TypeError("Expected the download form");
      }

      form.requestSubmit();
      form.requestSubmit();
    });
  await expect(button(page)).toBeDisabled();
  expect(await attempts(page)).toBe("1");
  await expect(button(page)).toHaveAttribute("aria-busy", "true");
  await expect(button(page).locator("svg.animate-spin")).toBeVisible();
  await input(page).press("Enter");
  await input(page).press("Enter");
  expect(await attempts(page)).toBe("1");
  await input(page).fill(newer);
  await page.screenshot({
    path: "test-results/submission-pending.png",
  });
  await release(page);
  await expect(notice(page)).toBeVisible();
  await expect(input(page)).toHaveValue(newer);
  await expect(button(page)).toHaveAttribute("aria-busy", "false");
  expect(await attempts(page)).toBe("1");

  const saved = await page.evaluate(() =>
    JSON.parse(sessionStorage.getItem("test-snapshot") ?? "null")
  );

  expect(saved.workspaces[0].downloads).toHaveLength(4);
  expect(saved.workspaces[0].downloads[3].source_url).toBe(
    "https://example.com/first.zip"
  );
});

test("failed additions retain the draft and cross-workspace destination for retry", async ({
  page,
}) => {
  await mockDesktop(page, {
    failFirstAdd: true,
    persist: true,
    snapshot: library(),
  });
  await page.goto("/");
  await navigate(page, "Downloads");
  await chooseMedia(page);
  await input(page).fill("https://example.com/retry.zip");
  await button(page).click();
  const error = page.locator('[data-slot="toast"][data-type="error"]');
  await expect(error).toContainText("Couldn't add download. Try again.");
  await expect(input(page)).toHaveValue("https://example.com/retry.zip");
  await expect(
    page.locator("main > header").getByRole("button", {
      exact: true,
      name: "Save location: Media › Films",
    })
  ).toBeEnabled();
  await expect(button(page)).toBeEnabled();
  await expect(notice(page)).toHaveCount(0);
  await input(page).press("Enter");
  await expect(notice(page)).toBeVisible();
  await expect(input(page)).toHaveValue("");
  expect(await attempts(page)).toBe("2");
  await notice(page).getByRole("button", { exact: true, name: "View" }).click();
  await expect(
    page.getByRole("complementary", { name: "Download details" })
  ).toContainText("Films");
  await expect(
    page
      .locator("main > header")
      .getByRole("heading", { exact: true, name: "Downloads" })
  ).toBeVisible();
});

test("navigation during flight keeps progress and leaves the new view's destination intact", async ({
  page,
}) => {
  await mockDesktop(page, { holdAdd: true, snapshot: library() });
  await page.goto("/");
  await chooseMedia(page);
  await input(page).fill("https://example.com/first.zip");
  await button(page).click();
  await expect(button(page)).toHaveAttribute("aria-busy", "true");
  await navigate(page, "Downloads");
  await expect(button(page).locator("svg.animate-spin")).toBeVisible();
  await input(page).fill("new search");
  await release(page);
  await expect(notice(page)).toBeVisible();
  await expect(input(page)).toHaveValue("new search");
  await expect(
    page
      .locator("main > header")
      .getByRole("button", { exact: true, name: "Save location: Downloads" })
  ).toBeEnabled();
  await notice(page).getByRole("button", { exact: true, name: "View" }).click();
  await expect(
    page
      .getByRole("complementary", { name: "Download details" })
      .getByRole("heading", { name: "new-file.zip" })
  ).toBeVisible();
  await expect(
    page
      .locator("main > header")
      .getByRole("heading", { exact: true, name: "Downloads" })
  ).toBeVisible();
  await expect(input(page)).toHaveValue("new search");
});

test("a failed pending request preserves newer typing and enables a corrected retry", async ({
  page,
}) => {
  await mockDesktop(page, {
    failFirstAdd: true,
    holdAdd: true,
    snapshot: library(),
  });
  await page.goto("/");
  await chooseMedia(page);
  await input(page).fill("https://example.com/first.zip");
  await input(page).press("Enter");
  await expect(button(page)).toBeDisabled();
  await input(page).fill("https://example.com/corrected.zip");
  await release(page);
  await expect(
    page.getByText("Couldn't add download. Try again.", { exact: true })
  ).toBeVisible();
  await expect(input(page)).toHaveValue("https://example.com/corrected.zip");
  await expect(button(page)).toBeEnabled();
  await expect(
    page.locator("main > header").getByRole("button", {
      exact: true,
      name: "Save location: Media › Films",
    })
  ).toBeEnabled();
  await input(page).press("Enter");
  await expect.poll(() => attempts(page)).toBe("2");
  await release(page);
  await expect(notice(page)).toBeVisible();
  await expect(input(page)).toHaveValue("");
});

test("aggregate confirmation submits once and its toast works at the minimum window width", async ({
  page,
}) => {
  await mockDesktop(page, { holdAdd: true, snapshot: library() });
  await page.setViewportSize({ height: 800, width: 720 });
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
    dialog.getByRole("button", { exact: true, name: "Adding…" })
  ).toBeDisabled();
  await expect(
    page.locator('[aria-label="Download link"] svg.animate-spin')
  ).toBeVisible();
  await page.keyboard.press("Enter");
  expect(await attempts(page)).toBe("1");
  await release(page);
  await expect(dialog).toHaveCount(0);
  await expect(input(page)).toHaveValue("");
  await expect(notice(page)).toBeVisible();
  await page.screenshot({ path: "test-results/submission-720-toast.png" });

  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="toast-viewport"]')
    .analyze();

  expect(accessibility.violations).toEqual([]);
  await notice(page).getByRole("button", { exact: true, name: "View" }).click();
  await expect(
    page
      .getByRole("dialog", { exact: true, name: "Download details" })
      .getByRole("heading", { name: "new-file.zip" })
  ).toBeVisible();
  await expect(
    page.locator("main > header").getByText("All downloads", { exact: true })
  ).toBeVisible();
  await page.screenshot({ path: "test-results/submission-720-inspector.png" });
});
