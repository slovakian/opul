import { expect } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";

import type { Download, Snapshot } from "../../frontend/lib/model";
import { baseDownload, snapshot } from "./fixture-data";
import { replaceSnapshot } from "./workspace-sidebar-fixture";

export const MB = 1024 * 1024;
export const GB = 1024 * MB;

export const idle = {
  ...baseDownload.capabilities,
  can_cancel: false,
  can_pause: false,
};

export const row = (
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
export const everyState: Download[] = [
  row("starting", "Starting", {
    bytes_downloaded: 0,
    download_rate: 0,
    eta_seconds: null,
    progress_millionths: 0,
  }),
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
    capabilities: { ...idle, can_reveal_output: true, can_stop_seeding: true },
    download_rate: 0,
    peers: 12,
    source: "Torrent",
    upload_rate: 1.2 * MB,
  }),
  row("not-seeding", "NotSeeding", {
    capabilities: { ...idle, can_reveal_output: true, can_start_seeding: true },
    download_rate: 0,
    source: "Torrent",
  }),
  row("checking", "CheckingFiles", {
    capabilities: { ...idle, can_reveal_output: true, can_stop_seeding: true },
    download_rate: 0,
    source: "Torrent",
  }),
  row("file-info", "Fetching", {
    bytes_downloaded: 0,
    download_rate: 0,
    eta_seconds: null,
    progress_millionths: 0,
    source: "Torrent",
    total_size: 0,
  }),
  row("stalled", "Fetching", {
    download_rate: 0,
    eta_seconds: null,
    peers: 0,
    source: "Torrent",
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

export const byId = (id: string) => {
  const found = everyState.find((download) => download.id === id);

  if (!found) {
    throw new Error(`No fixture row named ${id}.`);
  }

  return found;
};

export const library = (downloads: Download[]): Snapshot => {
  const value = structuredClone(snapshot);
  value.workspaces[0].downloads = downloads;

  return value;
};

/** Sends the next snapshot, as the 800ms poll would. */
export const advance = async (
  page: Page,
  start: Snapshot,
  downloads: Download[]
) => {
  const next = library(downloads);
  next.sequence = start.sequence + 1;
  next.revision = start.revision + 1;
  await replaceSnapshot(page, next);

  return next;
};

/** The tiers follow the list's width: ring, short bar, full bar. */
export const tiers = { medium: 580, narrow: 460, wide: 760 } as const;

export const pinListWidth = async (page: Page, width: number) => {
  await page.addStyleTag({
    content: `.download-list { width: ${width}px; max-width: none; }`,
  });
};

/** Rounded boxes, so sub-pixel layout noise doesn't count as movement. */
export const box = (locator: Locator) =>
  locator.evaluate((element) => {
    const { height, width, x, y } = element.getBoundingClientRect();

    return {
      height: Math.round(height),
      width: Math.round(width),
      x: Math.round(x),
      y: Math.round(y),
    };
  });

export const lefts = (locator: Locator) =>
  locator.evaluateAll((elements) =>
    elements.map((element) => Math.round(element.getBoundingClientRect().x))
  );

export const rows = (page: Page) =>
  page.locator('[data-slot="item"][role="listitem"]');

export const rowFor = (page: Page, download: Download) =>
  rows(page).filter({ hasText: download.name });

export const activeLine = (item: Locator) =>
  item
    .locator(
      '[data-slot="item-description"] > [data-slot="download-row-layer"][data-active="true"]'
    )
    .first();

/** The visible line as read aloud, with each separator as " · ". */
export const lineText = (line: Locator) =>
  line.evaluate((element) => {
    const copy = element.cloneNode(true);

    if (!(copy instanceof HTMLElement)) {
      return null;
    }

    // Pieces for another width tier (display: none) aren't part of the line.
    const originals = [...element.querySelectorAll("*")];
    const copies = [...copy.querySelectorAll("*")];

    for (const [index, original] of originals.entries()) {
      if (getComputedStyle(original).display === "none") {
        copies[index]?.remove();
      }
    }

    for (const separator of copy.querySelectorAll(
      '[data-slot="download-separator"]'
    )) {
      separator.textContent = " · ";
    }

    for (const hidden of copy.querySelectorAll(".sr-only")) {
      hidden.remove();
    }

    return copy.textContent;
  });

export const expectLine = async (line: Locator, expected: string) => {
  await expect.poll(() => lineText(line)).toBe(expected);
};

export const statusSlot = (item: Locator) =>
  item.locator('[data-slot="download-status"]');

export const bar = (item: Locator) => item.locator('[data-slot="progress"]');
