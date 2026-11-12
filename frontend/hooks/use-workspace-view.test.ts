import { expect, test } from "vite-plus/test";

import { isActiveDownload } from "@/lib/download-activity";

import { baseDownload } from "../../tests/ui/fixture-data";
import { filterAndSortDownloads } from "./use-workspace-view";

test("waiting video downloads stay visible in In progress without a Pause capability", () => {
  const waiting = {
    ...baseDownload,
    capabilities: { ...baseDownload.capabilities, can_pause: false },
    state: "WaitingForVideoTools" as const,
  };

  expect(
    filterAndSortDownloads({
      downloads: [waiting],
      effectiveFilter: "active",
      query: "",
      sort: "Order",
    })
  ).toEqual([waiting]);
});

test("In progress membership follows the explicit active state set", () => {
  const queued = {
    ...baseDownload,
    capabilities: { ...baseDownload.capabilities, can_pause: false },
    id: "queued",
    state: "Queued" as const,
  };

  const paused = {
    ...baseDownload,
    capabilities: { ...baseDownload.capabilities, can_pause: false },
    id: "paused",
    state: "Paused" as const,
  };

  const finished = {
    ...baseDownload,
    capabilities: { ...baseDownload.capabilities, can_pause: false },
    id: "finished",
    state: "Finished" as const,
  };

  expect(isActiveDownload(queued)).toBe(true);
  expect(isActiveDownload(paused)).toBe(false);
  expect(isActiveDownload(finished)).toBe(false);

  expect(
    filterAndSortDownloads({
      downloads: [queued, paused, finished],
      effectiveFilter: "active",
      query: "",
      sort: "Order",
    })
  ).toEqual([queued]);
});
