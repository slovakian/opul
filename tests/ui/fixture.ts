import type { Page } from "@playwright/test";
import folderTreatments from "../../src/application/folder-treatments.json" with { type: "json" };
import type {
  Command,
  Download,
  Preferences,
  Snapshot,
  Receipt,
} from "../../frontend/lib/model";

export const baseDownload: Download = {
  id: "download-1",
  folder_id: "folder-1",
  name: "Design resources.zip",
  source: "Direct",
  source_url: "https://example.com/resources.zip",
  page_url: null,
  format: "Original",
  state: "Fetching",
  hold_cause: null,
  info_hash: null,
  total_size: 52_428_800,
  bytes_downloaded: 23_592_960,
  bytes_uploaded: 0,
  seed_ratio: 0,
  seed_ratio_limit: null,
  download_rate: 2_097_152,
  upload_rate: 0,
  eta_seconds: 14,
  peers: 0,
  seeds: 0,
  progress_millionths: 450_000,
  piece_length: 0,
  piece_count: 0,
  comment: null,
  trackers: [],
  output_path: null,
  content_folder: null,
  thumbnail_url: null,
  extractor: null,
  playlist_index: null,
  error_message: null,
  files: [],
  capabilities: {
    can_pause: true,
    can_resume: false,
    can_retry: false,
    can_cancel: true,
    can_stop_seeding: false,
    can_change_file_selection: false,
    can_remove: true,
    can_reveal_output: false,
    can_move: false,
  },
};
export const preferences: Preferences = {
  theme: "light",
  max_download_kbps: 0,
  max_upload_kbps: 0,
  concurrent_downloads: 3,
  listen_port: 6881,
  randomize_port: true,
  enable_dht: true,
  max_connections: 200,
  yt_dlp_path: null,
  ffmpeg_path: null,
};
export const snapshot: Snapshot = {
  revision: 1,
  boot_id: "boot-1",
  sequence: 1,
  session: "Running",
  recovery_notices: [],
  workspaces: [
    {
      id: "workspace-1",
      name: "Downloads",
      icon: { kind: "Symbol", value: "arrow.down.circle" },
      appearance: { automatic_folder: folderTreatments[0] },
      folders: [
        {
          id: "folder-1",
          name: "Downloads",
          path: "/Users/test/Downloads",
          is_default: true,
        },
      ],
      default_format: "Best",
      file_organization: "Automatic",
      layout: "List",
      sort: "Order",
      grouping: "None",
      downloads: [
        baseDownload,
        {
          ...baseDownload,
          id: "download-2",
          name: "The creative process.mp4",
          source: "VideoPage",
          state: "Finished",
          progress_millionths: 1_000_000,
          bytes_downloaded: 52_428_800,
          download_rate: 0,
          output_path: "/Users/test/Downloads/Film.mp4",
          capabilities: {
            ...baseDownload.capabilities,
            can_pause: false,
            can_cancel: false,
            can_reveal_output: true,
            can_move: true,
          },
        },
        {
          ...baseDownload,
          id: "download-3",
          name: "Field notes.pdf",
          state: "Paused",
          download_rate: 0,
          capabilities: {
            ...baseDownload.capabilities,
            can_pause: false,
            can_resume: true,
          },
        },
      ],
    },
  ],
};

