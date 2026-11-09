import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";

import type { Download, ListStyle, Snapshot } from "../../frontend/lib/model";
import { mockDesktop } from "./fixture";
import { baseDownload, preferences, snapshot } from "./fixture-data";
import type { DesktopInvokeArgs } from "./fixture-options";
import { replaceSnapshot } from "./workspace-sidebar-fixture";

declare global {
  interface Window {
    __TAURI_INTERNALS__: {
      invoke: (name: string, args?: DesktopInvokeArgs) => Promise<object>;
    };
    testHeldCommands: { count: number; release: () => void };
  }
}

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

/** Sends the next snapshot, as the 800ms poll would. */
const advance = async (page: Page, start: Snapshot, downloads: Download[]) => {
  const next = library(downloads);
  next.sequence = start.sequence + 1;
  next.revision = start.revision + 1;
  await replaceSnapshot(page, next);

  return next;
};

/** The tiers follow the list's width: ring, short bar, full bar. */
const tiers = { medium: 580, narrow: 460, wide: 760 } as const;

const pinListWidth = async (page: Page, width: number) => {
  await page.addStyleTag({
    content: `.download-list { width: ${width}px; max-width: none; }`,
  });
};

/** Rounded boxes, so sub-pixel layout noise doesn't count as movement. */
const box = (locator: Locator) =>
  locator.evaluate((element) => {
    const { height, width, x, y } = element.getBoundingClientRect();

    return {
      height: Math.round(height),
      width: Math.round(width),
      x: Math.round(x),
      y: Math.round(y),
    };
  });

const lefts = (locator: Locator) =>
  locator.evaluateAll((elements) =>
    elements.map((element) => Math.round(element.getBoundingClientRect().x))
  );

const rows = (page: Page) =>
  page.locator('[data-slot="item"][role="listitem"]');

const rowFor = (page: Page, download: Download) =>
  rows(page).filter({ hasText: download.name });

const activeLine = (item: Locator) =>
  item
    .locator(
      '[data-slot="item-description"] > [data-slot="download-row-layer"][data-active="true"]'
    )
    .first();

