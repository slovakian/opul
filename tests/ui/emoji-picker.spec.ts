import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import { mockDesktop } from "./fixture";

const editor = (page: Page) =>
  page.getByRole("dialog", { exact: true, name: "Change icon" });

const openPicker = async (page: Page) => {
  await page
    .getByRole("button", { exact: true, name: "Downloads actions" })
    .click();
  await page.getByRole("button", { exact: true, name: "Change icon…" }).click();
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" })
  ).toBeFocused();
};

test("the complete emoji grid exposes positive one-based indices and passes every Axe rule", async ({
  page,
}) => {
  // Auditing the complete emoji dataset can exceed 30 seconds on CI WebKit.
  test.slow();
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  const grid = editor(page).getByRole("grid", { name: "Emoji categories" });
  await expect(grid.getByRole("row").first()).toHaveAttribute(
    "aria-rowindex",
    "1"
  );

  const indices = await grid.getByRole("row").evaluateAll((rows) =>
    rows.map((row) => ({
      columns: [...row.querySelectorAll('[role="gridcell"]')].map((cell) =>
        Number(cell.getAttribute("aria-colindex"))
      ),
      index: Number(row.getAttribute("aria-rowindex")),
    }))
  );

  expect(indices.length).toBeGreaterThan(100);
  expect(
    indices.every(
      (row, index) =>
        row.index === index + 1 &&
        row.columns.every((column, offset) => column === offset + 1)
    )
  ).toBe(true);

  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="popover-content"]')
    .analyze();

  expect(accessibility.violations).toEqual([]);
});

test("Tab enters one grid cell and arrows, row boundaries, and category jumps move real focus", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  const grid = editor(page).getByRole("grid");
  const active = grid.getByRole("gridcell", { selected: true });
  await page.keyboard.press("Tab");
  await expect(grid.getByRole("gridcell").first()).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(
    grid.getByRole("row").nth(1).getByRole("gridcell").first()
  ).toBeFocused();
  await page.keyboard.press("End");
  await expect(active).toHaveAttribute("aria-colindex", "8");
  await page.keyboard.press("ArrowRight");
  await expect(
    grid.getByRole("row").nth(2).getByRole("gridcell").first()
  ).toBeFocused();
  await page.keyboard.press("ArrowLeft");
  await expect(
    grid.getByRole("row").nth(1).getByRole("gridcell").last()
  ).toBeFocused();
  await page.keyboard.press("PageDown");
  await expect(
    grid
      .getByRole("rowgroup", { name: "People & Body" })
      .getByRole("gridcell")
      .first()
  ).toBeFocused();
  await expect(active).toBeInViewport({ ratio: 1 });
  await page.keyboard.press("PageUp");
  await expect(grid.getByRole("gridcell").first()).toBeFocused();
  await page.keyboard.press("Control+End");
  await expect(grid.getByRole("gridcell").last()).toBeFocused();
  await expect(active).toBeInViewport({ ratio: 1 });
  await page.keyboard.press("Control+Home");
  await expect(grid.getByRole("gridcell").first()).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(
    editor(page).getByRole("textbox", { name: "Search emoji" })
  ).toBeFocused();
});

test("empty search results announce their status and clearing search resets keyboard position and scroll", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await mockDesktop(page);
  await page.goto("/");
  await openPicker(page);
  const search = editor(page).getByRole("textbox", { name: "Search emoji" });
  await search.press("ArrowDown");
  await page.keyboard.press("PageDown");
  await search.fill("no-such-emoji-123456");
  await expect(
    editor(page).getByRole("status").filter({ hasText: "No emoji found." })
  ).toBeVisible();
  await search.press("ArrowDown");
  await expect(search).toBeFocused();
  await expect(editor(page).getByRole("gridcell")).toHaveCount(0);

  const accessibility = await new AxeBuilder({ page })
    .include('[data-slot="popover-content"]')
    .analyze();

  expect(accessibility.violations).toEqual([]);
  await search.fill("  PANDA  ");
  await expect(
    editor(page).getByRole("gridcell", { exact: true, name: "panda" })
  ).toBeVisible();
  await search.fill("");
  await expect(
    editor(page).getByRole("gridcell", { selected: true })
  ).toHaveCount(0);
  await expect
    .poll(() =>
      editor(page)
        .locator('[data-slot="emoji-viewport"]')
        .evaluate((node) => node.scrollTop)
    )
    .toBe(0);
  await search.press("ArrowUp");
  await expect(editor(page).getByRole("gridcell").first()).toBeFocused();
  expect(errors).toEqual([]);
});
