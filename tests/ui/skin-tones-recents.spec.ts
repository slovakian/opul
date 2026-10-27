import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

import { snapshotSchema } from "../../frontend/lib/model";
import { mockDesktop, snapshot } from "./fixture";

const editor = (page: Page) =>
  page.getByRole("dialog", { name: "Change icon", exact: true });

async function openPicker(page: Page, workspace = "Downloads") {
  await page
    .getByRole("button", { name: `${workspace} actions`, exact: true })
    .click();
  await page.getByRole("button", { name: "Change icon…", exact: true }).click();
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" })
  ).toBeFocused();
}

async function savedSnapshot(page: Page) {
  return snapshotSchema.parse(
    await page.evaluate(async () => {
      const path = "/frontend/lib/bridge.ts";
      const { bridge } = await import(path);
      return bridge.snapshot();
    })
  );
}

async function choose(page: Page, search: string, label: string) {
  await page.getByRole("textbox", { name: "Search emoji" }).fill(search);
  await page.getByRole("gridcell", { name: label, exact: true }).click();
}

test("skin tones change the selected sequence and recents start with successful saves", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  await expect(
    editor(page).getByRole("group", { name: "Skin tone", exact: true })
  ).toHaveCount(0);
  await editor(page)
    .getByRole("textbox", { name: "Search emoji" })
    .fill("woman technologist");
  await editor(page)
    .getByRole("gridcell", { name: "woman technologist", exact: true })
    .click();
  await editor(page)
    .getByRole("button", { name: "Medium skin tone", exact: true })
    .click();
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 👩🏽‍💻" })
  ).toBeVisible();
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  await openPicker(page);
  await expect(
    editor(page).locator("[frimousse-category-header]:visible").first()
  ).toHaveText("Recently Used");
  await expect(editor(page).getByRole("gridcell").first()).toHaveText("👩🏽‍💻");
  await expect(
    editor(page).getByRole("button", { name: "Medium skin tone", exact: true })
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
    await choose(page, search, label);
    await expect(
      editor(page).getByRole("group", { name: "Skin tone", exact: true })
    ).toHaveCount(0);
  }
  await choose(page, "thumbs up", "thumbs up");
  const tones = editor(page).getByRole("group", {
    name: "Skin tone",
    exact: true,
  });
  await expect(tones.getByRole("button")).toHaveCount(6);
  await tones
    .getByRole("button", { name: "Default skin tone", exact: true })
    .focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    tones.getByRole("button", { name: "Light skin tone", exact: true })
  ).toBeFocused();
  await page.keyboard.press("Space");
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 👍🏻" })
  ).toBeVisible();
  await editor(page)
    .getByRole("button", { name: "Custom", exact: true })
    .click();
  await editor(page)
    .getByRole("button", { name: "Back to emoji browsing", exact: true })
    .click();
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" })
  ).toHaveValue("thumbs up");
  await expect(
    tones.getByRole("button", { name: "Light skin tone", exact: true })
  ).toHaveAttribute("aria-pressed", "true");
  expect((await savedSnapshot(page)).recent_emoji).toEqual([]);
  await editor(page)
    .getByRole("button", { name: "Cancel", exact: true })
    .click();
  expect((await savedSnapshot(page)).recent_emoji).toEqual([]);
});

