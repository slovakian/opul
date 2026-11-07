import type { Page } from "@playwright/test";

import type { Receipt } from "../../frontend/lib/model";
import folderTreatments from "../../src/application/folder-treatments.json" with { type: "json" };
import { baseDownload, preferences, snapshot } from "./fixture-data";
import type { DesktopInvokeArgs, DesktopOptions } from "./fixture-options";
import { mockFolderAddition } from "./folder-addition-fixture";
import { mockPreferenceResponses } from "./preference-response-fixture";
import { mockVideoTools } from "./video-tools-fixture";
import { mockWaitingVideos } from "./waiting-video-fixture";
import { mockWorkspaceExpansion } from "./workspace-expansion-fixture";

export { preferences, snapshot } from "./fixture-data";

export const mockDesktop = async (page: Page, options: DesktopOptions = {}) => {
  await mockVideoTools(page, options);
  await mockWaitingVideos(page);
  await mockFolderAddition(page, options);
  await mockWorkspaceExpansion(page, options);
  await mockPreferenceResponses(page, options);
  await page.addInitScript(
    ({
      defaultDownload,
      initPreferences,
      initSnapshot,
      mockOpts,
      treatments,
    }) => {
      const [, defaultTreatment, alternateTreatment] = treatments;

      const restore = <T>(key: string, initial: T): T => {
        const saved = mockOpts.persist ? sessionStorage.getItem(key) : null;

        return saved ? JSON.parse(saved) : structuredClone(initial);
      };

      const currentSnapshot = restore("test-snapshot", initSnapshot);

      let snapshotReadFailure = false;

      window.addEventListener("test:fail-snapshot", () => {
        snapshotReadFailure = true;
      });

      window.addEventListener("test:resume-snapshot", () => {
        snapshotReadFailure = false;
      });

      window.addEventListener("test:replace-snapshot", () => {
        const replacement = sessionStorage.getItem("test-snapshot");

        if (replacement) {
          Object.assign(currentSnapshot, JSON.parse(replacement));
        }
      });

      let currentPreferences = restore("test-preferences", initPreferences);

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
          invoke: async (name: string, args: DesktopInvokeArgs = {}) => {
            if (name === "snapshot") {
              window.testWaitingVideos.project(currentSnapshot);

              const held =
                await window.testFolderAddition.snapshot(currentSnapshot);

              if (held) {
                return held;
              }

              if (mockOpts.fail || snapshotReadFailure) {
                sessionStorage.setItem("test-snapshot-failed", "true");
                throw new Error("The state directory is already in use");
              }

              sessionStorage.setItem(
                "test-snapshot-sequence",
                String(currentSnapshot.sequence)
              );

              return structuredClone(currentSnapshot);
            }

            if (name === "mark_video_tools_setup_shown") {
              return window.testMarkVideoSetupShown(currentPreferences);
            }

            if (name === "get_preferences") {
              return await window.testPreferenceResponse(
                "read",
                currentPreferences
              );
            }

            if (name === "save_workspace_expansion" && args.choices) {
              const saved = await window.testSaveWorkspaceExpansion(
                args.choices,
                currentPreferences
              );

              return await window.testPreferenceResponse("expansion", saved);
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

              currentPreferences = window.testSettingsPreferences(
                args.preferences,
                currentPreferences
              );

              if (mockOpts.persist) {
                sessionStorage.setItem(
                  "test-preferences",
                  JSON.stringify(currentPreferences)
                );
              }

              return await window.testPreferenceResponse(
                "settings",
                currentPreferences
              );
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

              return await window.testPreferenceResponse(
                "display",
                currentPreferences
              );
            }

            if (
              name.startsWith("video_tools_") ||
              name === "tools_snapshot" ||
              name.startsWith("plugin:event|")
            ) {
              return window.testVideoTools.invoke(name, args);
            }

            if (name === "plugin:dialog|open") {
              return window.testFolderAddition.choose(args);
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

                  if (command.action === "Retry") {
                    window.testWaitingVideos.retry(download);
                  }
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
                const result = window.testFolderAddition.execute(
                  command.request,
                  currentSnapshot
                );

                if (result !== undefined) {
                  return result;
                }
              }

              if (command.kind === "remove_downloads") {
                for (const item of currentSnapshot.workspaces) {
                  item.downloads = item.downloads.filter(
                    (download) => !command.download_ids.includes(download.id)
                  );
                }
              }

              if (command.kind === "delete_workspace") {
                if (mockOpts.workspaceDeletion === "rejected") {
                  throw new Error("Couldn't delete workspace. Try again.");
                }

                if (mockOpts.workspaceDeletion !== "deferred") {
                  currentSnapshot.workspaces =
                    currentSnapshot.workspaces.filter(
                      (workspace) => workspace.id !== command.workspace_id
                    );
                }
              }

              if (command.kind === "remove_folder") {
                if (mockOpts.folderRemoval === "rejected") {
                  throw new Error("Couldn't remove saved folder. Try again.");
                }

                const targetWorkspace = currentSnapshot.workspaces.find(
                  (item) => item.id === command.request.workspace_id
                );

                if (!targetWorkspace || targetWorkspace.folders.length === 1) {
                  throw new Error("A workspace must keep a saved folder.");
                }

                if (mockOpts.folderRemoval !== "deferred") {
                  targetWorkspace.folders = targetWorkspace.folders.filter(
                    (item) => item.id !== command.request.folder_id
                  );
                  targetWorkspace.downloads = targetWorkspace.downloads.filter(
                    (item) => item.folder_id !== command.request.folder_id
                  );

                  if (
                    !targetWorkspace.folders.some((item) => item.is_default)
                  ) {
                    targetWorkspace.folders[0].is_default = true;
                  }
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
                    ...window.testWaitingVideos.addition(
                      command,
                      defaultDownload
                    ),
                    id: downloadId,
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

              window.testWaitingVideos.project(currentSnapshot);

              if (mockOpts.persist) {
                sessionStorage.setItem(
                  "test-snapshot",
                  JSON.stringify(currentSnapshot)
                );
              }

              if (command.kind === "add_folder") {
                await window.testFolderAddition.waitForReceipt();
              }

              return receipt;
            }

            return null;
          },
          metadata: {
            currentWebview: { label: "main" },
            currentWindow: { label: "main" },
          },
          transformCallback: window.testVideoTools.register,
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
