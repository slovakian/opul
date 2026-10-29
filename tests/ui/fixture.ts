import type { Page } from "@playwright/test";

import type {
  AggregateView,
  DisplayPreferences,
  Command,
  Preferences,
  Snapshot,
  Receipt,
} from "../../frontend/lib/model";
import folderTreatments from "../../src/application/folder-treatments.json" with { type: "json" };
import { baseDownload } from "./fixture-data";
import type { DesktopOptions } from "./fixture-options";

export const preferences: Preferences = {
  aggregate_destination: { kind: "prompt" },
  aggregate_views: {
    active: { grouping: "None", layout: "List", sort: "Order" },
    all: { grouping: "None", layout: "List", sort: "Order" },
    finished: { grouping: "None", layout: "List", sort: "Order" },
  },
  concurrent_downloads: 3,
  enable_dht: true,
  ffmpeg_path: null,
  listen_port: 6881,
  max_connections: 200,
  max_download_kbps: 0,
  max_upload_kbps: 0,
  randomize_port: true,
  theme: "light",
  yt_dlp_path: null,
};

export const snapshot: Snapshot = {
  boot_id: "boot-1",
  recent_emoji: [],
  recovery_notices: [],
  revision: 1,
  sequence: 1,
  session: "Running",
  workspaces: [
    {
      appearance: {
        automatic_folder: folderTreatments[0],
        preference: { kind: "Automatic" },
      },
      default_format: "Best",
      downloads: [
        baseDownload,
        {
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
          id: "download-2",
          name: "The creative process.mp4",
          output_path: "/Users/test/Downloads/Film.mp4",
          progress_millionths: 1_000_000,
          source: "VideoPage",
          state: "Finished",
        },
        {
          ...baseDownload,
          capabilities: {
            ...baseDownload.capabilities,
            can_pause: false,
            can_resume: true,
          },
          download_rate: 0,
          id: "download-3",
          name: "Field notes.pdf",
          state: "Paused",
        },
      ],
      file_organization: "Automatic",
      folders: [
        {
          id: "folder-1",
          is_default: true,
          name: "Downloads",
          path: "/Users/test/Downloads",
        },
      ],
      grouping: "None",
      icon: { kind: "Symbol", value: "arrow.down.circle" },
      id: "workspace-1",
      layout: "List",
      name: "Downloads",
      sort: "Order",
    },
  ],
};

