import type { Page } from "@playwright/test";

import type {
  FolderRequest,
  Receipt,
  Snapshot,
} from "../../frontend/lib/model";
import type { DesktopInvokeArgs, DesktopOptions } from "./fixture-options";

interface FolderAdditionFixture {
  choose: (args: DesktopInvokeArgs) => string | null;
  execute: (
    request: FolderRequest,
    snapshot: Snapshot
  ) => Receipt | null | undefined;
  snapshot: (snapshot: Snapshot) => Promise<Snapshot | undefined>;
  waitForReceipt: () => Promise<void>;
}

declare global {
  interface Window {
    testFolderAddition: FolderAdditionFixture;
  }
}

export const mockFolderAddition = async (
  page: Page,
  options: DesktopOptions
) => {
  await page.addInitScript((mockOpts) => {
    let attempts = 0;
    let snapshotsUnavailable = false;
    let holdNextRefresh = false;
    let pending: FolderRequest | undefined;
    let library: Snapshot | undefined;

    window.addEventListener("test:confirm-folder", () => {
      if (!library) {
        return;
      }

      if (pending) {
        library.workspaces
          .find((workspace) => workspace.id === pending?.workspace_id)
          ?.folders.push({
            id: pending.folder_id,
            is_default: false,
            name: pending.name,
            path: pending.path,
          });
        pending = undefined;
        library.revision += 1;
      }

      library.sequence += 1;
      snapshotsUnavailable = false;
    });

    // The IPC adapter calls these only after all page initialization scripts run.
    window.testFolderAddition = {
      choose: (args) => {
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
      },
      execute: (request, currentSnapshot) => {
        library = currentSnapshot;
        attempts += 1;
        sessionStorage.setItem("test-folder-attempts", String(attempts));
        sessionStorage.setItem(
          "test-folder-command",
          JSON.stringify({ kind: "add_folder", request })
        );

        if (mockOpts.folderAddition === "rejected") {
          throw new Error("Couldn't save folder. Try again.");
        }

        if (mockOpts.folderAddition === "null") {
          return null;
        }

        if (
          mockOpts.folderAddition === "deferred" ||
          mockOpts.folderAddition === "refresh-failure"
        ) {
          pending = request;
          holdNextRefresh = mockOpts.holdFolderRefresh ?? false;
          snapshotsUnavailable = mockOpts.folderAddition === "refresh-failure";

          return { created_download_ids: [], revision: library.revision };
        }

        library.workspaces
          .find((workspace) => workspace.id === request.workspace_id)
          ?.folders.push({
            id: request.folder_id,
            is_default: false,
            name: request.name,
            path: request.path,
          });
      },
      snapshot: async (currentSnapshot) => {
        library = currentSnapshot;

        if (holdNextRefresh) {
          holdNextRefresh = false;
          const held = structuredClone(currentSnapshot);

          sessionStorage.setItem("test-held-folder-refresh", "true");
          // oxlint-disable-next-line promise/avoid-new
          await new Promise<void>((resolve) => {
            window.addEventListener(
              "test:release-folder-refresh",
              () => resolve(),
              { once: true }
            );
          });

          return held;
        }

        if (snapshotsUnavailable) {
          throw new Error("Library refresh unavailable. Try again.");
        }
      },
      waitForReceipt: async () => {
        if (mockOpts.holdFolderReceipt) {
          sessionStorage.setItem("test-held-folder-receipt", "true");
          // oxlint-disable-next-line promise/avoid-new
          await new Promise<void>((resolve) => {
            window.addEventListener(
              "test:release-folder-receipt",
              () => resolve(),
              { once: true }
            );
          });
        }
      },
    };
  }, options);
};
