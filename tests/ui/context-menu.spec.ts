import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Locator } from "@playwright/test";

import { mockDesktop } from "./fixture";

const menuItemLabels = async (menu: Locator) =>
  await menu.getByRole("menuitem").allTextContents();

test("right-clicking a list row opens the same menu as its … button", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");

  // CSS locators: an open menu hides the rest of the page from the a11y tree.
  const row = page
    .locator('[data-slot="item"]')
    .filter({ hasText: "Design resources.zip" });

  const menu = page.getByRole("menu", {
    name: "Actions for Design resources.zip",
  });

  const details = page.getByRole("complementary", { name: "Download details" });

  await row
    .getByRole("button", {
      exact: true,
      name: "Actions for Design resources.zip",
    })
    .click();
  await expect(menu.locator('[data-slot="sidebar"]')).toBeVisible();
  const buttonItems = await menuItemLabels(menu);
  await page.keyboard.press("Escape");
  await expect(menu).toHaveCount(0);

  await row.locator('[data-slot="item-description"]').click({
    button: "right",
  });
  await expect(menu.locator('[data-slot="sidebar"]')).toBeVisible();
  expect(await menuItemLabels(menu)).toEqual(buttonItems);

  // The row is marked as the menu's target without becoming selected.
  await expect(row).toHaveAttribute("data-popup-open", "");
  await expect(row).toHaveAttribute("data-selected", "false");
  await expect(details).toHaveCount(0);
  await page.screenshot({ path: "test-results/context-menu-list.png" });

  const result = await new AxeBuilder({ page })
    .include('[role="menu"]')
    .exclude("[data-base-ui-focus-guard]")
    .analyze();

  expect(result.violations).toEqual([]);

  await menu.getByRole("menuitem", { name: "Show details" }).click();
  await expect(menu).toHaveCount(0);
  await expect(details).toBeVisible();
  await expect(row).toHaveAttribute("data-selected", "true");
});

test("right-clicking a row's own controls still opens its menu", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");

  await page
    .getByRole("button", { exact: true, name: "Design resources.zip" })
    .click({ button: "right" });
  await expect(
    page.getByRole("menu", { name: "Actions for Design resources.zip" })
  ).toBeVisible();
  await expect(
    page.getByRole("complementary", { name: "Download details" })
  ).toHaveCount(0);
});

test("right-clicking a table row opens its menu", async ({ page }) => {
  await mockDesktop(page);
  await page.goto("/");
  await page.getByRole("button", { exact: true, name: "Table view" }).click();

  // CSS locator: an open menu hides the rest of the page from the a11y tree.
  const row = page.locator("tbody tr").filter({ hasText: "Field notes.pdf" });
  await row.getByRole("cell").nth(2).click({ button: "right" });

  const menu = page.getByRole("menu", { name: "Actions for Field notes.pdf" });
  await expect(menu.locator('[data-slot="sidebar"]')).toBeVisible();
  await expect(row).toHaveAttribute("data-popup-open", "");
  await page.screenshot({ path: "test-results/context-menu-table.png" });

  await menu.getByRole("menuitem", { name: "Remove from library" }).click();
  await expect(
    page.getByRole("button", { exact: true, name: "Remove download" })
  ).toBeVisible();
});

for (const key of ["Shift+F10", "ContextMenu"]) {
  test(`${key} on a focused row opens its … menu`, async ({ page }) => {
    await mockDesktop(page);
    await page.goto("/");

    await page
      .getByRole("button", { exact: true, name: "Design resources.zip" })
      .focus();
    await page.keyboard.press(key);

    const menu = page.getByRole("menu", {
      name: "Actions for Design resources.zip",
    });

    await expect(menu).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(menu).toHaveCount(0);
  });
}

test("closing details opened from the right-click menu returns focus to the row", async ({
  page,
}) => {
  // Narrow enough that the details panel is a modal sheet.
  await page.setViewportSize({ height: 800, width: 720 });
  await mockDesktop(page);
  await page.goto("/");

  const name = page.getByRole("button", {
    exact: true,
    name: "Design resources.zip",
  });

  await name.click({ button: "right" });
  await page.getByRole("menuitem", { name: "Show details" }).click();

  const details = page.getByRole("dialog", {
    exact: true,
    name: "Download details",
  });

  await expect(details).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(details).toHaveCount(0);
  await expect(name).toBeFocused();
});