export async function mockDesktop(
  page: Page,
  options: {
    empty?: boolean;
    fail?: boolean;
    recovery?: boolean;
    snapshot?: Snapshot;
    folderPickerPath?: string | null;
    folderPickerError?: string;
    failFirstCreate?: boolean;
    loseFirstCreateResponse?: boolean;
    changeSnapshotDuringDraft?: boolean;
  } = {},
) {
  await page.addInitScript(
    ({
      initialSnapshot,
      initialPreferences,
      options,
      baseDownload,
      folderTreatments,
    }) => {
      let snapshot = structuredClone(initialSnapshot);
      let preferences = structuredClone(initialPreferences);
      if (options.empty) snapshot.workspaces[0].downloads = [];
      if (options.recovery) {
        snapshot.session = "RecoveryRequired";
        snapshot.recovery_notices = [
          "A previous checkpoint was recovered. Review your library before continuing.",
        ];
      }
      let callbackId = 0;
      let draftCalls = 0;
      let createAttempts = 0;
      const receipts = new Map<string, Receipt>();
      Object.defineProperty(window, "isTauri", { value: true });
      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
        value: { unregisterListener: () => {} },
      });
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: {
          metadata: {
            currentWindow: { label: "main" },
            currentWebview: { label: "main" },
          },
          transformCallback: () => ++callbackId,
          unregisterCallback: () => {},
          invoke: async (
            name: string,
            args: { command?: Command; preferences?: Preferences } = {},
          ) => {
            if (name === "snapshot") {
              if (options.fail)
                throw new Error("The state directory is already in use");
              return structuredClone(snapshot);
            }
            if (name === "get_preferences") return preferences;
            if (name === "workspace_draft") {
              draftCalls++;
              const draft = {
                workspace_id: `draft-workspace-${draftCalls}`,
                folder_id: `draft-folder-${draftCalls}`,
                appearance: { automatic_folder: folderTreatments[1] },
              };
              if (options.changeSnapshotDuringDraft) {
                snapshot.workspaces[0].appearance.automatic_folder =
                  folderTreatments[2];
                snapshot.revision++;
                snapshot.sequence++;
              }
              return draft;
            }
            if (name === "save_preferences" && args.preferences) {
              preferences = args.preferences;
              return;
            }
            if (name === "tool_status")
              return {
                ready: true,
                message: "yt-dlp and FFmpeg are available.",
                yt_dlp_path: "/opt/homebrew/bin/yt-dlp",
                yt_dlp_version: "2026.10",
                ffmpeg_path: "/opt/homebrew/bin/ffmpeg",
                ffmpeg_version: "8.0",
              };
            if (name === "plugin:dialog|open") {
              if (options.folderPickerError)
                throw new Error(options.folderPickerError);
              return options.folderPickerPath === undefined
                ? "/Users/test/Downloads"
                : options.folderPickerPath;
            }
            if (name === "execute") {
              const command = args.command;
              if (!command) throw new Error("Missing command");
              if (command.kind === "view") {
                const workspace = snapshot.workspaces.find(
                  (workspace) => workspace.id === command.workspace_id,
                );
                if (workspace)
                  Object.assign(workspace, {
                    layout: command.layout,
                    sort: command.sort,
                    grouping: command.grouping,
                  });
              }
              if (command.kind === "download") {
                const download = snapshot.workspaces
                  .flatMap((workspace) => workspace.downloads)
                  .find((download) => download.id === command.download_id);
                if (download) {
                  download.state =
                    command.action === "Pause" ? "Paused" : "Fetching";
                  download.capabilities.can_pause = command.action !== "Pause";
                  download.capabilities.can_resume = command.action === "Pause";
                }
              }
              if (command.kind === "create_workspace") {
                const receipt = receipts.get(command.request.request_id);
                if (receipt) return receipt;
                if (
                  snapshot.workspaces.some(
                    (workspace) =>
                      workspace.id === command.request.workspace_id,
                  )
                )
                  throw new Error("That workspace already exists.");
                createAttempts++;
                if (options.failFirstCreate && createAttempts === 1)
                  throw new Error("Couldn't save the checkpoint. Try again.");
                const request = command.request;
                snapshot.workspaces.push({
                  ...snapshot.workspaces[0],
                  id: request.workspace_id,
                  name: request.name,
                  icon: request.icon,
                  appearance: request.appearance,
                  folders: [
                    {
                      id: request.folder_id,
                      name: request.folder_name,
                      path: request.folder_path,
                      is_default: true,
                    },
                  ],
                  downloads: [],
                });
              }
              if (command.kind === "update_workspace") {
                const workspace = snapshot.workspaces.find(
                  (workspace) => workspace.id === command.request.workspace_id,
                );
                if (workspace) {
                  workspace.name = command.request.name;
                  workspace.icon = command.request.icon;
                }
              }
              if (command.kind === "add_folder") {
                const workspace = snapshot.workspaces.find(
                  (workspace) => workspace.id === command.request.workspace_id,
                );
                workspace?.folders.push({
                  id: command.request.folder_id,
                  name: command.request.name,
                  path: command.request.path,
                  is_default: false,
                });
              }
              if (command.kind === "remove_downloads")
                for (const workspace of snapshot.workspaces)
                  workspace.downloads = workspace.downloads.filter(
                    (download) => !command.download_ids.includes(download.id),
                  );
              if (command.kind === "add_url") {
                if (!/^https?:\/\//.test(command.url))
                  throw new Error("Use an HTTP, HTTPS, or magnet link.");
                snapshot.workspaces
                  .find((workspace) => workspace.id === command.workspace_id)
                  ?.downloads.push({
                    ...baseDownload,
                    id: "imported",
                    name: "new-file.zip",
                    source_url: command.url,
                    folder_id: command.folder_id,
                  });
              }
              if (command.kind === "acknowledge_recovery") {
                snapshot.session = "Running";
                snapshot.recovery_notices = [];
              }
              snapshot.revision++;
              snapshot.sequence++;
              const receipt = {
                revision: snapshot.revision,
                created_download_ids: [],
              };
              if (command.kind === "create_workspace") {
                receipts.set(command.request.request_id, receipt);
                if (options.loseFirstCreateResponse && createAttempts === 1)
                  throw new Error(
                    "The response was lost after saving. Try again.",
                  );
              }
              return receipt;
            }
            return null;
          },
        },
      });
    },
    {
      initialSnapshot: options.snapshot ?? snapshot,
      initialPreferences: preferences,
      options,
      baseDownload,
      folderTreatments,
    },
  );
}
