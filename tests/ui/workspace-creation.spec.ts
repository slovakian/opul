import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { snapshotSchema } from "../../frontend/lib/model";
import treatments from "../../src/application/folder-treatments.json" with { type: "json" };
import { mockDesktop } from "./fixture";

const artwork = '[data-slot="emoji-artwork"]';

const dialog = (page: Page) =>
  page.getByRole("dialog", { exact: true, name: "A place for your downloads" });

const picker = (page: Page) =>
  page.getByRole("dialog", { exact: true, name: "Choose workspace icon" });

const iconButton = (page: Page) =>
  dialog(page).getByRole("button", { name: /^Workspace emoji:/u });

const openCreation = async (page: Page) => {
  await page
    .getByRole("button", { exact: true, name: "New workspace" })
    .last()
    .click();
  await expect(
    dialog(page).getByRole("textbox", { exact: true, name: "Name" })
  ).toBeFocused();
};

const choose = async (page: Page, search: string, label: string) => {
  await picker(page)
    .getByRole("textbox", { name: "Search emoji" })
    .fill(search);
  await picker(page)
    .getByRole("gridcell", { exact: true, name: label })
    .click();
  await expect(picker(page)).toBeVisible();
};

const savedSnapshot = async (page: Page) =>
  snapshotSchema.parse(
    await page.evaluate(async () => {
      const path = "/frontend/lib/bridge.ts";
      const { bridge } = await import(path);

      return bridge.snapshot();
    })
  );

test("nested Escape retains icon choices and restores focus before canceling the whole draft", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openCreation(page);
  await iconButton(page).click();
  await expect(
    picker(page).getByRole("textbox", { name: "Search emoji" })
  ).toBeFocused();
  await choose(page, "family", "family: man, woman, girl, boy");
  await picker(page)
    .getByRole("button", { exact: true, name: "Original" })
    .click();
  await page.keyboard.press("Escape");
  await expect(picker(page)).toHaveCount(0);
  await expect(dialog(page)).toBeVisible();
  await expect(iconButton(page)).toBeFocused();
  await expect(iconButton(page).locator(artwork)).toHaveText("👨‍👩‍👧‍👦");
  await iconButton(page).click();
  await expect(
    picker(page).getByRole("button", { exact: true, name: "Original" })
  ).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  const savedAfterEsc = await savedSnapshot(page);
  expect(savedAfterEsc.workspaces).toHaveLength(1);
  await openCreation(page);
  await expect(iconButton(page).locator(artwork)).toHaveText("📁");
  await expect(iconButton(page).locator(artwork)).toHaveAttribute(
    "data-preset",
    "coral"
  );
});

test("creation without opening the picker saves the frozen folder preview", async ({
  page,
}) => {
  await mockDesktop(page, { changeSnapshotDuringDraft: true });
  await page.goto("/");
  await openCreation(page);
  await expect(iconButton(page).locator(artwork)).toHaveAttribute(
    "data-preset",
    "coral"
  );

  const filter = await iconButton(page)
    .locator(artwork)
    .evaluate((element) => getComputedStyle(element).filter);

  await dialog(page)
    .getByRole("textbox", { exact: true, name: "Name" })
    .fill("Quick workspace");
  await dialog(page)
    .getByRole("button", { exact: true, name: "Create workspace" })
    .click();
  await expect(dialog(page)).toHaveCount(0);
  await expect(
    page
      .getByRole("button", { exact: true, name: "Quick workspace workspace" })
      .locator(artwork)
  ).toHaveCSS("filter", filter);

  const savedState = await savedSnapshot(page);

  const created = savedState.workspaces.find(
    (workspace) => workspace.name === "Quick workspace"
  );

  expect(created?.appearance).toEqual({
    automatic_folder: treatments[1],
    preference: { kind: "Automatic" },
  });
});

