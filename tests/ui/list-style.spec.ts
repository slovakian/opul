import { expect, test } from "@playwright/test";

import { mockDesktop } from "./fixture";

const NO_SHADOW = "none";

test("download rows default to cards, select on click, and can switch to minimal", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");

  const list = page.locator('[data-slot="item-group"]');

  const row = page
    .getByRole("listitem")
    .filter({ hasText: "Design resources.zip" });

  const details = page.getByRole("complementary", { name: "Download details" });

  await expect(list).toHaveAttribute("data-list-style", "cards");
  await expect(row).not.toHaveCSS("box-shadow", NO_SHADOW);
  await expect(row).toHaveAttribute("data-selected", "false");

  // Row controls keep their own behavior instead of selecting the row.
  await row
    .getByRole("button", { exact: true, name: "Pause Design resources.zip" })
    .click();
  await expect(details).toHaveCount(0);
  await expect(row).toHaveAttribute("data-selected", "false");

  // Clicking anywhere else on the row selects it.
  await row.locator('[data-slot="item-description"]').click();
  await expect(details).toBeVisible();
  await expect(row).toHaveAttribute("data-selected", "true");
  await page.screenshot({ path: "test-results/list-style-cards.png" });
  await page.getByRole("button", { name: "Close details" }).click();

  await page.getByRole("button", { exact: true, name: "Settings" }).click();
  const settings = page.getByRole("dialog", { exact: true, name: "Settings" });
  await expect(
    settings.getByRole("button", { exact: true, name: "Cards" })
  ).toHaveAttribute("aria-pressed", "true");
  await settings.getByRole("button", { exact: true, name: "Minimal" }).click();
  await settings.getByRole("button", { name: "Save changes" }).click();
  await expect(settings).toHaveCount(0);

  await expect(list).toHaveAttribute("data-list-style", "minimal");
  await expect(row).toHaveCSS("box-shadow", NO_SHADOW);
  await page.screenshot({ path: "test-results/list-style-minimal.png" });
});
