import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { snapshotSchema } from "../../frontend/lib/model";
import { mockDesktop, snapshot } from "./fixture";

const editor = (page: Page) =>
  page.getByRole("dialog", { exact: true, name: "Change icon" });

const openPicker = async (page: Page, workspace = "Downloads") => {
  await page
    .getByRole("button", { exact: true, name: `${workspace} actions` })
    .click();
  await page.getByRole("button", { exact: true, name: "Change icon…" }).click();
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" })
  ).toBeFocused();
};

const savedSnapshot = async (page: Page) =>
  snapshotSchema.parse(
    await page.evaluate(async () => {
      const path = "/frontend/lib/bridge.ts";
      const { bridge } = await import(path);

      return bridge.snapshot();
    })
  );

const choose = async (page: Page, search: string, label: string) => {
  await page.getByRole("textbox", { name: "Search emoji" }).fill(search);
  await page.getByRole("gridcell", { exact: true, name: label }).click();
};

test("skin tones change the selected sequence and recents start with successful saves", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await expect(
    editor(page).getByRole("group", { exact: true, name: "Skin tone" })
  ).toHaveCount(0);
  await editor(page)
    .getByRole("textbox", { name: "Search emoji" })
    .fill("woman technologist");
  await editor(page)
    .getByRole("gridcell", { exact: true, name: "woman technologist" })
    .click();
  await editor(page)
    .getByRole("button", { exact: true, name: "Medium skin tone" })
    .click();
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 👩🏽‍💻" })
  ).toBeVisible();
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page)).toHaveCount(0);
  await openPicker(page);
  await expect(
    editor(page).locator('[data-slot="emoji-category-header"]:visible').first()
  ).toHaveText("Recently Used");
  await expect(editor(page).getByRole("gridcell").first()).toHaveText("👩🏽‍💻");
  await expect(
    editor(page).getByRole("button", { exact: true, name: "Medium skin tone" })
  ).toHaveAttribute("aria-pressed", "true");
});

test("only supported selections have tones and keyboard tone choices stay local through search and Custom", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);

  for (const [search, label] of [
    ["panda", "panda"],
    ["family", "family: man, woman, girl, boy"],
  ]) {
    // oxlint-disable-next-line eslint/no-await-in-loop
    await choose(page, search, label);
    // oxlint-disable-next-line eslint/no-await-in-loop
    await expect(
      editor(page).getByRole("group", { exact: true, name: "Skin tone" })
    ).toHaveCount(0);
  }

  await choose(page, "thumbs up", "thumbs up");

  const tones = editor(page).getByRole("group", {
    exact: true,
    name: "Skin tone",
  });

  await expect(tones.getByRole("button")).toHaveCount(6);
  await tones
    .getByRole("button", { exact: true, name: "Default skin tone" })
    .focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    tones.getByRole("button", { exact: true, name: "Light skin tone" })
  ).toBeFocused();
  await page.keyboard.press("Space");
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 👍🏻" })
  ).toBeVisible();
  await editor(page)
    .getByRole("button", { exact: true, name: "Custom" })
    .click();
  await editor(page)
    .getByRole("button", { exact: true, name: "Back to emoji browsing" })
    .click();
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" })
  ).toHaveValue("thumbs up");
  await expect(
    tones.getByRole("button", { exact: true, name: "Light skin tone" })
  ).toHaveAttribute("aria-pressed", "true");
  const savedBeforeCancel = await savedSnapshot(page);
  expect(savedBeforeCancel.recent_emoji).toEqual([]);
  await editor(page)
    .getByRole("button", { exact: true, name: "Cancel" })
    .click();
  const savedAfterCancel = await savedSnapshot(page);
  expect(savedAfterCancel.recent_emoji).toEqual([]);
});

test("editing a failed save retains the exact draft and retries without duplicate recents", async ({
  context,
  page,
}) => {
  await mockDesktop(page, {
    failFirstAppearance: true,
    loseFirstAppearanceResponse: false,
  });
  await page.goto("/");
  await openPicker(page);
  await choose(page, "woman technologist", "woman technologist");
  await editor(page)
    .getByRole("button", { exact: true, name: "Dark skin tone" })
    .click();
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page).getByRole("alert")).toContainText(
    "Couldn't save the icon"
  );
  const savedAfterFail = await savedSnapshot(page);
  expect(savedAfterFail.recent_emoji).toEqual([]);
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 👩🏿‍💻" })
  ).toBeVisible();
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page)).toHaveCount(0);
  const saved = await savedSnapshot(page);
  expect(saved.recent_emoji).toEqual(["👩🏿‍💻"]);
  const reopened = await context.newPage();
  await mockDesktop(reopened, { snapshot: saved });
  await reopened.goto("/");
  await openPicker(reopened);
  await expect(editor(reopened).getByRole("gridcell").first()).toHaveText("👩🏿‍💻");
  await editor(reopened)
    .getByRole("button", { exact: true, name: "Save" })
    .click();
  await expect(editor(reopened)).toHaveCount(0);
  const savedReopened = await savedSnapshot(reopened);
  expect(savedReopened.recent_emoji).toEqual(["👩🏿‍💻"]);
});

