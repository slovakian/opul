import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import {
  appearanceSchema,
  preferencesSchema,
  receiptSchema,
  snapshotSchema,
  toolStatusSchema,
  workspaceDraftSchema,
  workspaceExpansionSchema,
} from "@/lib/model";
import type {
  AggregateView,
  Command,
  DisplayPreferences,
  Preferences,
  WorkspaceExpansion,
} from "@/lib/model";

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
  async saveAggregateView(
    view: AggregateView,
    preferences: DisplayPreferences
  ) {
    return preferencesSchema.parse(
      await invoke("save_aggregate_view", { preferences, view })
    );
  },
  async savePreferences(preferences: Preferences) {
    return preferencesSchema.parse(
      await invoke("save_preferences", { preferences })
    );
  },
  async saveWorkspaceExpansion(choices: WorkspaceExpansion) {
    return workspaceExpansionSchema.parse(
      await invoke("save_workspace_expansion", { choices })
    );
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

export const errorMessage = (cause: unknown) =>
  cause instanceof Error ? cause.message : String(cause);
