import type { Download } from "@/lib/model";

/** States that belong in the In progress sidebar bucket. */
export const activeStates = new Set<Download["state"]>([
  "WaitingForVideoTools",
  "Queued",
  "Starting",
  "Fetching",
  "Processing",
  "Verifying",
  "Pausing",
]);

/** Downloads the In progress view counts, independent of pause capability. */
export const isActiveDownload = (download: Download): boolean =>
  activeStates.has(download.state);
