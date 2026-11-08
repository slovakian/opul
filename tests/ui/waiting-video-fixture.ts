import type { Page } from "@playwright/test";

import type { Command, Download, Snapshot } from "../../frontend/lib/model";
import type { DesktopInvokeArgs } from "./fixture-options";

type ImportCommand = Extract<Command, { kind: "add_url" | "add_torrent_file" }>;

declare global {
  interface Window {
    testWaitingVideos: {
      project: (snapshot: Snapshot) => void;
      invokeTools: (
        name: string,
        args: DesktopInvokeArgs,
        snapshot: Snapshot
      ) => ReturnType<Window["testVideoTools"]["invoke"]>;
      retry: (download: Download) => void;
      addition: (command: ImportCommand, base: Download) => Download;
    };
  }
}

export const mockWaitingVideos = async (page: Page) => {
  await page.addInitScript(() => {
    window.testWaitingVideos = {
      addition: (command, base) => {
        const video = command.kind === "add_url" && command.mode === "Page";

        return {
          ...base,
          bytes_downloaded: video ? 0 : base.bytes_downloaded,
          capabilities: { ...base.capabilities, can_pause: !video },
          download_rate: video ? 0 : base.download_rate,
          eta_seconds: video ? null : base.eta_seconds,
          folder_id: command.folder_id,
          format: command.kind === "add_url" ? command.format : null,
          name: video ? "Waiting video" : "new-file.zip",
          progress_millionths: video ? 0 : base.progress_millionths,
          relies_on_video_tools: video,
          source: video ? "VideoPage" : base.source,
          source_url: command.kind === "add_url" ? command.url : command.path,
          state:
            video && window.testVideoTools.snapshot().state.kind !== "ready"
              ? "WaitingForVideoTools"
              : base.state,
          total_size: video ? 0 : base.total_size,
        };
      },
      invokeTools: (name, args, snapshot) => {
        const result = window.testVideoTools.invoke(name, args);

        if (name === "video_tools_remove") {
          for (const download of snapshot.workspaces.flatMap(
            (workspace) => workspace.downloads
          )) {
            if (
              (download.needs_video_tools || download.source === "VideoPage") &&
              download.state !== "Finished"
            ) {
              download.state = "WaitingForVideoTools";
              download.relies_on_video_tools = true;
              download.capabilities.can_pause = false;
            }
          }

          window.testWaitingVideos.project(snapshot);
        }

        return result;
      },
      project: (snapshot) => {
        const video = window.testVideoTools.snapshot();
        const { kind } = video.state;

        for (const download of snapshot.workspaces.flatMap(
          (workspace) => workspace.downloads
        )) {
          download.capabilities.can_update_video_tools =
            !video.keep_up_to_date &&
            kind === "ready" &&
            video.state.kind === "ready" &&
            video.update_available &&
            download.state === "Failed" &&
            (download.needs_video_tools || download.source === "VideoPage");

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
