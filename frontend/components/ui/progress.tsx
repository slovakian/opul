"use client";

import { Progress as ProgressPrimitive } from "@base-ui/react/progress";
import { cn } from "cn";

const ProgressIndicator = ({
  className,
  ...props
}: ProgressPrimitive.Indicator.Props) => (
  <ProgressPrimitive.Indicator
    data-slot="progress-indicator"
    // An indeterminate bar has no inline width, so `w-0` lets the fill grow in
    // from nothing once a value arrives.
    className={cn(
      "bg-primary relative h-full w-0 overflow-hidden rounded-full transition-all",
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

type ProgressProps = ProgressPrimitive.Root.Props & {
  /**
   * Sweeps a soft highlight across the fill, or across the whole track while
   * indeterminate, to say the work is moving. See `.progress-shimmer` in styles.css.
   */
  shimmer?: boolean;
  /** Mutes the fill, for work that's held rather than moving. */
  muted?: boolean;
};

const Progress = ({
  className,
  children,
  value,
  shimmer = false,
  muted = false,
  ...props
}: ProgressProps) => (
  <ProgressPrimitive.Root
    value={value}
    data-slot="progress"
    data-shimmer={shimmer || undefined}
    data-muted={muted || undefined}
    className={cn("group/progress flex flex-wrap gap-3", className)}
    {...props}
  >
    {children}
    <ProgressTrack>
      <ProgressIndicator className="group-data-muted/progress:bg-muted-foreground/40" />
    </ProgressTrack>
  </ProgressPrimitive.Root>
);

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
