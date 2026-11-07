import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { z } from "zod";

import {
  appearanceSchema,
  preferencesSchema,
  receiptSchema,
  snapshotSchema,
  workspaceDraftSchema,
  workspaceExpansionSchema,
} from "@/lib/model";
import type {
  AggregateView,
  Command,
  DisplayPreferences,
  Preferences,
  WorkspaceExpansionChanges,
} from "@/lib/model";
import { toolsSnapshotSchema, videoToolsSchema } from "@/lib/video-tools";
import type {
  VideoToolsOperation,
  VideoToolsSnapshot,
} from "@/lib/video-tools";

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
  async markVideoToolsSetupShown() {
    return z.boolean().parse(await invoke("mark_video_tools_setup_shown"));
  },
  async onVideoTools(onSnapshot: (snapshot: VideoToolsSnapshot) => void) {
    return await listen("video-tools", (event) => {
      const result = videoToolsSchema.safeParse(event.payload);

      if (result.success) {
        onSnapshot(result.data);
      }
    });
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
  async saveWorkspaceExpansion(choices: WorkspaceExpansionChanges) {
    return workspaceExpansionSchema.parse(
      await invoke("save_workspace_expansion", { choices })
    );
  },
  async snapshot() {
    requireDesktop();

    return snapshotSchema.parse(await invoke("snapshot"));
  },
  async toolsSnapshot() {
    return toolsSnapshotSchema.parse(await invoke("tools_snapshot"));
  },
  async videoToolsDetails() {
    return z.string().parse(await invoke("video_tools_details"));
  },
  async videoToolsExecute(operation: VideoToolsOperation) {
    return videoToolsSchema.parse(
      await invoke("video_tools_execute", { operation })
    );
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