for (const lost of [false, true]) {
  test(`editing ${lost ? "a lost response" : "a failed save"} retains the exact draft and retries without duplicate recents`, async ({
    page,
    context,
  }) => {
    await mockDesktop(page, {
      failFirstAppearance: !lost,
      loseFirstAppearanceResponse: lost,
    });
    await page.goto("/");
    await openPicker(page);
    await choose(page, "woman technologist", "woman technologist");
    await editor(page)
      .getByRole("button", { name: "Dark skin tone", exact: true })
      .click();
    await editor(page)
      .getByRole("button", { name: "Save", exact: true })
      .click();
    await expect(editor(page).getByRole("alert")).toContainText(
      "Couldn't save the icon"
    );
    expect((await savedSnapshot(page)).recent_emoji).toEqual(
      lost ? ["👩🏿‍💻"] : []
    );
    await expect(
      editor(page).getByRole("img", { name: "Icon preview: 👩🏿‍💻" })
    ).toBeVisible();
    await editor(page)
      .getByRole("button", { name: "Save", exact: true })
      .click();
    await expect(editor(page)).toHaveCount(0);
    const saved = await savedSnapshot(page);
    expect(saved.recent_emoji).toEqual(["👩🏿‍💻"]);
    const reopened = await context.newPage();
    await mockDesktop(reopened, { snapshot: saved });
    await reopened.goto("/");
    await openPicker(reopened);
    await expect(editor(reopened).getByRole("gridcell").first()).toHaveText(
      "👩🏿‍💻"
    );
    await editor(reopened)
      .getByRole("button", { name: "Save", exact: true })
      .click();
    await expect(editor(reopened)).toHaveCount(0);
    expect((await savedSnapshot(reopened)).recent_emoji).toEqual(["👩🏿‍💻"]);
  });
}

test("creation cancels drafts, retains nested choices, and records only the successful retry", async ({
  page,
  context,
}) => {
  await mockDesktop(page, { failFirstCreate: true });
  await page.goto("/");
  const dialog = page.getByRole("dialog", {
    name: "A place for your downloads",
  });
  async function start() {
    await page
      .getByRole("button", { name: "New workspace", exact: true })
      .first()
      .click();
    await dialog
      .getByRole("textbox", { name: "Name", exact: true })
      .fill("Hands");
    await dialog
      .getByRole("button", { name: "Workspace emoji: 📁", exact: true })
      .click();
    await choose(page, "thumbs up", "thumbs up");
    await page
      .getByRole("button", { name: "Medium-dark skin tone", exact: true })
      .click();
    await page.keyboard.press("Escape");
    await expect(
      dialog.getByRole("button", { name: "Workspace emoji: 👍🏾", exact: true })
    ).toBeFocused();
    expect((await savedSnapshot(page)).recent_emoji).toEqual([]);
  }
  await start();
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  expect((await savedSnapshot(page)).recent_emoji).toEqual([]);
  await start();
  await dialog
    .getByRole("button", { name: "Create workspace", exact: true })
    .click();
  await expect(dialog.getByRole("alert")).toContainText(
    "Couldn't save the workspace"
  );
  expect((await savedSnapshot(page)).recent_emoji).toEqual([]);
  await dialog
    .getByRole("button", { name: "Create workspace", exact: true })
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
      name: "Medium-dark skin tone",
      exact: true,
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
    editor(page).locator("[frimousse-emoji][data-active]")
  ).toHaveText("🫱🏻‍🫲🏿");
  await page.keyboard.press("Enter");
  await expect(
    editor(page).getByRole("img", { name: "Icon preview: 🫱🏻‍🫲🏿" })
  ).toBeVisible();
  await editor(page)
    .getByRole("gridcell", {
      name: "handshake, Dark skin tone, Light skin tone, 🫱🏿‍🫲🏻",
      exact: true,
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
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  expect((await savedSnapshot(page)).recent_emoji).toEqual(
    initial.recent_emoji
  );
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
  await mockDesktop(page, { snapshot: initial, failFirstAppearance: true });
  await page.goto("/");
  await openPicker(page);
  await page.setViewportSize({ width: 720, height: 520 });
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" })
  ).toBeInViewport();
  await expect(
    editor(page).getByRole("button", { name: "Dark skin tone", exact: true })
  ).toBeInViewport();
  await choose(page, "rocket", "rocket");
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page).getByRole("alert")).toBeVisible();
  await expect(
    editor(page).getByRole("button", { name: "Save", exact: true })
  ).toBeInViewport();
  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="popover-content"]')
    .disableRules(["button-name"])
    .analyze();
  expect(accessibility.violations).toEqual([]);
  await page.screenshot({
    path: "test-results/skin-tones-recents-minimum.png",
  });
  await editor(page).getByRole("button", { name: "Save", exact: true }).click();
  await expect(editor(page)).toHaveCount(0);
  expect((await savedSnapshot(page)).recent_emoji).toEqual([
    "🚀",
    ...initial.recent_emoji.slice(0, 23),
  ]);
  expect(errors).toEqual([]);
});
