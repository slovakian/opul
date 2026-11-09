import { z } from "zod";

const version = z.string();

const phaseSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("idle") }),
  z.object({ kind: z.literal("checking") }),
  z.object({ kind: z.literal("up_to_date") }),
  z.object({ kind: z.literal("available"), version }),
  z.object({
    automatic: z.boolean(),
    downloaded: z.number(),
    kind: z.literal("downloading"),
    total: z.number().nullable(),
    version,
  }),
  z.object({ kind: z.literal("ready"), version }),
  z.object({ kind: z.literal("check_failed"), message: z.string() }),
  z.object({
    automatic: z.boolean(),
    kind: z.literal("download_failed"),
    message: z.string(),
    version,
  }),
]);

export const appUpdatesSchema = z.object({
  channel: z.enum(["stable", "nightly"]),
  check_automatically: z.boolean(),
  current_version: z.string(),
  install_automatically: z.boolean(),
  install_blocker: z
    .enum(["development", "translocated", "disk_image"])
    .nullable(),
  last_checked: z.number().nullable(),
  phase: phaseSchema,
  release_notes_url: z.string().nullable(),
  sequence: z.number(),
});

export type AppUpdates = z.infer<typeof appUpdatesSchema>;

export type AppUpdatePhase = AppUpdates["phase"];

export type InstallBlocker = NonNullable<AppUpdates["install_blocker"]>;

export const blockerText: Record<InstallBlocker, string> = {
  development: "Development builds can't install updates.",
  disk_image: "Move Opul to your Applications folder to install updates.",
  translocated: "Move Opul to your Applications folder to install updates.",
};

const PERCENT = 100;

/** Whole percent downloaded, or null while the size is unknown. */
export const downloadPercent = (
  phase: Extract<AppUpdatePhase, { kind: "downloading" }>
): number | null =>
  phase.total
    ? Math.min(PERCENT, Math.round((phase.downloaded / phase.total) * PERCENT))
    : null;

/** Settings marks the Updates section while something needs a decision. */
export const needsAttention = (updates: AppUpdates | null): boolean => {
  const kind = updates?.phase.kind;

  return kind === "available" || kind === "download_failed" || kind === "ready";
};

/** Automatic installs only run while automatic checks do. */
export const installsAutomatically = (updates: AppUpdates): boolean =>
  updates.check_automatically && updates.install_automatically;

export const appName = (updates: Pick<AppUpdates, "channel">): string =>
  updates.channel === "nightly" ? "Opul Nightly" : "Opul";