/** The visible line as read aloud, with each separator as " · ". */
const lineText = (line: Locator) =>
  line.evaluate((element) => {
    const copy = element.cloneNode(true);

    if (!(copy instanceof HTMLElement)) {
      return null;
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

const expectLine = async (line: Locator, expected: string) => {
  await expect.poll(() => lineText(line)).toBe(expected);
};

const statusSlot = (item: Locator) =>
  item.locator('[data-slot="download-status"]');

const bar = (item: Locator) => item.locator('[data-slot="progress"]');

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

for (const [tier, barWidth] of [
  ["narrow", null],
  ["medium", 112],
  ["wide", 192],
] as const) {
  test(`a ${tier} list shows the ${barWidth ? `${barWidth}px bar` : "ring"}`, async ({
    page,
  }) => {
    await mockDesktop(page, { snapshot: library([byId("fetching")]) });
    await page.goto("/");
    await pinListWidth(page, tiers[tier]);

    const item = rows(page).first();
    const ring = item.locator('[data-slot="circular-progress"]');

    if (barWidth === null) {
      await expect(ring).toBeVisible();
      await expect(bar(item)).toBeHidden();

      return;
    }

    await expect(ring).toBeHidden();
    await expect(bar(item)).toBeVisible();

    const { width } = await box(bar(item));
    expect(width).toBe(barWidth);
  });
}

test("empty speed, time left and seeding numbers aren't shown", async ({
  page,
}) => {
  const stalled = row("stalled", "Fetching", {
    download_rate: 0,
    eta_seconds: null,
  });

  const quiet = row("quiet", "Seeding", {
    capabilities: { ...idle, can_stop_seeding: true },
    peers: 0,
    source: "Torrent",
    upload_rate: 0,
  });

  await mockDesktop(page, { snapshot: library([stalled, quiet]) });
  await page.goto("/");
  await pinListWidth(page, tiers.wide);

  const line = activeLine(rowFor(page, stalled));
  await expectLine(line, "22.5 MB of 50.0 MB");
  await expect(line.locator('[data-segment="speed"]')).toHaveCount(0);
  await expect(line.locator('[data-segment="eta"]')).toHaveCount(0);
  await expect(rowFor(page, quiet)).toContainText("Seeding");
  await expect(rowFor(page, quiet)).not.toContainText("0 B/s");
  await expect(rowFor(page, quiet)).not.toContainText("peers");
});

test("changing live values never moves the row, its bar or its buttons", async ({
  page,
}) => {
  const fetching = byId("fetching");
  const start = library([fetching]);
  await mockDesktop(page, { snapshot: start });
  await page.goto("/");
  await pinListWidth(page, tiers.wide);

  const item = rows(page).first();
  const line = activeLine(item);
  await expectLine(line, "4.2 MB of 2.20 GB · 512 KB/s · 45s");

  const measure = async () => ({
    bar: await box(bar(item)),
    buttons: await lefts(
      item.getByRole("button", { name: /^(?:Pause|Actions)/u })
    ),
    row: await box(item),
  });

  const before = await measure();

  await advance(page, start, [
    {
      ...fetching,
      bytes_downloaded: 412 * MB,
      download_rate: 99.9 * MB,
      eta_seconds: 3 * 3600 + 12 * 60,
      progress_millionths: 600_000,
    },
  ]);
  await expectLine(line, "412 MB of 2.20 GB · 99.9 MB/s · 3h 12m");
  expect(await measure()).toEqual(before);
});

test("changing digits keeps a label's width while the character count holds", async ({
  page,
}) => {
  const seeding = byId("seeding");
  const start = library([seeding]);
  await mockDesktop(page, { snapshot: start });
  await page.goto("/");
  await pinListWidth(page, tiers.wide);

  const item = rows(page).first();
  const slot = statusSlot(item);
  await expect(slot).toContainText("↑ 1.2 MB/s");
  await expect(slot).toContainText("12 peers");

  await page.evaluate(async () => {
    await document.fonts.ready;
  });
  await expect(slot).toHaveAttribute("data-ready", "true");

  const before = await box(slot);

  await advance(page, start, [
    { ...seeding, peers: 47, upload_rate: 3.4 * MB },
  ]);
  await expect(slot).toContainText("↑ 3.4 MB/s");
  await expect(slot).toContainText("47 peers");
  expect(await box(slot)).toEqual(before);
});

test("resuming keeps the bar's value and says so", async ({ page }) => {
  const paused = row("resume", "Paused", {
    capabilities: { ...idle, can_resume: true },
    download_rate: 0,
    eta_seconds: null,
    progress_millionths: 450_000,
  });

  const start = library([paused]);
  await mockDesktop(page, { snapshot: start });
  await page.goto("/");
  await pinListWidth(page, tiers.wide);

  const item = rows(page).first();
  const progress = bar(item);
  const line = activeLine(item);
  await expectLine(line, "22.5 MB of 50.0 MB · Paused");
  await expect(progress).toHaveAttribute("aria-valuenow", "45");
  await expect(progress).toHaveAttribute("data-motion", "muted");

  const resuming = await advance(page, start, [
    { ...paused, capabilities: idle, state: "Starting" },
  ]);

  await expectLine(line, "22.5 MB of 50.0 MB · Resuming…");
  await expect(progress).toHaveAttribute("aria-valuenow", "45");
  await expect(progress).toHaveAttribute("data-motion", "pulse");

  const fill = progress.locator('[data-slot="progress-indicator"]');
  const { width } = await box(fill);
  expect(width).toBeGreaterThan(0);

  await advance(page, resuming, [
    {
      ...paused,
      capabilities: baseDownload.capabilities,
      download_rate: 2 * MB,
      eta_seconds: 14,
      progress_millionths: 460_000,
      state: "Fetching",
    },
  ]);
  await expect(progress).toHaveAttribute("aria-valuenow", "46");
  await expect(progress).toHaveAttribute("data-motion", "sheen");
  const { width: after } = await box(fill);
  expect(after).toBeGreaterThanOrEqual(width);
});

test("pause guards only its own button while Rust answers", async ({
  page,
}) => {
  const fetching = byId("fetching");
  const failed = byId("failed");
  await mockDesktop(page, { snapshot: library([fetching, failed]) });
  await page.goto("/");

  // Hold every command until the test lets Rust answer.
  await page.evaluate(() => {
    const internals = window.__TAURI_INTERNALS__;
    const { invoke } = internals;
    // oxlint-disable-next-line promise/avoid-new -- the test resolves this hold itself.
    const held = new Promise<void>((resolve) => {
      window.testHeldCommands = { count: 0, release: resolve };
    });

    internals.invoke = async (name, args) => {
      if (name === "execute") {
        window.testHeldCommands.count += 1;
        await held;
      }

      return invoke(name, args);
    };
  });

  const pause = page.getByRole("button", { name: `Pause ${fetching.name}` });
  const retry = page.getByRole("button", { name: `Retry ${failed.name}` });
  await pause.click();
  await pause.click();

  await expect(pause).toBeEnabled();
  await expect(retry).toBeEnabled();
  const sent = await page.evaluate(() => window.testHeldCommands.count);
  expect(sent).toBe(1);
  await page.evaluate(() => window.testHeldCommands.release());
});

test("a download that finishes while visible shows a check, then nothing", async ({
  page,
}) => {
  const fetching = byId("fetching");
  const finished = byId("finished");
  const start = library([fetching, finished]);
  await mockDesktop(page, { snapshot: start });
  await page.goto("/");

  const active = rowFor(page, fetching);
  const done = rowFor(page, finished);

  // Already finished at launch: no check, and the status slot is collapsed.
  await expect(done).toHaveAttribute("data-status", "empty");
  await expect(done.getByText("Download complete")).toBeHidden();
  await expect(active).toHaveAttribute("data-status", "progress");

  await advance(page, start, [
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

  await expect(active).toHaveAttribute("data-status", "check");
  await expect(active.getByText("Download complete")).toBeVisible();
  await expect(active).toHaveAttribute("data-status", "empty", {
    timeout: 5000,
  });
  await expect(active.getByText("Download complete")).toBeHidden();
  await expect(done).toHaveAttribute("data-status", "empty");
});
