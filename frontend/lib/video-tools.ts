import { z } from "zod";

export const updateNoticeSchema = z.object({
  identity: z.string(),
  version: z.string(),
});

const installedPairSchema = z.object({
  python_version: z.string(),
  runtime: z.number().int().positive(),
  yt_dlp: z.string(),
});

const updateFlagSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("none") }),
  z.object({ kind: z.literal("available"), version: z.string() }),
  z.object({
    kind: z.literal("updating"),
    progress: z.number(),
    version: z.string(),
  }),
  z.object({ kind: z.literal("failed"), version: z.string() }),
  z.object({ kind: z.literal("waiting_for_runtime"), version: z.string() }),
]);

export const setupStepSchema = z.object({
  component: z.enum(["python", "yt_dlp"]),
  downloaded_bytes: z.number(),
  status: z.enum([
    "pending",
    "waiting_for_internet",
    "downloading",
    "verifying",
    "installing",
    "done",
    "failed",
  ]),
  total_bytes: z.number().nullable(),
});

export const videoToolsSchema = z.object({
  keep_up_to_date: z.boolean(),
  nightly_available: z.boolean(),
  published_size: z.number().nullable(),
  sequence: z.number(),
  state: z.discriminatedUnion("kind", [
    z.object({ kind: z.literal("not_set_up") }),
    z.object({ kind: z.literal("turned_off") }),
    z.object({
      kind: z.literal("setting_up"),
      steps: z.array(setupStepSchema),
    }),
    z.object({
      installed: installedPairSchema,
      kind: z.literal("ready"),
      last_checked: z.number().nullable(),
      update_flag: updateFlagSchema,
    }),
    z.object({
      details: z.string(),
      kind: z.literal("not_working"),
      reason: z.enum([
        "download",
        "verification",
        "health_check",
        "stopped_working",
      ]),
    }),
  ]),
  update_available: z.boolean(),
  use_nightly: z.boolean(),
});

export const toolsSnapshotSchema = z.object({
  built_in: z.object({
    ffmpeg: z.string().nullable(),
    ffprobe: z.string().nullable(),
  }),
  video: videoToolsSchema,
});

export type VideoToolsSnapshot = z.infer<typeof videoToolsSchema>;

export type ToolsSnapshot = z.infer<typeof toolsSnapshotSchema>;

export type SetupStep = z.infer<typeof setupStepSchema>;

export type UpdateFlag = z.infer<typeof updateFlagSchema>;

export type VideoToolsOperation =
  | { kind: "set_up" }
  | { kind: "remove"; turn_off: boolean }
  | { kind: "cancel_setup" }
  | { kind: "check_for_updates" }
  | { kind: "update" }
  | { kind: "retry" }
  | { kind: "turn_off" }
  | { kind: "refresh_published_size" }
  | { kind: "set_keep_up_to_date"; enabled: boolean }
  | { kind: "set_use_nightly"; enabled: boolean };

export const downloadSize = (bytes: number | null) =>
  bytes === null ? "about 23 MB" : `${(bytes / 1_000_000).toFixed(1)} MB`;

const SECONDS_PER_MINUTE = 60;

const relativeUnits = [
  ["year", 31_536_000],
  ["month", 2_592_000],
  ["week", 604_800],
  ["day", 86_400],
  ["hour", 3600],
  ["minute", SECONDS_PER_MINUTE],
] as const;

const relativeTime = new Intl.RelativeTimeFormat(undefined, {
  numeric: "auto",
});

/** "checked 3 days ago" for a Unix timestamp in seconds. */
export const checkedAgo = (seconds: number, now = Date.now()): string => {
  const elapsed = now / 1000 - seconds;

  for (const [unit, size] of relativeUnits) {
    if (elapsed >= size) {
      return `checked ${relativeTime.format(-Math.floor(elapsed / size), unit)}`;
    }
  }

  return "checked just now";
};

export const setupProgress = (steps: SetupStep[]): string => {
  if (steps.length === 0) {
    return "";
  }

  let progress = 0;

  for (const step of steps) {
    if (step.status === "done") {
      progress += 1;
    } else if (step.total_bytes) {
      progress += Math.min(1, step.downloaded_bytes / step.total_bytes);
    }
  }

  return `${Math.floor((progress / steps.length) * 100)}%`;
};