export const mockDesktop = async (page: Page, options: DesktopOptions = {}) => {
  await page.addInitScript(
    ({
      defaultDownload,
      initPreferences,
      initSnapshot,
      mockOpts,
      treatments,
    }) => {
      const [, defaultTreatment, alternateTreatment] = treatments;

      const savedSnapshot = mockOpts.persist
        ? sessionStorage.getItem("test-snapshot")
        : null;

      const savedPreferences = mockOpts.persist
        ? sessionStorage.getItem("test-preferences")
        : null;

      const currentSnapshot: Snapshot = savedSnapshot
        ? JSON.parse(savedSnapshot)
        : structuredClone(initSnapshot);

      let currentPreferences: Preferences = savedPreferences
        ? JSON.parse(savedPreferences)
        : structuredClone(initPreferences);

      if (mockOpts.empty) {
        currentSnapshot.workspaces[0].downloads = [];
      }

      if (mockOpts.recovery) {
        currentSnapshot.session = "RecoveryRequired";
        currentSnapshot.recovery_notices = [
          "A previous checkpoint was recovered. Review your library before continuing.",
        ];
      }

      let viewAttempts = 0;
      let addAttempts = 0;
      let preferenceAttempts = 0;
      let callbackId = 0;
      let draftCalls = 0;
      let createAttempts = 0;
      let appearanceAttempts = 0;
      const receipts = new Map<string, Receipt>();

      Object.defineProperty(window, "isTauri", { value: true });
      Object.defineProperty(window, "__TAURI_EVENT_PLUGIN_INTERNALS__", {
        value: { unregisterListener: () => null },
      });
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: {
          // oxlint-disable-next-line eslint/complexity
          invoke: async (
            name: string,
            args: {
              command?: Command;
              preferences?: Preferences | DisplayPreferences;
              view?: AggregateView;
              workspaceId?: string;
            } = {}
          ) => {
            if (name === "snapshot") {
              if (mockOpts.fail) {
                throw new Error("The state directory is already in use");
              }

              return structuredClone(currentSnapshot);
            }

            if (name === "get_preferences") {
              if (mockOpts.failPreferencesRead) {
                throw new Error("Preferences are unavailable");
              }

              return currentPreferences;
            }

            if (name === "workspace_appearance_draft") {
              const targetWorkspace = currentSnapshot.workspaces.find(
                (item) => item.id === args.workspaceId
              );

              if (!targetWorkspace) {
                throw new Error("That workspace no longer exists.");
              }

              return {
                ...targetWorkspace.appearance,
                automatic_folder:
                  targetWorkspace.appearance.automatic_folder ??
                  defaultTreatment,
              };
            }

            if (name === "workspace_draft") {
              draftCalls += 1;

              const draft = {
                appearance: {
                  automatic_folder: defaultTreatment,
                  preference: { kind: "Automatic" },
                },
                folder_id: `draft-folder-${draftCalls}`,
                workspace_id: `draft-workspace-${draftCalls}`,
              };

              if (mockOpts.changeSnapshotDuringDraft) {
                const [firstWorkspace] = currentSnapshot.workspaces;

                firstWorkspace.appearance.automatic_folder = alternateTreatment;
                currentSnapshot.revision += 1;
                currentSnapshot.sequence += 1;
              }

              return draft;
            }

            if (
              name === "save_preferences" &&
              args.preferences &&
              "theme" in args.preferences
            ) {
              preferenceAttempts += 1;

              if (mockOpts.failFirstPreferences && preferenceAttempts === 1) {
                throw new Error("Couldn't write preferences. Try again.");
              }

              currentPreferences = {
                ...args.preferences,
                aggregate_views: currentPreferences.aggregate_views,
              };

              if (mockOpts.persist) {
                sessionStorage.setItem(
                  "test-preferences",
                  JSON.stringify(currentPreferences)
                );
              }

              return structuredClone(currentPreferences);
            }

            if (
              name === "save_aggregate_view" &&
              args.view &&
              args.preferences &&
              "layout" in args.preferences
            ) {
              viewAttempts += 1;

              if (mockOpts.failFirstView && viewAttempts === 1) {
                throw new Error(
                  "Couldn't write display preferences. Try again."
                );
              }

              currentPreferences.aggregate_views[args.view] = args.preferences;

              if (mockOpts.persist) {
                sessionStorage.setItem(
                  "test-preferences",
                  JSON.stringify(currentPreferences)
                );
              }

              return structuredClone(currentPreferences);
            }

            if (name === "tool_status") {
              return {
                ffmpeg_path: "/opt/homebrew/bin/ffmpeg",
                ffmpeg_version: "8.0",
                message: "yt-dlp and FFmpeg are available.",
                ready: true,
                yt_dlp_path: "/opt/homebrew/bin/yt-dlp",
                yt_dlp_version: "2026.10",
              };
            }

            if (name === "plugin:dialog|open") {
              sessionStorage.setItem("test-picker", JSON.stringify(args));

              if (JSON.stringify(args).includes("Open a torrent")) {
                return mockOpts.torrentPickerPath ?? null;
              }

              if (mockOpts.folderPickerError) {
                throw new Error(mockOpts.folderPickerError);
              }

              return mockOpts.folderPickerPath === undefined
                ? "/Users/test/Downloads"
                : mockOpts.folderPickerPath;
            }

            if (name === "execute") {
              const { command } = args;

              if (!command) {
                throw new Error("Missing command");
              }

              sessionStorage.setItem("test-command", JSON.stringify(command));

              if (command.kind === "set_workspace_appearance") {
                const cached = receipts.get(command.request.request_id);

                if (cached) {
                  return cached;
                }

                appearanceAttempts += 1;

                if (mockOpts.failFirstAppearance && appearanceAttempts === 1) {
                  throw new Error("Couldn't save the checkpoint. Try again.");
                }

                const targetWorkspace = currentSnapshot.workspaces.find(
                  (item) => item.id === command.request.workspace_id
                );

                if (!targetWorkspace) {
                  throw new Error("That workspace no longer exists.");
                }

                targetWorkspace.icon = {
                  kind: "Emoji",
                  value: command.request.emoji,
                };
                targetWorkspace.appearance = {
                  automatic_folder:
                    targetWorkspace.appearance.automatic_folder ??
                    (command.request.emoji.replaceAll(
                      /[\uFE0E\uFE0F]/gu,
                      ""
                    ) === "📁"
                      ? command.request.appearance.automatic_folder
                      : null),
                  preference: command.request.appearance.preference,
                };
              }

              if (command.kind === "format") {
                const targetWorkspace = currentSnapshot.workspaces.find(
                  (item) => item.id === command.workspace_id
                );

                if (targetWorkspace) {
                  targetWorkspace.default_format = command.format;
                }
              }

              if (command.kind === "view") {
                const targetWorkspace = currentSnapshot.workspaces.find(
                  (item) => item.id === command.workspace_id
                );

                if (targetWorkspace) {
                  Object.assign(targetWorkspace, {
                    grouping: command.grouping,
                    layout: command.layout,
                    sort: command.sort,
                  });
                }
              }

              if (command.kind === "download") {
                const download = currentSnapshot.workspaces
                  .flatMap((item) => item.downloads)
                  .find((item) => item.id === command.download_id);

                if (download) {
                  download.state =
                    command.action === "Pause" ? "Paused" : "Fetching";
                  download.capabilities.can_pause = command.action !== "Pause";
                  download.capabilities.can_resume = command.action === "Pause";
                }
              }

              if (command.kind === "create_workspace") {
                const receipt = receipts.get(command.request.request_id);

                if (receipt) {
                  return receipt;
                }

                if (
                  currentSnapshot.workspaces.some(
                    (item) => item.id === command.request.workspace_id
                  )
                ) {
                  throw new Error("That workspace already exists.");
                }

                createAttempts += 1;

                if (mockOpts.failFirstCreate && createAttempts === 1) {
                  throw new Error("Couldn't save the checkpoint. Try again.");
                }

                const { request } = command;
                const [firstWorkspace] = currentSnapshot.workspaces;

                currentSnapshot.workspaces.push({
                  ...firstWorkspace,
                  appearance: request.appearance,
                  downloads: [],
                  folders: [
                    {
                      id: request.folder_id,
                      is_default: true,
                      name: request.folder_name,
                      path: request.folder_path,
                    },
                  ],
                  icon: request.icon,
                  id: request.workspace_id,
                  name: request.name,
                });
              }

              let savedEmoji = null;

              if (command.kind === "set_workspace_appearance") {
                savedEmoji = command.request.emoji;
              } else if (
                command.kind === "create_workspace" &&
                command.request.icon.kind === "Emoji"
              ) {
                savedEmoji = command.request.icon.value;
              }

              if (savedEmoji) {
                currentSnapshot.recent_emoji = [
                  savedEmoji,
                  ...currentSnapshot.recent_emoji.filter(
                    (emoji) => emoji !== savedEmoji
                  ),
                ].slice(0, 24);
              }

              if (command.kind === "update_workspace") {
                const targetWorkspace = currentSnapshot.workspaces.find(
                  (item) => item.id === command.request.workspace_id
                );

                if (targetWorkspace) {
                  targetWorkspace.name = command.request.name;
                  targetWorkspace.icon = command.request.icon;
                }
              }

              if (command.kind === "add_folder") {
                const targetWorkspace = currentSnapshot.workspaces.find(
                  (item) => item.id === command.request.workspace_id
                );

                targetWorkspace?.folders.push({
                  id: command.request.folder_id,
                  is_default: false,
                  name: command.request.name,
                  path: command.request.path,
                });
              }

              if (command.kind === "remove_downloads") {
                for (const item of currentSnapshot.workspaces) {
                  item.downloads = item.downloads.filter(
                    (download) => !command.download_ids.includes(download.id)
                  );
                }
              }

              const createdDownloadIds: string[] = [];

              if (
                command.kind === "add_url" ||
                command.kind === "add_torrent_file"
              ) {
                addAttempts += 1;
                sessionStorage.setItem(
                  "test-add-attempts",
                  String(addAttempts)
                );

                if (mockOpts.holdAdd) {
                  // oxlint-disable-next-line promise/avoid-new
                  await new Promise<void>((resolve) => {
                    window.addEventListener(
                      "test:release-add",
                      () => resolve(),
                      {
                        once: true,
                      }
                    );
                  });
                }

                if (mockOpts.failFirstAdd && addAttempts === 1) {
                  throw new Error("Couldn't add download. Try again.");
                }

                if (
                  command.kind === "add_url" &&
                  !/^(?:https?:\/\/|magnet:)/u.test(command.url)
                ) {
                  throw new Error("Use an HTTP, HTTPS, or magnet link.");
                }

                const downloadId = `imported-${currentSnapshot.sequence}`;
                createdDownloadIds.push(downloadId);

                currentSnapshot.workspaces
                  .find((item) => item.id === command.workspace_id)
                  ?.downloads.push({
                    ...defaultDownload,
                    folder_id: command.folder_id,
                    format: command.kind === "add_url" ? command.format : null,
                    id: downloadId,

                    name: "new-file.zip",
                    source_url:
                      command.kind === "add_url" ? command.url : command.path,
                  });
              }

              if (command.kind === "acknowledge_recovery") {
                currentSnapshot.session = "Running";
                currentSnapshot.recovery_notices = [];
              }

              currentSnapshot.revision += 1;
              currentSnapshot.sequence += 1;

              const receipt = {
                created_download_ids: createdDownloadIds,
                revision: currentSnapshot.revision,
              };

              if (command.kind === "create_workspace") {
                receipts.set(command.request.request_id, receipt);

                if (mockOpts.loseFirstCreateResponse && createAttempts === 1) {
                  throw new Error(
                    "The response was lost after saving. Try again."
                  );
                }
              }

              if (command.kind === "set_workspace_appearance") {
                receipts.set(command.request.request_id, receipt);

                if (
                  mockOpts.loseFirstAppearanceResponse &&
                  appearanceAttempts === 1
                ) {
                  throw new Error(
                    "The response was lost after saving. Try again."
                  );
                }
              }

              if (mockOpts.persist) {
                sessionStorage.setItem(
                  "test-snapshot",
                  JSON.stringify(currentSnapshot)
                );
              }

              return receipt;
            }

            return null;
          },
          metadata: {
            currentWebview: { label: "main" },
            currentWindow: { label: "main" },
          },
          transformCallback: () => {
            callbackId += 1;

            return callbackId;
          },
          unregisterCallback: () => null,
        },
      });
    },
    {
      defaultDownload: baseDownload,
      initPreferences: options.preferences ?? preferences,
      initSnapshot: options.snapshot ?? snapshot,
      mockOpts: options,
      treatments: folderTreatments,
    }
  );
};
