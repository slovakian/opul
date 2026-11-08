import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";

import type { Download, ListStyle, Snapshot } from "../../frontend/lib/model";
import { mockDesktop } from "./fixture";
import { baseDownload, preferences, snapshot } from "./fixture-data";
import { replaceSnapshot } from "./workspace-sidebar-fixture";

const MB = 1024 * 1024;
const GB = 1024 * MB;

const idle = {
  ...baseDownload.capabilities,
  can_cancel: false,
  can_pause: false,
};

const row = (
  id: string,
  state: Download["state"],
  patch: Partial<Download> = {}
): Download => ({
  ...baseDownload,
  id,
  name: `${state} ${id}.zip`,
  state,
  ...patch,
});

/** One row for every state the list can show. */
const everyState: Download[] = [
  row("starting", "Starting", { download_rate: 0, eta_seconds: null }),
  row("fetching", "Fetching", {
    bytes_downloaded: 4.2 * MB,
    download_rate: 512 * 1024,
    eta_seconds: 45,
    total_size: 2.2 * GB,
  }),
  row("processing", "Processing", {
    capabilities: idle,
    download_rate: 0,
    source: "VideoPage",
  }),
  row("verifying", "Verifying", { capabilities: idle, download_rate: 0 }),
  row("paused", "Paused", {
    capabilities: { ...idle, can_resume: true },
    download_rate: 0,
  }),
  row("seeding", "Seeding", {
    capabilities: { ...idle, can_stop_seeding: true },
    download_rate: 0,
    peers: 12,
    source: "Torrent",
    upload_rate: 1.2 * MB,
  }),
  row("queued", "Queued", { download_rate: 0, eta_seconds: null }),
  row("waiting", "WaitingForVideoTools", {
    capabilities: { ...idle, can_set_up_video_tools: true },
    download_rate: 0,
    source: "VideoPage",
    waiting_label: "Waiting for video tools…",
  }),
  row("finished", "Finished", {
    bytes_downloaded: baseDownload.total_size,
    capabilities: { ...idle, can_reveal_output: true },
    download_rate: 0,
    progress_millionths: 1_000_000,
  }),
  row("failed", "Failed", {
    capabilities: { ...idle, can_retry: true },
    download_rate: 0,
    error_message:
      "The server closed the connection before the download finished, and a long explanation follows so the row has to truncate it",
  }),
  row("update", "Failed", {
    capabilities: { ...idle, can_retry: true, can_update_video_tools: true },
    download_rate: 0,
    error_message: "ERROR: original yt-dlp error text",
    source: "VideoPage",
  }),
  row("cancelled", "Cancelled", { capabilities: idle, download_rate: 0 }),
];

const byId = (id: string) => {
  const found = everyState.find((download) => download.id === id);

  if (!found) {
    throw new Error(`No fixture row named ${id}.`);
  }

  return found;
};

const library = (downloads: Download[]): Snapshot => {
  const value = structuredClone(snapshot);
  value.workspaces[0].downloads = downloads;

  return value;
};

/** The tiers follow the list's width, so pinning it tests each one directly. */
const tiers = { medium: 520, narrow: 380, wide: 760 } as const;

const pinListWidth = async (page: Page, width: number) => {
  await page.addStyleTag({
    content: `.download-list { width: ${width}px; max-width: none; }`,
  });
};

/** Rounded left edges, so sub-pixel layout noise doesn't count as movement. */
const lefts = (locator: Locator) =>
  locator.evaluateAll((elements) =>
    elements.map((element) => Math.round(element.getBoundingClientRect().x))
  );

const rows = (page: Page) =>
  page.locator('[data-slot="item"][role="listitem"]');

for (const listStyle of ["cards", "minimal"] satisfies ListStyle[]) {
  for (const [tier, width] of Object.entries(tiers)) {
    test(`${listStyle} rows share one height and one menu position when ${tier}`, async ({
      page,
    }) => {
      await mockDesktop(page, {
        preferences: { ...preferences, list_style: listStyle },
        snapshot: library(everyState),
      });
      await page.goto("/");
      await pinListWidth(page, width);
      await expect(rows(page)).toHaveCount(everyState.length);

      const heights = await rows(page).evaluateAll((elements) =>
        elements.map((element) =>
          Math.round(element.getBoundingClientRect().height)
        )
      );

      const menus = await lefts(
        rows(page).getByRole("button", { name: /^Actions for/u })
      );

      expect(new Set(heights).size).toBe(1);
      expect(menus).toHaveLength(everyState.length);
      expect(new Set(menus).size).toBe(1);
      await page.screenshot({
        fullPage: true,
        path: `test-results/download-rows-${listStyle}-${tier}.png`,
      });
    });
  }
}

test("changing live values never moves the row or its separators", async ({
  page,
}) => {
  const fetching = byId("fetching");
  const start = library([fetching]);
  await mockDesktop(page, { snapshot: start });
  await page.goto("/");
  await pinListWidth(page, tiers.wide);

  const item = rows(page).first();

  const live = item
    .locator('[data-slot="download-row-layer"][data-active="true"]')
    .first();

  const separators = live.locator('[data-slot="download-separator"]');
  await expect(live).toContainText("4.2 MB");
  await expect(separators).toHaveCount(2);

  const measure = async () => ({
    row: await item.boundingBox(),
    separators: await lefts(separators),
  });

  const before = await measure();

  const next = library([
    {
      ...fetching,
      bytes_downloaded: 412 * MB,
      download_rate: 99.9 * MB,
      eta_seconds: 3 * 3600 + 12 * 60,
      progress_millionths: 600_000,
    },
  ]);

  next.sequence = start.sequence + 1;
  next.revision = start.revision + 1;
  await replaceSnapshot(page, next);
  await expect(live).toContainText("412 MB");
  await expect(live).toContainText("3h 12m");

  const after = await measure();
  expect(after.separators).toEqual(before.separators);
  expect(after.row).toEqual(before.row);
});

test("a download that finishes while visible shows a check, then nothing", async ({
  page,
}) => {
  const fetching = byId("fetching");
  const finished = byId("finished");
  const start = library([fetching, finished]);
  await mockDesktop(page, { snapshot: start });
  await page.goto("/");

  const active = rows(page).filter({ hasText: fetching.name });
  const done = rows(page).filter({ hasText: finished.name });

  // Already finished at launch: no check, and the status slot is collapsed.
  await expect(done).toHaveAttribute("data-status", "empty");
  await expect(done.getByText("Download complete")).toBeHidden();
  await expect(active).toHaveAttribute("data-status", "progress");

  const next = library([
    {
      ...fetching,
      bytes_downloaded: fetching.total_size,
      capabilities: { ...idle, can_reveal_output: true },
      download_rate: 0,
      progress_millionths: 1_000_000,
      state: "Finished",
    },
    finished,
  ]);

  next.sequence = start.sequence + 1;
  next.revision = start.revision + 1;
  await replaceSnapshot(page, next);

  await expect(active).toHaveAttribute("data-status", "check");
  await expect(active.getByText("Download complete")).toBeVisible();
  await expect(active).toHaveAttribute("data-status", "empty", {
    timeout: 5000,
  });
  await expect(active.getByText("Download complete")).toBeHidden();
  await expect(done).toHaveAttribute("data-status", "empty");
});
