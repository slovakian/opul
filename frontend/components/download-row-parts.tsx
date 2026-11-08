import { cn } from "cn";
import { useEffect, useState } from "react";
import type { ComponentProps } from "react";

import type { Download } from "@/lib/model";
import { liveBytes } from "@/lib/model";

/** States where the bar (or, when narrow, the ring) stands for the transfer. */
export const progressStates = new Set<Download["state"]>([
  "Starting",
  "Fetching",
  "Processing",
  "Verifying",
  "Pausing",
  "Paused",
]);

/** Opul is actively working for you, so the bar shimmers. */
export const workingStates = new Set<Download["state"]>([
  "Starting",
  "Fetching",
  "Processing",
  "Verifying",
]);

/** The transfer is done: Finished, or a torrent that went on to seed. */
const completedStates = new Set<Download["state"]>(["Finished", "Seeding"]);

/** The bar's fill transition, then a short hold at 100%. */
const FILL_SETTLE_MS = 300;

/** The check's fade-in, then about 2.5s on screen. */
const CHECK_HOLD_MS = 2700;

export type CompletionPhase = "idle" | "filling" | "check";

export interface Completion {
  phase: CompletionPhase;
  /** The working state the download finished from, while the bar fills. */
  from: Download["state"] | null;
}

const idle: Completion = { from: null, phase: "idle" };
const checking: Completion = { from: null, phase: "check" };

/**
 * A row celebrates only a finish it watched happen: downloads already finished
 * at launch, or that finished while scrolled away or regrouped, never show
 * the check. The phase drives CSS transitions, so each step is interruptible.
 */
export const useCompletion = (state: Download["state"]) => {
  const [seenState, setSeenState] = useState(state);
  const [completion, setCompletion] = useState<Completion>(idle);

  if (seenState !== state) {
    setSeenState(state);
    setCompletion(
      workingStates.has(seenState) && completedStates.has(state)
        ? { from: seenState, phase: "filling" }
        : idle
    );
  }

  useEffect(() => {
    if (completion.phase === "idle") {
      return;
    }

    const filling = completion.phase === "filling";

    const timer = setTimeout(
      () => setCompletion(filling ? checking : idle),
      filling ? FILL_SETTLE_MS : CHECK_HOLD_MS
    );

    return () => clearTimeout(timer);
  }, [completion.phase]);

  return completion;
};

export type StatusKind = "progress" | "check" | "text" | "empty";

export const statusKind = (
  state: Download["state"],
  phase: CompletionPhase
): StatusKind => {
  if (phase === "check") {
    return "check";
  }

  // The bar stays to fill up to 100% before the check replaces it.
  if (phase === "filling") {
    return "progress";
  }

  if (progressStates.has(state)) {
    return "progress";
  }

  return state === "Finished" ? "empty" : "text";
};

/** Keeps the last value a layer showed, so it fades out with it instead of going blank. */
export const useRetained = <T extends string | number | boolean>(
  value: T | null
) => {
  const [retained, setRetained] = useState(value);

  if (value !== null && value !== retained) {
    setRetained(value);
  }

  return value ?? retained;
};

/** One line that never wraps; every row line and status label uses it. */
export const rowLine =
  "line-clamp-none min-w-0 truncate text-xs leading-4 tabular-nums";

export const RowLayer = ({
  active,
  pop = false,
  className,
  ...props
}: ComponentProps<"span"> & { active: boolean; pop?: boolean }) => (
  <span
    data-slot="download-row-layer"
    data-active={active}
    data-pop={pop || undefined}
    aria-hidden={active ? undefined : true}
    className={cn(
      "download-row-layer col-start-1 row-start-1 min-w-0",
      className
    )}
    {...props}
  />
);

/**
 * A value that changes while you watch gets a box wide enough for its widest
 * form, so the separators after it never move.
 */
export const LiveValue = ({ className, ...props }: ComponentProps<"span">) => (
  <span
    data-slot="download-live-value"
    className={cn("inline-block tabular-nums", className)}
    {...props}
  />
);

export const Separator = () => (
  <span data-slot="download-separator" aria-hidden className="px-1.5">
    ·
  </span>
);

export const sizeLabel = (download: Download) =>
  download.total_size ? liveBytes(download.total_size) : "Size unknown";
