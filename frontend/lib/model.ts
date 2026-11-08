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

export const displayPreferencesSchema = z.object({
  grouping: groupingSchema.default("None"),
  layout: layoutSchema.default("List"),
  sort: sortSchema.default("Order"),
});

export const aggregateViewSchema = z.enum(["all", "active", "finished"]);

export type AggregateView = z.infer<typeof aggregateViewSchema>;

export type DisplayPreferences = z.infer<typeof displayPreferencesSchema>;

export const defaultDisplay = displayPreferencesSchema.parse({});

export const aggregateNames: Record<AggregateView, string> = {
  active: "In progress",
  all: "All downloads",
  finished: "Completed",
};

export const aggregateViewsSchema = z.object({
  active: displayPreferencesSchema.prefault({}),
  all: displayPreferencesSchema.prefault({}),
  finished: displayPreferencesSchema.prefault({}),
});

export const folderSchema = z.object({
  id: z.string(),
  is_default: z.boolean(),
  name: z.string(),
  path: z.string(),
});

export const torrentFileSchema = z.object({
  index: z.number(),
  path: z.string(),
  priority: z.string(),
  size: z.number(),
});

export const downloadSchema = z.object({
  bytes_downloaded: z.number(),
  bytes_uploaded: z.number(),
  capabilities: z.object({
    can_cancel: z.boolean(),
    can_change_file_selection: z.boolean(),
    can_move: z.boolean(),
    can_pause: z.boolean(),
    can_remove: z.boolean(),
    can_resume: z.boolean(),
    can_retry: z.boolean(),
    can_retry_video_tools: z.boolean().default(false),
    can_reveal_output: z.boolean(),
    can_set_up_video_tools: z.boolean().default(false),
    can_stop_seeding: z.boolean(),
    can_update_video_tools: z.boolean().default(false),
  }),
  comment: z.string().nullable(),
  content_folder: z.string().nullable(),
  download_rate: z.number(),
  error_message: z.string().nullable(),
  eta_seconds: z.number().nullable(),
  extractor: z.string().nullable(),
  files: z.array(torrentFileSchema),
  folder_id: z.string(),
  format: formatSchema.nullable(),
  hold_cause: z.enum(["Person", "Shutdown", "Recovery"]).nullable(),
  id: z.string(),
  info_hash: z.string().nullable(),
  name: z.string(),
  needs_video_tools: z.boolean().default(false),
  output_path: z.string().nullable(),
  page_url: z.string().nullable(),
  peers: z.number(),
  piece_count: z.number(),
  piece_length: z.number(),
  playlist_index: z.number().nullable(),
  progress_millionths: z.number(),
  relies_on_video_tools: z.boolean().default(false),
  seed_ratio: z.number(),
  seed_ratio_limit: z.number().nullable(),
  seeds: z.number(),
  source: z.enum(["Torrent", "VideoPage", "Direct", "Guided"]),
  source_url: z.string(),
  state: z.enum([
    "WaitingForVideoTools",
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
  thumbnail_url: z.string().nullable(),
  total_size: z.number(),
  trackers: z.array(z.string()),
  upload_rate: z.number(),
  waiting_label: z.string().nullable().default(null),
});

export const treatmentParametersSchema = z.object({
  brightness: z.number().int().min(50).max(150),
  hue: z.number().int().min(0).max(359),
  saturation: z.number().int().min(0).max(1000),
});

export const folderTreatmentSchema = treatmentParametersSchema.extend({
  preset_id: z.string().min(1).max(80),
  version: z.number().int().positive(),
});

export type TreatmentParameters = z.infer<typeof treatmentParametersSchema>;

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
  appearance: appearanceSchema,
  folder_id: z.string(),
  workspace_id: z.string(),
});

export type WorkspaceDraft = z.infer<typeof workspaceDraftSchema>;

