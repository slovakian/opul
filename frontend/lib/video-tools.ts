import { z } from "zod";

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
  | { kind: "cancel_setup" }
  | { kind: "retry" }
  | { kind: "turn_off" }
  | { kind: "refresh_published_size" }
  | { kind: "set_keep_up_to_date"; enabled: boolean };

export const downloadSize = (bytes: number | null) =>
  bytes === null ? "about 23 MB" : `${(bytes / 1_000_000).toFixed(1)} MB`;

export const updateMessage = (flag: UpdateFlag): string | null => {
  switch (flag.kind) {
    case "none": {
      return null;
    }

    case "available": {
      return `${flag.version} is available.`;
    }

    case "updating": {
      return `Updating to ${flag.version}… ${flag.progress}%`;
    }

    case "failed": {
      return `Couldn't update to ${flag.version}. Opul will try again.`;
    }

    case "waiting_for_runtime": {
      return `${flag.version} needs a newer Python runtime. Opul will update when one is available.`;
    }

    default: {
      const exhaustive: never = flag;

      return exhaustive;
    }
  }
};
