import { writeFileSync } from "node:fs";

import { expect, test } from "@playwright/test";

import type { Download, Snapshot } from "../../frontend/lib/model";
import { mockDesktop } from "../ui/fixture";
import { baseDownload, snapshot as baseSnapshot } from "../ui/fixture-data";
import { installRenderProbe } from "./render-probe";
import type { CommitRecord } from "./render-probe";

/**
 * Measures how much the UI does to stay in sync with the engine.
 *
 *   vp run bench                       # 500 downloads
 *   BENCH_DOWNLOADS=2000 vp run bench  # a larger library
 *   BENCH_OUT=results.json vp run bench
 */
const downloadCount = Number(process.env.BENCH_DOWNLOADS ?? 500);
const idleSeconds = Number(process.env.BENCH_IDLE_SECONDS ?? 30);
const tickHz = 4;
const tickSeconds = 10;
const latencySamples = 20;
const activeId = "active-1";
const activeName = "Active transfer.zip";

const finished = (index: number, folderId: string): Download => ({
  ...baseDownload,
  bytes_downloaded: 52_428_800,
  capabilities: {
    ...baseDownload.capabilities,
    can_cancel: false,
    can_move: true,
    can_pause: false,
    can_reveal_output: true,
  },
  download_rate: 0,
  eta_seconds: null,
  folder_id: folderId,
  id: `finished-${index}`,
  name: `Archive ${index}.zip`,
  output_path: `/Users/test/Downloads/Archive ${index}.zip`,
  progress_millionths: 1_000_000,
  source_url: `https://example.com/archive-${index}.zip`,
  state: "Finished",
});

const largeLibrary = (): Snapshot => {
  const [first] = baseSnapshot.workspaces;
  const half = Math.floor(downloadCount / 2);

  const active: Download = {
    ...baseDownload,
    download_rate: 1_048_576,
    id: activeId,
    name: activeName,
  };

  return {
    ...baseSnapshot,
    workspaces: [
      {
        ...first,
        downloads: [
          active,
          ...Array.from({ length: half - 1 }, (_, index) =>
            finished(index, "folder-1")
          ),
        ],
      },
      {
        ...first,
        downloads: Array.from({ length: downloadCount - half }, (_, index) =>
          finished(half + index, "folder-2")
        ),
        folders: [
          {
            id: "folder-2",
            is_default: true,
            name: "Media",
            path: "/Users/test/Media",
          },
        ],
        id: "workspace-2",
        name: "Media",
      },
    ],
  };
};

const areas = {
  header: ["AppHeader"],
  rows: ["DownloadActions"],
  shell: ["App"],
  sidebar: ["WorkspaceSidebar"],
} as const;

const round = (value: number) => Math.round(value * 10) / 10;

const summarize = (commits: CommitRecord[], per: number) => {
  const total = (names?: readonly string[]) =>
    commits.reduce(
      (sum, commit) =>
        sum +
        Object.entries(commit.rendered)
          .filter(([name]) => !names || names.includes(name))
          .reduce((count, [, value]) => count + value, 0),
      0
    );

  return {
    commits: round(commits.length / per),
    componentRenders: round(total() / per),
    headerRenders: round(total(areas.header) / per),
    renderMs: round(
      commits.reduce((sum, commit) => sum + commit.durationMs, 0) / per
    ),
    rowRenders: round(total(areas.rows) / per),
    shellRenders: round(total(areas.shell) / per),
    sidebarRenders: round(total(areas.sidebar) / per),
  };
};

const percentile = (values: number[], fraction: number) => {
  const sorted = values.toSorted((a, b) => a - b);

  return sorted[
    Math.min(sorted.length - 1, Math.floor(sorted.length * fraction))
  ];
};

test("library sync cost", async ({ page }) => {
  test.setTimeout((idleSeconds + tickSeconds + latencySamples * 2 + 60) * 1000);
  await installRenderProbe(page);
  await mockDesktop(page, { snapshot: largeLibrary() });
  await page.goto("/");

  const activeRow = page.getByRole("listitem").filter({ hasText: activeName });
  await expect(activeRow).toBeVisible();
  await expect(page.locator("main [role='listitem']")).toHaveCount(
    downloadCount
  );
  // Let startup work (preferences, tools, fonts) settle before measuring.
  await page.waitForTimeout(2000);

  // 1. Idle: nothing changes in the engine.
  await page.evaluate(() => window.renderProbe.reset());
  await page.waitForTimeout(idleSeconds * 1000);
  const idleCommits = await page.evaluate(() => window.renderProbe.commits);
  const idlePerMinute = summarize(idleCommits, idleSeconds / 60);

  // 2. One transfer advancing at the engine's telemetry rate.
  await page.evaluate(() => window.renderProbe.reset());
  const ticks = tickHz * tickSeconds;

  for (let tick = 0; tick < ticks; tick += 1) {
    // oxlint-disable-next-line no-await-in-loop
    await page.evaluate(
      ({ id, value }) =>
        window.testUpdateDownload(id, {
          bytes_downloaded: value * 1000,
          download_rate: 1_048_576 + value,
          progress_millionths: 450_000 + value,
        }),
      { id: activeId, value: tick + 1 }
    );
    // oxlint-disable-next-line no-await-in-loop
    await page.waitForTimeout(1000 / tickHz);
  }

  // Allow the last update to land before counting.
  await page.waitForTimeout(1000);
  const tickCommits = await page.evaluate(() => window.renderProbe.commits);
  const perEngineUpdate = summarize(tickCommits, ticks);

  // 3. Engine change → DOM latency, sampled at random phases.
  const latencies: number[] = [];

  for (let sample = 0; sample < latencySamples; sample += 1) {
    // oxlint-disable-next-line no-await-in-loop
    await page.waitForTimeout(Math.random() * 1000);

    // oxlint-disable-next-line no-await-in-loop
    const latency = await page.evaluate(
      async ({ id, name, megabytes }) => {
        const row = [...document.querySelectorAll("[role='listitem']")].find(
          (item) => item.textContent?.includes(name)
        );

        if (!row) {
          throw new Error("Active row is missing");
        }

        const expected = `${megabytes} MB/s`;

        // oxlint-disable-next-line promise/avoid-new
        return await new Promise<number>((resolve) => {
          const started = performance.now();

          const observer = new MutationObserver(() => {
            if (row.textContent?.includes(expected)) {
              observer.disconnect();
              resolve(performance.now() - started);
            }
          });

          observer.observe(row, {
            characterData: true,
            childList: true,
            subtree: true,
          });
          window.testUpdateDownload(id, {
            download_rate: megabytes * 1_048_576,
          });
        });
      },
      { id: activeId, megabytes: 100 + sample, name: activeName }
    );

    latencies.push(latency);
  }

  const results = {
    downloads: downloadCount,
    idlePerMinute,
    latencyMs: {
      max: round(Math.max(...latencies)),
      mean: round(
        latencies.reduce((sum, value) => sum + value, 0) / latencies.length
      ),
      p50: round(percentile(latencies, 0.5)),
      p95: round(percentile(latencies, 0.95)),
    },
    perEngineUpdate,
  };

  // oxlint-disable-next-line no-console
  console.log(JSON.stringify(results, null, 2));

  if (process.env.BENCH_OUT) {
    writeFileSync(process.env.BENCH_OUT, JSON.stringify(results, null, 2));
  }
});