export const workspaceSchema = z.object({
  appearance: appearanceSchema,
  default_format: formatSchema,
  downloads: z.array(downloadSchema),
  file_organization: z.enum([
    "Automatic",
    "AlwaysCreateFolder",
    "KeepInSaveLocation",
  ]),
  folders: z.array(folderSchema),
  grouping: groupingSchema,
  icon: z.object({ kind: z.enum(["Symbol", "Emoji"]), value: z.string() }),
  id: z.string(),
  layout: layoutSchema,
  name: z.string(),
  sort: sortSchema,
});

export const snapshotSchema = z.object({
  boot_id: z.string().nullable(),
  recent_emoji: z.array(z.string()).max(24).default([]),
  recovery_notices: z.array(z.string()),
  revision: z.number(),
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
});

export const workspaceExpansionSchema = z.record(z.string(), z.boolean());

export type WorkspaceExpansion = z.infer<typeof workspaceExpansionSchema>;

export type WorkspaceExpansionChanges = Record<string, boolean | null>;

export const listStyleSchema = z.enum(["cards", "minimal"]);

export type ListStyle = z.infer<typeof listStyleSchema>;

export const preferencesSchema = z.object({
  aggregate_destination: z
    .discriminatedUnion("kind", [
      z.object({ kind: z.literal("prompt") }),
      z.object({ kind: z.literal("workspace"), workspace_id: z.string() }),
    ])
    .default({ kind: "prompt" }),
  aggregate_views: aggregateViewsSchema.prefault({}),
  concurrent_downloads: z.number(),
  enable_dht: z.boolean(),
  list_style: listStyleSchema.default("cards"),
  listen_port: z.number(),
  max_connections: z.number(),
  max_download_kbps: z.number(),
  max_upload_kbps: z.number(),
  randomize_port: z.boolean(),
  theme: z.enum(["system", "light", "dark"]),
  video_tools_setup_shown: z.boolean().default(false),
  workspace_expansion: workspaceExpansionSchema.default({}),
  workspace_expansion_startup: z
    .enum(["remember", "collapsed"])
    .default("remember"),
});

export const receiptSchema = z.object({
  created_download_ids: z.array(z.string()),
  revision: z.number(),
});

export type Download = z.infer<typeof downloadSchema>;

export type Workspace = z.infer<typeof workspaceSchema>;

export type Folder = z.infer<typeof folderSchema>;

export type Snapshot = z.infer<typeof snapshotSchema>;

export type Preferences = z.infer<typeof preferencesSchema>;

export type Format = z.infer<typeof formatSchema>;

export type Receipt = z.infer<typeof receiptSchema>;

export interface WorkspaceRequest {
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
}

export interface FolderRequest {
  request_id: string;
  expected_revision: number | null;
  workspace_id: string;
  folder_id: string;
  name: string;
  path: string;
}

export type DownloadAction =
  | "Pause"
  | "Resume"
  | "Retry"
  | "Cancel"
  | "StopSeeding";

export interface WorkspaceAppearanceRequest {
  request_id: string;
  workspace_id: string;
  emoji: string;
  appearance: Workspace["appearance"];
}

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
  AudioM4a: "M4A audio",
  AudioMp3: "MP3 audio",
  Best: "Best available",
  BestMp4: "Best MP4",
  Original: "Original file",
  P1080: "1080p",
  P720: "720p",
};

export const stateNames: Record<Download["state"], string> = {
  Cancelled: "Cancelled",
  Failed: "Failed",
  Fetching: "Downloading",
  Finished: "Complete",
  Paused: "Paused",
  Pausing: "Pausing",
  Processing: "Processing",
  Queued: "Queued",
  Seeding: "Seeding",
  Starting: "Starting",
  Verifying: "Verifying",
  WaitingForVideoTools: "Waiting for video tools…",
};

export const sourceNames: Record<Download["source"], string> = {
  Direct: "Direct link",
  Guided: "Guided",
  Torrent: "BitTorrent",
  VideoPage: "Video",
};

export const workspaceEmoji = (workspace: Pick<Workspace, "icon">) =>
  workspace.icon.kind === "Emoji" && workspace.icon.value.trim()
    ? workspace.icon.value
    : "📁";

