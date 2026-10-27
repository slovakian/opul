import { z } from "zod";

export const formatSchema = z.enum([
  "Best",
  "BestMp4",
  "P1080",
  "P720",
  "Original",
  "AudioMp3",
  "AudioM4a",
]);
export const layoutSchema = z.enum(["List", "Table"]);
export const sortSchema = z.enum(["Order", "Name", "State", "Source"]);
export const groupingSchema = z.enum([
  "None",
  "State",
  "Folder",
  "Kind",
  "Source",
]);
export const folderSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
  is_default: z.boolean(),
});
export const torrentFileSchema = z.object({
  index: z.number(),
  path: z.string(),
  size: z.number(),
  priority: z.string(),
});
export const downloadSchema = z.object({
  id: z.string(),
  folder_id: z.string(),
  name: z.string(),
  source: z.enum(["Torrent", "VideoPage", "Direct", "Guided"]),
  source_url: z.string(),
  page_url: z.string().nullable(),
  format: formatSchema.nullable(),
  state: z.enum([
    "Queued",
    "Starting",
    "Fetching",
    "Processing",
    "Verifying",
    "Seeding",
    "Pausing",
    "Paused",
    "Finished",
    "Failed",
    "Cancelled",
  ]),
  hold_cause: z.enum(["Person", "Shutdown", "Recovery"]).nullable(),
  info_hash: z.string().nullable(),
  total_size: z.number(),
  bytes_downloaded: z.number(),
  bytes_uploaded: z.number(),
  seed_ratio: z.number(),
  seed_ratio_limit: z.number().nullable(),
  download_rate: z.number(),
  upload_rate: z.number(),
  eta_seconds: z.number().nullable(),
  peers: z.number(),
  seeds: z.number(),
  progress_millionths: z.number(),
  piece_length: z.number(),
  piece_count: z.number(),
  comment: z.string().nullable(),
  trackers: z.array(z.string()),
  output_path: z.string().nullable(),
  content_folder: z.string().nullable(),
  thumbnail_url: z.string().nullable(),
  extractor: z.string().nullable(),
  playlist_index: z.number().nullable(),
  error_message: z.string().nullable(),
  files: z.array(torrentFileSchema),
  capabilities: z.object({
    can_pause: z.boolean(),
    can_resume: z.boolean(),
    can_retry: z.boolean(),
    can_cancel: z.boolean(),
    can_stop_seeding: z.boolean(),
    can_change_file_selection: z.boolean(),
    can_remove: z.boolean(),
    can_reveal_output: z.boolean(),
    can_move: z.boolean(),
  }),
});
export const treatmentParametersSchema = z.object({
  hue: z.number().int().min(0).max(359),
  saturation: z.number().int().min(0).max(1000),
  brightness: z.number().int().min(50).max(150),
});
export const folderTreatmentSchema = treatmentParametersSchema.extend({
  preset_id: z.string().min(1).max(80),
  version: z.number().int().positive(),
});
export const appearancePreferenceSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("Automatic") }),
  z.object({ kind: z.literal("Original") }),
  z.object({ kind: z.literal("Preset"), treatment: folderTreatmentSchema }),
  treatmentParametersSchema.extend({ kind: z.literal("Custom") }),
]);
export const appearanceSchema = z.object({
  automatic_folder: folderTreatmentSchema.nullable(),
  preference: appearancePreferenceSchema.default({ kind: "Automatic" }),
});
export const workspaceDraftSchema = z.object({
  workspace_id: z.string(),
  folder_id: z.string(),
  appearance: appearanceSchema,
});
export type WorkspaceDraft = z.infer<typeof workspaceDraftSchema>;
export const workspaceSchema = z.object({
  id: z.string(),
  name: z.string(),
  icon: z.object({ kind: z.enum(["Symbol", "Emoji"]), value: z.string() }),
  appearance: appearanceSchema,
  folders: z.array(folderSchema),
  default_format: formatSchema,
  file_organization: z.enum([
    "Automatic",
    "AlwaysCreateFolder",
    "KeepInSaveLocation",
  ]),
  layout: layoutSchema,
  sort: sortSchema,
  grouping: groupingSchema,
  downloads: z.array(downloadSchema),
});
export const snapshotSchema = z.object({
  revision: z.number(),
  boot_id: z.string().nullable(),
  sequence: z.number(),
  session: z.enum([
    "Booting",
    "Reconciling",
    "Running",
    "Quiescing",
    "Stopped",
    "RecoveryRequired",
    "ReadOnly",
  ]),
  workspaces: z.array(workspaceSchema),
  recovery_notices: z.array(z.string()),
});
export const preferencesSchema = z.object({
  theme: z.enum(["system", "light", "dark"]),
  max_download_kbps: z.number(),
  max_upload_kbps: z.number(),
  concurrent_downloads: z.number(),
  listen_port: z.number(),
  randomize_port: z.boolean(),
  enable_dht: z.boolean(),
  max_connections: z.number(),
  yt_dlp_path: z.string().nullable(),
  ffmpeg_path: z.string().nullable(),
});
export const toolStatusSchema = z.object({
  ready: z.boolean(),
  message: z.string(),
  yt_dlp_path: z.string().nullable(),
  yt_dlp_version: z.string().nullable(),
  ffmpeg_path: z.string().nullable(),
  ffmpeg_version: z.string().nullable(),
});
export const receiptSchema = z.object({
  revision: z.number(),
  created_download_ids: z.array(z.string()),
});

