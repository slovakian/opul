import type { Page } from "@playwright/test";

import type { Command, Download, Snapshot } from "../../frontend/lib/model";

type ImportCommand = Extract<Command, { kind: "add_url" | "add_torrent_file" }>;

declare global {
  interface Window {
    testWaitingVideos: {
      project: (snapshot: Snapshot) => void;
      retry: (download: Download) => void;
      addition: (
        command: ImportCommand,
        base: Download
      ) => Pick<
        Download,
        | "folder_id"
        | "format"
        | "source_url"
        | "name"
        | "source"
        | "state"
        | "capabilities"
      >;
    };
  }
}

export const mockWaitingVideos = async (page: Page) => {
  await page.addInitScript(() => {
    window.testWaitingVideos = {
      addition: (command, base) => {
        const video = command.kind === "add_url" && command.mode === "Page";

        return {
          capabilities: { ...base.capabilities, can_pause: !video },
          folder_id: command.folder_id,
          format: command.kind === "add_url" ? command.format : null,
          name: video ? "Waiting video" : "new-file.zip",
          source: video ? "VideoPage" : base.source,
          source_url: command.kind === "add_url" ? command.url : command.path,
          state:
            video && window.testVideoTools.snapshot().state.kind !== "ready"
              ? "WaitingForVideoTools"
              : base.state,
        };
      },
      project: (snapshot) => {
        const { kind } = window.testVideoTools.snapshot().state;

        for (const download of snapshot.workspaces.flatMap(
          (workspace) => workspace.downloads
        )) {
          if (download.state !== "WaitingForVideoTools") {
            continue;
          }

          download.waiting_label =
            kind === "not_working"
              ? "Video tools aren't working"
              : "Waiting for video tools…";

          if (kind === "turned_off") {
            download.waiting_label = "Needs video tools";
          }

          download.capabilities.can_set_up_video_tools =
            kind === "turned_off" || kind === "not_set_up";
          download.capabilities.can_retry_video_tools = kind === "not_working";

          if (kind === "ready") {
            download.state = "Queued";
            download.waiting_label = null;
            download.capabilities.can_pause = true;
          }
        }
      },
      retry: (download) => {
        if (
          download.source === "VideoPage" &&
          window.testVideoTools.snapshot().state.kind !== "ready"
        ) {
          download.state = "WaitingForVideoTools";
          download.capabilities.can_pause = false;
          download.capabilities.can_retry = false;
        }
      },
    };
  });
};
