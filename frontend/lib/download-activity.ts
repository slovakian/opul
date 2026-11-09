import type { Download } from "@/lib/model";

/** Downloads the In progress view counts: transferring, pausing, or waiting for video tools. */
export const isActiveDownload = (download: Download): boolean =>
  download.capabilities.can_pause ||
  download.state === "Pausing" ||
  download.state === "WaitingForVideoTools";
