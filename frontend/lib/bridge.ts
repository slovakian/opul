import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import {
  appearanceSchema,
  preferencesSchema,
  receiptSchema,
  snapshotSchema,
  toolStatusSchema,
  workspaceDraftSchema,
} from "@/lib/model";
import type { Command, Preferences } from "@/lib/model";

const requireDesktop = () => {
  if (!isTauri()) {
    throw new Error("Open the Opul desktop app to access your downloads.");
  }
};

export const bridge = {
  chooseFolder() {
    return open({
      directory: true,
      multiple: false,
      title: "Choose a download folder",
    });
  },
  chooseTorrent() {
    return open({
      filters: [{ extensions: ["torrent"], name: "Torrent" }],
      multiple: false,
      title: "Open a torrent",
    });
  },
  async execute(command: Command) {
    return receiptSchema.parse(await invoke("execute", { command }));
  },
  async preferences() {
    requireDesktop();

    return preferencesSchema.parse(await invoke("get_preferences"));
  },
  async reveal(downloadId: string) {
    await invoke("reveal_download", { downloadId });
  },
  async savePreferences(preferences: Preferences) {
    await invoke("save_preferences", { preferences });
  },
  async snapshot() {
    requireDesktop();

    return snapshotSchema.parse(await invoke("snapshot"));
  },
  async toolStatus() {
    return toolStatusSchema.parse(await invoke("tool_status"));
  },
  async workspaceAppearanceDraft(workspaceId: string) {
    return appearanceSchema.parse(
      await invoke("workspace_appearance_draft", { workspaceId })
    );
  },
  async workspaceDraft() {
    return workspaceDraftSchema.parse(await invoke("workspace_draft"));
  },
};

export const errorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);