export type Download = z.infer<typeof downloadSchema>;
export type Workspace = z.infer<typeof workspaceSchema>;
export type Folder = z.infer<typeof folderSchema>;
export type Snapshot = z.infer<typeof snapshotSchema>;
export type Preferences = z.infer<typeof preferencesSchema>;
export type Format = z.infer<typeof formatSchema>;
export type ToolStatus = z.infer<typeof toolStatusSchema>;
export type Receipt = z.infer<typeof receiptSchema>;
export type WorkspaceRequest = {
  request_id: string;
  expected_revision: number | null;
  workspace_id: string;
  name: string;
  icon: Workspace["icon"];
  appearance: Workspace["appearance"];
  folder_id: string;
  folder_name: string;
  folder_path: string;
  default_format: Format;
  file_organization: Workspace["file_organization"];
};
export type FolderRequest = {
  request_id: string;
  expected_revision: number | null;
  workspace_id: string;
  folder_id: string;
  name: string;
  path: string;
};
export type DownloadAction =
  "Pause" | "Resume" | "Retry" | "Cancel" | "StopSeeding";
export type WorkspaceAppearanceRequest = {
  request_id: string;
  workspace_id: string;
  emoji: string;
  appearance: Workspace["appearance"];
};
export type Command =
  | { kind: "set_workspace_appearance"; request: WorkspaceAppearanceRequest }
  | { kind: "create_workspace" | "update_workspace"; request: WorkspaceRequest }
  | { kind: "delete_workspace"; workspace_id: string }
  | {
      kind: "add_folder" | "default_folder" | "remove_folder";
      request: FolderRequest;
    }
  | {
      kind: "view";
      workspace_id: string;
      layout: Workspace["layout"];
      sort: Workspace["sort"];
      grouping: Workspace["grouping"];
    }
  | { kind: "format"; workspace_id: string; format: Format }
  | {
      kind: "add_url";
      workspace_id: string;
      folder_id: string;
      url: string;
      mode: "Direct" | "Torrent" | "Page";
      format: Format;
    }
  | {
      kind: "add_torrent_file";
      workspace_id: string;
      folder_id: string;
      path: string;
    }
  | { kind: "download"; download_id: string; action: DownloadAction }
  | { kind: "remove_downloads"; download_ids: string[] }
  | { kind: "torrent_files"; download_id: string; files: Download["files"] }
  | { kind: "seed_ratio"; download_id: string; limit: number | null }
  | { kind: "acknowledge_recovery" };

export const formatNames: Record<Format, string> = {
  Best: "Best available",
  BestMp4: "Best MP4",
  P1080: "1080p",
  P720: "720p",
  Original: "Original file",
  AudioMp3: "MP3 audio",
  AudioM4a: "M4A audio",
};
export const stateNames: Record<Download["state"], string> = {
  Queued: "Queued",
  Starting: "Starting",
  Fetching: "Downloading",
  Processing: "Processing",
  Verifying: "Verifying",
  Seeding: "Seeding",
  Pausing: "Pausing",
  Paused: "Paused",
  Finished: "Complete",
  Failed: "Failed",
  Cancelled: "Cancelled",
};
export const sourceNames: Record<Download["source"], string> = {
  Torrent: "BitTorrent",
  Direct: "Direct link",
  VideoPage: "Video",
  Guided: "Guided",
};

export function workspaceEmoji(workspace: Pick<Workspace, "icon">) {
  return workspace.icon.kind === "Emoji" && workspace.icon.value.trim()
    ? workspace.icon.value
    : "📁";
}
export function bytes(value: number) {
  if (value === 0) return "0 B";
  const unit = Math.min(Math.floor(Math.log(value) / Math.log(1024)), 4);
  return `${(value / 1024 ** unit).toLocaleString(undefined, { maximumFractionDigits: 1 })} ${["B", "KB", "MB", "GB", "TB"][unit]}`;
}
export function timeRemaining(seconds: number | null) {
  if (seconds === null) return "Calculating…";
  if (seconds < 60) return `${seconds}s left`;
  if (seconds < 3600) return `${Math.ceil(seconds / 60)}m left`;
  return `${Math.floor(seconds / 3600)}h ${Math.ceil((seconds % 3600) / 60)}m left`;
}
export function sourceHost(download: Download) {
  if (download.source === "Torrent") return "BitTorrent";
  try {
    return new URL(download.page_url || download.source_url).hostname.replace(
      /^www\./,
      "",
    );
  } catch {
    return sourceNames[download.source];
  }
}
