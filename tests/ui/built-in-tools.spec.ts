import { AxeBuilder } from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import { mockDesktop } from "./fixture";

test("Settings identifies FFmpeg and ffprobe as built in", async ({ page }) => {
  await mockDesktop(page);
  await page.goto("/");
  await page.getByRole("button", { exact: true, name: "Settings" }).click();
  const dialog = page.getByRole("dialog", { exact: true, name: "Settings" });
  await dialog.getByRole("button", { exact: true, name: "Tools" }).click();
  await expect(dialog.getByText("FFmpeg", { exact: true })).toBeVisible();
  await expect(
    dialog.getByText("9.0.2 · ffprobe 9.0.2", { exact: true })
  ).toBeVisible();
  await expect(dialog.getByText("Built in", { exact: true })).toBeVisible();
  await expect(dialog.getByLabel("FFmpeg path")).toHaveCount(0);
  await expect(dialog.getByText(/Restart Opul/u)).toHaveCount(0);
  await expect(dialog.getByLabel("yt-dlp path")).toHaveCount(0);

  const audit = await new AxeBuilder({ page })
    .include('[role="dialog"]')
    .analyze();

  expect(audit.violations).toEqual([]);
  await page.screenshot({ path: "test-results/built-in-tools.png" });
});