test("creation saves emoji and appearance together after a lost response", async ({
  page,
}) => {
  await mockDesktop(page, {
    changeSnapshotDuringDraft: true,
    failFirstCreate: false,
    loseFirstCreateResponse: true,
  });
  await page.goto("/");
  await openCreation(page);
  await dialog(page)
    .getByRole("textbox", { exact: true, name: "Name" })
    .fill("Family");
  await iconButton(page).click();
  await choose(page, "family", "family: man, woman, girl, boy");
  await picker(page)
    .getByRole("button", { exact: true, name: "Original" })
    .click();
  await expect(
    picker(page).getByRole("img", { name: "Icon preview: 👨‍👩‍👧‍👦" })
  ).toBeVisible();
  await picker(page).getByRole("button", { exact: true, name: "Done" }).click();
  await expect(iconButton(page)).toBeFocused();

  const filter = await iconButton(page)
    .locator(artwork)
    .evaluate((element) => getComputedStyle(element).filter);

  const savedBeforeCreate = await savedSnapshot(page);
  expect(savedBeforeCreate.workspaces).toHaveLength(1);
  await dialog(page)
    .getByRole("button", { exact: true, name: "Create workspace" })
    .click();
  await expect(dialog(page).getByRole("alert")).toContainText(
    "Your changes are kept here"
  );
  await expect(iconButton(page).locator(artwork)).toHaveText("👨‍👩‍👧‍👦");
  await iconButton(page).click();
  await expect(
    picker(page).getByRole("button", { exact: true, name: "Original" })
  ).toHaveAttribute("aria-pressed", "true");
  await picker(page).getByRole("button", { exact: true, name: "Done" }).click();
  await dialog(page)
    .getByRole("button", { exact: true, name: "Create workspace" })
    .click();
  await expect(dialog(page)).toHaveCount(0);
  await expect(
    page
      .getByRole("button", { exact: true, name: "Family workspace" })
      .locator(artwork)
  ).toHaveCSS("filter", filter);
  await expect(
    page.getByRole("heading", { exact: true, name: "Family" }).locator(artwork)
  ).toHaveText("👨‍👩‍👧‍👦");

  const savedAfterRetry = await savedSnapshot(page);

  const created = savedAfterRetry.workspaces.filter(
    (workspace) => workspace.name === "Family"
  );

  expect(created).toHaveLength(1);
  expect(created[0].id).toBe("draft-workspace-1");
  expect(created[0].folders[0].id).toBe("draft-folder-1");
  expect(created[0].icon.value).toBe("👨‍👩‍👧‍👦");
  expect(created[0].appearance).toEqual({
    automatic_folder: treatments[1],
    preference: { kind: "Original" },
  });
});

test("outside close retains choices, Cancel discards them, and the next creation gets a new identity", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openCreation(page);
  await iconButton(page).click();
  await choose(page, "panda", "panda");
  await dialog(page)
    .getByRole("heading", { exact: true, name: "A place for your downloads" })
    .click({ position: { x: 5, y: 5 } });
  await expect(picker(page)).toHaveCount(0);
  await expect(iconButton(page).locator(artwork)).toHaveText("🐼");
  await dialog(page)
    .getByRole("button", { exact: true, name: "Cancel" })
    .click();
  const savedAfterCancel = await savedSnapshot(page);
  expect(savedAfterCancel.workspaces).toHaveLength(1);
  await openCreation(page);
  await expect(iconButton(page).locator(artwork)).toHaveText("📁");
  await dialog(page)
    .getByRole("textbox", { exact: true, name: "Name" })
    .fill("Fresh");
  await dialog(page)
    .getByRole("button", { exact: true, name: "Create workspace" })
    .click();
  await expect(dialog(page)).toHaveCount(0);
  const savedAfterFresh = await savedSnapshot(page);
  expect(
    savedAfterFresh.workspaces.find((workspace) => workspace.name === "Fresh")
      ?.id
  ).toBe("draft-workspace-2");
});

test("nested picker is keyboard accessible and contained in the minimum window", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.setViewportSize({ height: 520, width: 720 });
  await mockDesktop(page);
  await page.goto("/");
  await page
    .getByRole("button", { exact: true, name: "Toggle Sidebar" })
    .click();
  await openCreation(page);
  await iconButton(page).click();
  const search = picker(page).getByRole("textbox", { name: "Search emoji" });
  await expect(search).toBeFocused();
  await search.fill("panda");
  await expect(
    picker(page).getByRole("gridcell", { exact: true, name: "panda" })
  ).toBeVisible();
  await search.press("ArrowDown");
  await page.keyboard.press("Enter");
  await expect(
    picker(page).getByRole("img", { name: "Icon preview: 🐼" })
  ).toBeVisible();
  await expect(search).toBeInViewport();
  await expect(
    picker(page).getByRole("button", { exact: true, name: "Done" })
  ).toBeInViewport();
  const bounds = await picker(page).boundingBox();
  expect(bounds).not.toBeNull();

  if (bounds) {
    expect(bounds.x).toBeGreaterThanOrEqual(0);
    expect(bounds.y).toBeGreaterThanOrEqual(0);
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(720);
    expect(bounds.y + bounds.height).toBeLessThanOrEqual(520);
  }

  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="popover-content"]')
    .disableRules(["button-name"])
    .analyze();

  expect(accessibility.violations).toEqual([]);
  expect(errors).toEqual([]);
  await page.screenshot({
    path: "test-results/workspace-creation-minimum.png",
  });
  await page.keyboard.press("Escape");
  await expect(iconButton(page)).toBeFocused();
  await expect(
    dialog(page).getByRole("button", { exact: true, name: "Create workspace" })
  ).toBeInViewport();
});
