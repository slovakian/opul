import { expect, test } from "vite-plus/test";

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
