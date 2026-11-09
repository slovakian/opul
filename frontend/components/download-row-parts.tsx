import { cn } from "cn";
import { useEffect, useState } from "react";
import type { ComponentProps } from "react";

import type { Download } from "@/lib/model";
import { liveBytes } from "@/lib/model";
import { isStalled, workingStates } from "@/lib/row-view";
import type { RowView } from "@/lib/row-view";

/**
 * The transfer is done: Finished, or a torrent that went on to seed (or
 * stopped at once). Checking files isn't here: that's a complete torrent
 * starting to seed again, which never replays the check.
 */
const completedStates = new Set<Download["state"]>([
  "Finished",
  "Seeding",
  "NotSeeding",
]);

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
  status: RowView["status"],
  phase: CompletionPhase
): StatusKind => {
  if (phase === "check") {
    return "check";
  }

  // The bar stays to fill up to 100% before the check replaces it.
  return phase === "filling" ? "progress" : status;
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

export const Separator = () => (
  <span data-slot="download-separator" aria-hidden className="px-1.5">
    ·
  </span>
);

export const sizeLabel = (download: Download) =>
  download.total_size ? liveBytes(download.total_size) : "Size unknown";

/** How long a live value stays after it drops to nothing, so a brief stall doesn't flicker. */
const LINGER_MS = 3000;

/** Matches the segment fade in styles.css. */
const FADE_MS = 200;

export type LingerPhase = "shown" | "fading" | "gone";

/**
 * Shows a value as soon as it's there and keeps its last real value for about
 * 3s after it goes away, then fades it out before it's removed.
 */
export const useLinger = (
  value: number | null,
  /** Starts as if the value had just gone, so even a first absence waits. */
  graceAtStart = false
) => {
  const [last, setLast] = useState(value);

  const [phase, setPhase] = useState<LingerPhase>(
    value === null && !graceAtStart ? "gone" : "shown"
  );

  if (value !== null && (value !== last || phase !== "shown")) {
    setLast(value);
    setPhase("shown");
  }

  const absent = value === null;

  useEffect(() => {
    if (!absent || phase === "gone") {
      return;
    }

    const lingering = phase === "shown";

    const timer = setTimeout(
      () => setPhase(lingering ? "fading" : "gone"),
      lingering ? LINGER_MS : FADE_MS
    );

    return () => clearTimeout(timer);
  }, [absent, phase]);

  return { phase, value: phase === "gone" ? null : last };
};

/**
 * One piece of a row line with the separator before it, so hiding the piece
 * hides its separator too. A lingering piece fades out before it's removed.
 */
export const Segment = ({
  first = false,
  phase = "shown",
  className,
  children,
  ...props
}: ComponentProps<"span"> & { first?: boolean; phase?: LingerPhase }) => (
  <span
    data-slot="download-segment"
    data-phase={phase}
    className={cn("download-segment", className)}
    {...props}
  >
    {!first && <Separator />}
    {children}
  </span>
);

/**
 * A torrent stalled at 0 B/s with nobody connected says "Looking for peers…"
 * in place of its bar once that has lasted as long as speed lingers, and goes
 * back to the bar as soon as data or a peer arrives. It's the speed's grace
 * and linger, run on "something is happening".
 */
export const useLookingForPeers = (download: Download) => {
  const stalled = isStalled(download);
  const active = useLinger(stalled ? null : 1, true);

  return stalled && active.phase === "gone";
};
