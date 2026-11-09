import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";

import { mockDesktop } from "./fixture";

const folderIcon = (scope: Locator | Page) =>
  scope.locator('[data-slot="download-folder-icon"]');

const row = (page: Page, name: string) =>
  page.getByRole("listitem").filter({ hasText: name });

const isDecoded = (image: HTMLImageElement) =>
  image.complete && image.naturalWidth > 0;

test("list rows and details show a colored folder, while the table keeps file icons", async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto("/");

  // The fixture IDs land on a tinted color and on the artwork's own blue.
  const zip = folderIcon(row(page, "Design resources.zip"));
  await expect(zip).toHaveAttribute("data-color", "green");
  await expect(zip).toHaveCSS(
    "filter",
    "hue-rotate(282deg) saturate(1.05) brightness(0.97)"
  );
  await expect(zip).toHaveCSS("width", "32px");
  await expect(zip).toHaveAttribute("alt", "");
  expect(await zip.evaluate(isDecoded)).toBe(true);

  const pdf = folderIcon(row(page, "Field notes.pdf"));
  await expect(pdf).toHaveAttribute("data-color", "blue");
  await expect(pdf).toHaveCSS("filter", "none");

  await row(page, "Design resources.zip")
    .locator('[data-slot="item-description"]')
    .first()
    .click();
  const details = page.getByRole("complementary", { name: "Download details" });
  await expect(folderIcon(details)).toHaveAttribute("data-color", "green");
  await expect(folderIcon(details)).toHaveCSS("width", "48px");
  await page.screenshot({ path: "test-results/download-folder-icon.png" });
  await page.getByRole("button", { name: "Close details" }).click();

  await page.getByRole("button", { exact: true, name: "Table view" }).click();
  await expect(page.getByRole("table")).toBeVisible();
  await expect(folderIcon(page)).toHaveCount(0);
});
