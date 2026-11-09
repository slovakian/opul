"use client";

import { Progress as ProgressPrimitive } from "@base-ui/react/progress";
import { cn } from "cn";
import type * as React from "react";

const ProgressIndicator = ({
  className,
  ...props
}: ProgressPrimitive.Indicator.Props) => (
  <ProgressPrimitive.Indicator
    data-slot="progress-indicator"
    // An indeterminate bar has no inline width, so `w-0` lets the fill grow in
    // from nothing once a value arrives.
    className={cn(
      "bg-primary relative h-full w-0 overflow-hidden rounded-full transition-[width,background-color] duration-300 ease-(--ease-out)",
      className
    )}
    {...props}
  />
);

const ProgressTrack = ({
  className,
  ...props
}: ProgressPrimitive.Track.Props) => (
  <ProgressPrimitive.Track
    className={cn(
      "bg-muted relative flex h-1.5 w-full items-center overflow-x-hidden rounded-full",
      className
    )}
    data-slot="progress-track"
    {...props}
  />
);

/** How a bar moves; the motion itself lives in styles.css. */
type ProgressMotion = "sheen" | "muted" | "pulse" | "still" | "searching";

type ProgressProps = ProgressPrimitive.Root.Props & {
  /**
   * - `sheen`: a soft highlight sweeps the empty track (never the fill): left
   *   to right with no value yet, right to left toward the fill once there is
   *   one. Switching between the two crossfades.
   * - `muted`: the fill greys out, for work that's held.
   * - `pulse`: the fill gently pulses, for work that's resuming.
   * - `still`: no motion.
   * - `searching`: the fill greys out and breathes slowly, for work that's
   *   waiting on someone else (a torrent looking for peers).
   */
  motion?: ProgressMotion;
};

const PERCENT = 100;

const Progress = ({
  className,
  children,
  value,
  min = 0,
  max = 100,
  motion,
  ...props
}: ProgressProps) => {
  const fraction =
    value === null || value === undefined
      ? 0
      : Math.min(1, Math.max(0, (value - min) / (max - min || 1)));

  // SAFETY: A CSS custom property is a valid key in React.CSSProperties.
  // The sheen with a value is clipped to the empty part of the track.
  const valueStyle = {
    "--progress-value": `${fraction * PERCENT}%`,
  } as React.CSSProperties;

  return (
    <ProgressPrimitive.Root
      value={value}
      min={min}
      max={max}
      data-slot="progress"
      data-motion={motion}
      className={cn("group/progress flex flex-wrap gap-3", className)}
      style={valueStyle}
      {...props}
    >
      {children}
      <ProgressTrack>
        <ProgressIndicator className="group-data-[motion=muted]/progress:bg-muted-foreground/40 group-data-[motion=searching]/progress:bg-muted-foreground/60" />
        {motion && (
          <>
            <span
              aria-hidden
              data-slot="progress-sheen"
              data-kind="indeterminate"
            />
            <span
              aria-hidden
              data-slot="progress-sheen"
              data-kind="determinate"
            />
          </>
        )}
      </ProgressTrack>
    </ProgressPrimitive.Root>
  );
};

const ProgressLabel = ({
  className,
  ...props
}: ProgressPrimitive.Label.Props) => (
  <ProgressPrimitive.Label
    className={cn("text-sm font-medium", className)}
    data-slot="progress-label"
    {...props}
  />
);

const ProgressValue = ({
  className,
  ...props
}: ProgressPrimitive.Value.Props) => (
  <ProgressPrimitive.Value
    className={cn(
      "text-muted-foreground ml-auto text-sm tabular-nums",
      className
    )}
    data-slot="progress-value"
    {...props}
  />
);

export {
  Progress,
  ProgressTrack,
  ProgressIndicator,
  ProgressLabel,
  ProgressValue,
};
