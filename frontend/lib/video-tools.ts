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

const setupStepSchema = z.object({
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

export type NotWorkingState = Extract<
  VideoToolsSnapshot["state"],
  { kind: "not_working" }
>;

export const PERCENT = 100;

export type VideoToolsOperation =
  | { kind: "set_up" }
  | { kind: "cancel_setup" }
  | { kind: "check_for_updates" }
  | { kind: "update" }
  | { kind: "retry" }
  | { kind: "turn_off" }
  | { kind: "refresh_published_size" }
  | { kind: "set_keep_up_to_date"; enabled: boolean }
  | { kind: "set_use_nightly"; enabled: boolean };

const BYTES_PER_MB = 1_000_000;

const megabytes = (bytes: number) => (bytes / BYTES_PER_MB).toFixed(1);

export const byteProgress = (downloaded: number, total: number) =>
  `${megabytes(downloaded)} of ${megabytes(total)} MB`;

/** Before the size is published, an estimate: "about 23 MB", or "~23 MB" when compact. */
export const downloadSize = (
  bytes: number | null,
  { compact = false } = {}
) => {
  if (bytes !== null) {
    return `${megabytes(bytes)} MB`;
  }

  return compact ? "~23 MB" : "about 23 MB";
};

type StepStatus = SetupStep["status"];

export const isWorkingStep = (status: StepStatus) =>
  status === "downloading" || status === "verifying" || status === "installing";

/** Working, but with no byte count to show progress against. */
export const isIndeterminateStep = (status: StepStatus) =>
  status === "verifying" || status === "installing";

export const isSettledStep = (step: SetupStep) =>
  step.status === "done" || step.status === "failed";

const stepLabels: Record<StepStatus, string> = {
  done: "Done",
  downloading: "Downloading",
  failed: "Failed",
  installing: "Installing",
  pending: "Waiting",
  verifying: "Verifying",
  waiting_for_internet: "Waiting for an internet connection…",
};

/** Settings ▸ Tools has a narrow column, so it uses the short wording. */
export const stepLabel = (status: StepStatus, { compact = false } = {}) =>
  compact && status === "waiting_for_internet"
    ? "Waiting for internet…"
    : stepLabels[status];

type FailureReason = NotWorkingState["reason"];

/** A short title, the sentence Setup shows, and the hint Settings shows. */
export const failureText: Record<
  FailureReason,
  { hint: string; summary: string; title: string }
> = {
  download: {
    hint: "The download couldn't finish.",
    summary: "The download couldn't finish",
    title: "Download failed",
  },
  health_check: {
    hint: "The video tools didn't pass their health check.",
    summary: "The video tools didn't pass their health check",
    title: "Health check failed",
  },
  stopped_working: {
    hint: "Files may be missing or blocked by macOS.",
    summary: "The video tools stopped working",
    title: "Stopped working",
  },
  verification: {
    hint: "The download couldn't be verified.",
    summary: "The download couldn't be verified",
    title: "Couldn't verify",
  },
};

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
export const checkedAgo = (seconds: number): string => {
  const elapsed = Date.now() / 1000 - seconds;

  for (const [unit, size] of relativeUnits) {
    if (elapsed >= size) {
      return `checked ${relativeTime.format(-Math.floor(elapsed / size), unit)}`;
    }
  }

  return "checked just now";
};

export const stepFraction = (step: SetupStep): number => {
  if (step.status === "done") {
    return 1;
  }

  return step.total_bytes
    ? Math.min(1, step.downloaded_bytes / step.total_bytes)
    : 0;
};

/** Overall setup progress from 0 to 1, counting each step equally. */
export const setupFraction = (steps: SetupStep[]): number => {
  if (steps.length === 0) {
    return 0;
  }

  let progress = 0;

  for (const step of steps) {
    progress += stepFraction(step);
  }

  return progress / steps.length;
};

export const setupProgress = (steps: SetupStep[]): string =>
  steps.length === 0 ? "" : `${Math.floor(setupFraction(steps) * 100)}%`;