export const bytes = (value: number) => {
  if (value === 0) {
    return "0 B";
  }

  const unit = Math.min(Math.floor(Math.log(value) / Math.log(1024)), 4);

  return `${(value / 1024 ** unit).toLocaleString(undefined, { maximumFractionDigits: 1 })} ${["B", "KB", "MB", "GB", "TB"][unit]}`;
};

const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;
const BYTES_PER_UNIT = 1024;

/** GB and TB keep two decimals below 10 so a large file still visibly moves. */
const FINE_UNIT = 3;

const liveDecimals = (scaled: number, unit: number) => {
  if (unit === 0 || scaled >= 99.95) {
    return 0;
  }

  if (unit >= FINE_UNIT && scaled < 9.995) {
    return 2;
  }

  return 1;
};

/**
 * Byte counts for values that change while you watch: at most three digits
 * and a steady number of decimals per range (`4.2 MB`, `42.0 MB`, `412 MB`,
 * `1.20 GB`), so the text keeps nearly the same width from tick to tick.
 * `bytes()` stays the precise formatter for the details panel and the table.
 */
export const liveBytes = (value: number) => {
  const safe = Math.max(0, value);

  let unit =
    safe < 1 ? 0 : Math.floor(Math.log(safe) / Math.log(BYTES_PER_UNIT));

  unit = Math.min(unit, BYTE_UNITS.length - 1);
  let scaled = safe / BYTES_PER_UNIT ** unit;

  // 1023.6 KB would print as four digits, so it moves up to the next unit.
  if (scaled >= 999.5 && unit < BYTE_UNITS.length - 1) {
    unit += 1;
    scaled /= BYTES_PER_UNIT;
  }

  const decimals = liveDecimals(scaled, unit);

  return `${scaled.toLocaleString(undefined, {
    maximumFractionDigits: decimals,
    minimumFractionDigits: decimals,
  })} ${BYTE_UNITS[unit]}`;
};

const SECONDS_PER_MINUTE = 60;
const MINUTES_PER_HOUR = 60;
const HOURS_PER_DAY = 24;
const MAX_DAYS = 99;

/**
 * Time left in at most about seven characters (`45s`, `12m`, `1h 12m`,
 * `2d 4h`), or `—` while it's unknown. `timeRemaining()` gives the full wording.
 */
export const compactTimeRemaining = (seconds: number | null) => {
  if (seconds === null) {
    return "—";
  }

  const safe = Math.max(0, Math.ceil(seconds));

  if (safe < SECONDS_PER_MINUTE) {
    return `${safe}s`;
  }

  const minutes = Math.ceil(safe / SECONDS_PER_MINUTE);

  if (minutes < MINUTES_PER_HOUR) {
    return `${minutes}m`;
  }

  const hours = Math.floor(minutes / MINUTES_PER_HOUR);

  if (hours < HOURS_PER_DAY) {
    return `${hours}h ${minutes % MINUTES_PER_HOUR}m`;
  }

  const days = Math.floor(hours / HOURS_PER_DAY);

  if (days > MAX_DAYS) {
    return `${MAX_DAYS}d+`;
  }

  return `${days}d ${hours % HOURS_PER_DAY}h`;
};

export const timeRemaining = (seconds: number | null) => {
  if (seconds === null) {
    return "Calculating…";
  }

  if (seconds < 60) {
    return `${seconds}s left`;
  }

  if (seconds < 3600) {
    return `${Math.ceil(seconds / 60)}m left`;
  }

  return `${Math.floor(seconds / 3600)}h ${Math.ceil((seconds % 3600) / 60)}m left`;
};

export const sourceHost = (download: Download) => {
  if (download.source === "Torrent") {
    return "BitTorrent";
  }

  try {
    return new URL(download.page_url || download.source_url).hostname.replace(
      /^www\./u,
      ""
    );
  } catch {
    return sourceNames[download.source];
  }
};
