import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { z } from "zod";

import { appUpdatesSchema } from "@/lib/app-updates";
import type { AppUpdates } from "@/lib/app-updates";
import {
  appearanceSchema,
  feedMessageSchema,
  preferencesSchema,
  receiptSchema,
  workspaceDraftSchema,
  workspaceExpansionSchema,
} from "@/lib/model";
import type {
  AggregateView,
  Command,
  DisplayPreferences,
  FeedMessage,
  FeedMessageInput,
  Preferences,
  WorkspaceExpansionChanges,
} from "@/lib/model";
import { truncateError } from "@/lib/truncate";
import {
  toolsSnapshotSchema,
  videoToolsSchema,
  updateNoticeSchema,
} from "@/lib/video-tools";
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
  async appUpdates() {
    return appUpdatesSchema.parse(await invoke("app_updates_snapshot"));
  },
  /** Starts a check; the result arrives through `onAppUpdates`. */
  async checkForAppUpdates() {
    return appUpdatesSchema.parse(await invoke("app_updates_check"));
  },
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
  async configureAppUpdates(choices: {
    checkAutomatically: boolean;
    installAutomatically: boolean;
  }) {
    return appUpdatesSchema.parse(
      await invoke("app_updates_configure", choices)
    );
  },
  async downloadAppUpdate() {
    return appUpdatesSchema.parse(await invoke("app_updates_download"));
  },
  async execute(command: Command) {
    return receiptSchema.parse(await invoke("execute", { command }));
  },
  async markVideoToolsSetupShown() {
    return z.boolean().parse(await invoke("mark_video_tools_setup_shown"));
  },
  async onAppUpdates(onSnapshot: (snapshot: AppUpdates) => void) {
    return await listen("app-updates", (event) => {
      const result = appUpdatesSchema.safeParse(event.payload);

      if (result.success) {
        onSnapshot(result.data);
      }
    });
  },
  /** The Help ▸ Set Up Video Tools… menu item. The shell emits each menu item's id. */
  async onSetUpVideoToolsMenu(handler: () => void) {
    return await listen("set-up-video-tools", handler);
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
  async restartForAppUpdate() {
    await invoke("app_updates_restart");
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
  /**
   * Streams the library: the whole library first, then each change as the
   * desktop publishes it. A message that fails validation ends the stream for
   * this subscriber through `onBroken`, since later changes build on it.
   */
  async subscribeLibrary(
    onMessage: (message: FeedMessage) => void,
    onBroken: (message: string) => void
  ) {
    requireDesktop();

    let broken = false;

    const feed = new Channel<FeedMessageInput>((message) => {
      if (broken) {
        return;
      }

      const result = feedMessageSchema.safeParse(message);

      if (result.success) {
        onMessage(result.data);
      } else {
        broken = true;
        onBroken(result.error.message);
      }
    });

    await invoke("subscribe_library", { feed });
  },
  /** Claims the one notification a version gets. Null when it was already shown. */
  async takeAppUpdateNotice() {
    return z
      .string()
      .nullable()
      .parse(await invoke("app_updates_take_notice"));
  },
  async takeVideoToolsUpdateNotice() {
    return updateNoticeSchema
      .nullable()
      .parse(await invoke("video_tools_take_update_notice"));
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
  async videoToolsRemove(turnOff: boolean) {
    return videoToolsSchema.parse(
      await invoke("video_tools_remove", { turnOff })
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

/** Short error text for toasts and other primary UI surfaces. */
export const shortError = (cause: unknown) =>
  truncateError(errorMessage(cause));
