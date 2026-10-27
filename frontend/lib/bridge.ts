import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  preferencesSchema,
  receiptSchema,
  snapshotSchema,
  toolStatusSchema,
  workspaceDraftSchema,
  type Command,
  type Preferences,
} from "@/lib/model";

function requireDesktop() {
  if (!isTauri())
    throw new Error("Open the Opul desktop app to access your downloads.");
}
export const bridge = {
  async workspaceDraft() {
    return workspaceDraftSchema.parse(await invoke("workspace_draft"));
  },
  async snapshot() {
    requireDesktop();
    return snapshotSchema.parse(await invoke("snapshot"));
  },
  async execute(command: Command) {
    return receiptSchema.parse(await invoke("execute", { command }));
  },
  async preferences() {
    requireDesktop();
    return preferencesSchema.parse(await invoke("get_preferences"));
  },
  async savePreferences(preferences: Preferences) {
    await invoke("save_preferences", { preferences });
  },
  async toolStatus() {
    return toolStatusSchema.parse(await invoke("tool_status"));
  },
  async reveal(downloadId: string) {
    await invoke("reveal_download", { downloadId });
  },
  async chooseFolder() {
    return open({
      directory: true,
      multiple: false,
      title: "Choose a download folder",
    });
  },
  async chooseTorrent() {
    return open({
      multiple: false,
      title: "Open a torrent",
      filters: [{ name: "Torrent", extensions: ["torrent"] }],
    });
  },
};
export function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