test("creation cancels drafts, retains nested choices, and records only the successful retry", async ({
  context,
  page,
}) => {
  await mockDesktop(page, { failFirstCreate: true });
  await page.goto("/");

  const dialog = page.getByRole("dialog", {
    name: "A place for your downloads",
  });

  const start = async () => {
    await page
      .getByRole("button", { exact: true, name: "New workspace" })
      .first()
      .click();
    await dialog
      .getByRole("textbox", { exact: true, name: "Name" })
      .fill("Hands");
    await dialog
      .getByRole("button", { exact: true, name: "Workspace emoji: 📁" })
      .click();
    await choose(page, "thumbs up", "thumbs up");
    await page
      .getByRole("button", { exact: true, name: "Medium-dark skin tone" })
      .click();
    await page.keyboard.press("Escape");
    await expect(
      dialog.getByRole("button", { exact: true, name: "Workspace emoji: 👍🏾" })
    ).toBeFocused();
    const savedInStart = await savedSnapshot(page);
    expect(savedInStart.recent_emoji).toEqual([]);
  };

  await start();
  await dialog.getByRole("button", { exact: true, name: "Cancel" }).click();
  const savedAfterFirstCancel = await savedSnapshot(page);
  expect(savedAfterFirstCancel.recent_emoji).toEqual([]);
  await start();
  await dialog
    .getByRole("button", { exact: true, name: "Create workspace" })
    .click();
  await expect(dialog.getByRole("alert")).toContainText(
    "Couldn't save the workspace"
  );
  const savedAfterFailCreate = await savedSnapshot(page);
  expect(savedAfterFailCreate.recent_emoji).toEqual([]);
  await dialog
    .getByRole("button", { exact: true, name: "Create workspace" })
    .click();
  await expect(dialog).toHaveCount(0);
  const saved = await savedSnapshot(page);
  expect(saved.recent_emoji).toEqual(["👍🏾"]);
  expect(saved.workspaces.at(-1)?.icon.value).toBe("👍🏾");
  const reopened = await context.newPage();
  await mockDesktop(reopened, { snapshot: saved });
  await reopened.goto("/");
  await openPicker(reopened, "Hands");
  await expect(editor(reopened).getByRole("gridcell").first()).toHaveText("👍🏾");
  await expect(
    editor(reopened).getByRole("button", {
      exact: true,
      name: "Medium-dark skin tone",
    })
  ).toHaveAttribute("aria-pressed", "true");
});

test("recent variants and joined sequences are searchable, keyboard selectable, and keep their exact Unicode", async ({
  page,
}) => {
  const initial = structuredClone(snapshot);
  initial.recent_emoji = ["👩🏽‍💻", "🫱🏻‍🫲🏿", "🫱🏿‍🫲🏻", "🏳️‍🌈", "👍🏿"];
  await mockDesktop(page, { snapshot: initial });
  await page.goto("/");
  await openPicker(page);
  const search = editor(page).getByRole("textbox", { name: "Search emoji" });
  await search.fill("handshake");
  await expect(editor(page).getByRole("gridcell").first()).toHaveText("🫱🏻‍🫲🏿");
  await page.keyboard.press("ArrowUp");
  await expect(
    editor(page).locator('[data-slot="emoji-cell"][data-active]')
  ).toHaveText("🫱🏻‍🫲🏿");
  await page.keyboard.press("Enter");
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 🫱🏻‍🫲🏿" })
  ).toBeVisible();
  await editor(page)
    .getByRole("gridcell", {
      exact: true,
      name: "handshake, Dark skin tone, Light skin tone, 🫱🏿‍🫲🏻",
    })
    .click();
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 🫱🏿‍🫲🏻" })
  ).toBeVisible();
  await search.fill("");
  await expect(editor(page).getByRole("gridcell").first()).toHaveText("👩🏽‍💻");
  await editor(page).getByRole("gridcell").first().click();
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 👩🏽‍💻" })
  ).toBeVisible();
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page)).toHaveCount(0);
  const savedAfterRecentsSave = await savedSnapshot(page);
  expect(savedAfterRecentsSave.recent_emoji).toEqual(initial.recent_emoji);
});

test("a full recent section is bounded and remains accessible at the minimum window", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const initial = structuredClone(snapshot);
  initial.recent_emoji = [
    "👩🏽‍💻",
    "👍🏿",
    "🫱🏻‍🫲🏿",
    "🏳️‍🌈",
    "👨‍👩‍👧‍👦",
    "🐼",
    "📁",
    "🏠",
    "💼",
    "🎬",
    "🎵",
    "📚",
    "🎮",
    "🎨",
    "📷",
    "💻",
    "🌱",
    "✨",
    "🌈",
    "🍎",
    "🐶",
    "🐱",
    "🐭",
    "🐹",
  ];
  initial.workspaces[0].icon = { kind: "Emoji", value: "👩🏽‍💻" };
  await mockDesktop(page, { failFirstAppearance: true, snapshot: initial });
  await page.goto("/");
  await openPicker(page);
  await page.setViewportSize({ height: 520, width: 720 });
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" })
  ).toBeInViewport();
  await expect(
    editor(page).getByRole("button", { exact: true, name: "Dark skin tone" })
  ).toBeInViewport();
  await choose(page, "rocket", "rocket");
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page).getByRole("alert")).toBeVisible();
  await expect(
    editor(page).getByRole("button", { exact: true, name: "Save" })
  ).toBeInViewport();

  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="popover-content"]')
    .disableRules(["button-name"])
    .analyze();

  expect(accessibility.violations).toEqual([]);
  await page.screenshot({
    path: "test-results/skin-tones-recents-minimum.png",
  });
  await editor(page).getByRole("button", { exact: true, name: "Save" }).click();
  await expect(editor(page)).toHaveCount(0);
  const savedFinal = await savedSnapshot(page);
  expect(savedFinal.recent_emoji).toEqual([
    "🚀",
    ...initial.recent_emoji.slice(0, 23),
  ]);
  expect(errors).toEqual([]);
});
