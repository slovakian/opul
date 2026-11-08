import type { Page } from "@playwright/test";

import type { Command, Snapshot } from "../../frontend/lib/model";
import type { DesktopOptions } from "./fixture-options";

declare global {
  interface Window {
    testWorkspaceRemoval: {
      /** Applies workspace deletion and saved-folder removal to the mock library. */
      execute: (command: Command, snapshot: Snapshot) => void;
    };
  }
}

export const mockWorkspaceRemoval = async (
  page: Page,
  options: DesktopOptions
) => {
  await page.addInitScript(
    ({ mockOpts }) => {
      window.testWorkspaceRemoval = {
        execute: (command, snapshot) => {
          if (command.kind === "delete_workspace") {
            if (mockOpts.workspaceDeletion === "rejected") {
              throw new Error("Couldn't delete workspace. Try again.");
            }

            if (mockOpts.workspaceDeletion !== "deferred") {
              snapshot.workspaces = snapshot.workspaces.filter(
                (workspace) => workspace.id !== command.workspace_id
              );
            }
          }

          if (command.kind !== "remove_folder") {
            return;
          }

          if (mockOpts.folderRemoval === "rejected") {
            throw new Error("Couldn't remove saved folder. Try again.");
          }

          const targetWorkspace = snapshot.workspaces.find(
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

            if (!targetWorkspace.folders.some((item) => item.is_default)) {
              targetWorkspace.folders[0].is_default = true;
            }
          }
        },
      };
    },
    { mockOpts: options }
  );
};
