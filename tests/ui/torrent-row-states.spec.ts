import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

import type { Download } from "../../frontend/lib/model";
import {
  activeLine,
  advance,
  bar,
  byId,
  expectLine,
  library,
  MB,
  pinListWidth,
  rowFor,
  rows,
  statusSlot,
  tiers,
} from "./download-row-helpers";
import { mockDesktop } from "./fixture";

const menuItems = async (page: Page, download: Download) => {
  await page
    .getByRole("button", { name: `Actions for ${download.name}` })
    .click();
  const menu = page.getByRole("menu", { name: `Actions for ${download.name}` });
  await expect(menu).toBeVisible();

  return menu;
};

const sentCommand = (page: Page) =>
  page.evaluate(() => sessionStorage.getItem("test-command") ?? "");

test("a seeding torrent offers Finder and Stop seeding, never pause or cancel", async ({
  page,
}) => {
  const seeding = byId("seeding");
  await mockDesktop(page, { snapshot: library([seeding]) });
  await page.goto("/");
  await pinListWidth(page, tiers.wide);

  const item = rows(page).first();
  await expect(
    item.getByRole("button", { name: `Reveal ${seeding.name}` })
  ).toBeVisible();
  await expect(item.getByRole("button", { name: /^Pause/u })).toHaveCount(0);

  const menu = await menuItems(page, seeding);
  await expect(menu.getByRole("menuitem")).toHaveText([
    "Show details",
    "Show in Finder",
    "Stop seeding",
    "Remove from library",
  ]);
  await menu.getByRole("menuitem", { name: "Stop seeding" }).click();
  await expect.poll(() => sentCommand(page)).toContain('"StopSeeding"');
});

test("a torrent that isn't seeding collapses like a finished file and can start again", async ({
  page,
}) => {
  const stopped = byId("not-seeding");
  await mockDesktop(page, { snapshot: library([stopped]) });
  await page.goto("/");
  await pinListWidth(page, tiers.wide);

  // No bar comes back, and the slot collapses like a finished file.
  const item = rows(page).first();
  await expect(item).toHaveAttribute("data-status", "empty");
  await expect(bar(item)).toBeHidden();
  await expect(
    item.getByRole("button", { name: `Reveal ${stopped.name}` })
  ).toBeVisible();

  const menu = await menuItems(page, stopped);
  await expect(menu.getByRole("menuitem")).toHaveText([
    "Show details",
    "Show in Finder",
    "Start seeding",
    "Remove from library",
  ]);
  await menu.getByRole("menuitem", { name: "Start seeding" }).click();
  await expect.poll(() => sentCommand(page)).toContain('"StartSeeding"');
});

test("seeding again after checking files never replays the finish check", async ({
  page,
}) => {
  const checking = byId("checking");
  const start = library([checking]);
  await mockDesktop(page, { snapshot: start });
  await page.goto("/");

  const item = rows(page).first();
  await expect(statusSlot(item)).toContainText("Checking files…");

  await advance(page, start, [
    { ...checking, state: "Seeding", upload_rate: 0 },
  ]);
  await expect(statusSlot(item)).toContainText("Seeding");
  await expect(item).toHaveAttribute("data-status", "text");
  await expect(item.getByText("Download complete")).toBeHidden();
});

test("a magnet waiting for its file list says so in place of the bar", async ({
  page,
}) => {
  const waiting = byId("file-info");
  await mockDesktop(page, { snapshot: library([waiting]) });
  await page.goto("/");
  await pinListWidth(page, tiers.wide);

  const item = rows(page).first();
  await expect(item).toHaveAttribute("data-status", "text");
  await expect(statusSlot(item)).toContainText("Getting file info…");
  await expectLine(activeLine(item), "Torrent");
});

test("a stalled torrent keeps its bar and says it's looking for peers under its name", async ({
  page,
}) => {
  const stalled = byId("stalled");
  const start = library([stalled]);
  await mockDesktop(page, { snapshot: start });
  await page.goto("/");
  await pinListWidth(page, tiers.wide);

  const item = rows(page).first();
  // The bar keeps its normal look through the same grace period speed gets.
  await expect(bar(item)).toHaveAttribute("data-motion", "sheen");
  await expect(bar(item)).toHaveAttribute("data-motion", "searching", {
    timeout: 10_000,
  });
  await expect(item).toHaveAttribute("data-status", "progress");
  await expect(bar(item)).toHaveAttribute("aria-valuenow", "45");
  await expectLine(activeLine(item), "22.5 MB of 50.0 MB · Looking for peers…");

  await advance(page, start, [{ ...stalled, download_rate: MB, peers: 3 }]);
  await expect(bar(item)).toHaveAttribute("data-motion", "sheen");
  await expect(activeLine(item)).not.toContainText("Looking for peers");
});

test("below 440px status text moves under the name", async ({ page }) => {
  const queued = byId("queued");
  const seeding = byId("seeding");
  const waiting = byId("file-info");
  await mockDesktop(page, { snapshot: library([queued, seeding, waiting]) });
  await page.goto("/");
  await pinListWidth(page, 400);

  // A label leaves the slot entirely.
  const queuedRow = rowFor(page, queued);
  await expect(statusSlot(queuedRow)).toHaveCSS("width", "0px");
  await expectLine(activeLine(queuedRow), "example.com · 50.0 MB · Queued");

  // Seeding keeps only its speed, without the word or the source.
  const seedingRow = rowFor(page, seeding);
  await expect(statusSlot(seedingRow)).toHaveCSS("width", "0px");
  await expectLine(activeLine(seedingRow), "50.0 MB · ↑ 1.2 MB/s");

  // A hint that took the bar's place gives the slot back to the ring.
  const waitingRow = rowFor(page, waiting);
  await expect(
    waitingRow.locator('[data-slot="circular-progress"]')
  ).toBeVisible();
  await expectLine(activeLine(waitingRow), "Torrent · Getting file info…");
});

test("the table shows upload speed while seeding and no pause on a complete torrent", async ({
  page,
}) => {
  const seeding = byId("seeding");
  await mockDesktop(page, { snapshot: library([seeding]) });
  await page.goto("/");
  await page.getByRole("button", { exact: true, name: "Table view" }).click();

  const tableRow = page.locator("tbody tr").filter({ hasText: seeding.name });
  await expect(tableRow).toContainText("↑ 1.2 MB/s");
  await expect(tableRow.getByRole("button", { name: /^Pause/u })).toHaveCount(
    0
  );
  await expect(
    tableRow.getByRole("button", { name: `Reveal ${seeding.name}` })
  ).toBeVisible();
});
