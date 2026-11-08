"use client";

import { Progress as ProgressPrimitive } from "@base-ui/react/progress";
import { cn } from "cn";
import type * as React from "react";

const INDETERMINATE_ARC = 0.25;

type CircularProgressProps = Omit<
  ProgressPrimitive.Root.Props,
  "className" | "style"
> & {
  className?: string;
  /** Diameter in pixels. */
  size?: number;
  style?: React.CSSProperties;
  /** Stroke width in pixels. */
  thickness?: number;
};

/**
 * A ring-shaped progress indicator built on Base UI's Progress root, so it
 * keeps `role="progressbar"`, `aria-valuenow` and the `data-indeterminate`,
 * `data-progressing` and `data-complete` state attributes. Pass `value={null}`
 * for an indeterminate spinner.
 */
const CircularProgress = ({
  className,
  children,
  max = 100,
  min = 0,
  size = 32,
  style,
  thickness = 4,
  value,
  ...props
}: CircularProgressProps) => {
  const center = size / 2;
  const radius = (size - thickness) / 2;
  const circumference = 2 * Math.PI * radius;
  const indeterminate = value === null || value === undefined;

  const fraction = indeterminate
    ? INDETERMINATE_ARC
    : Math.min(1, Math.max(0, (value - min) / (max - min || 1)));

  return (
    <ProgressPrimitive.Root
      data-slot="circular-progress"
      value={value ?? null}
      min={min}
      max={max}
      className={cn(
        "group/circular-progress relative inline-flex shrink-0 items-center justify-center",
        className
      )}
      style={
        // SAFETY: Custom properties (--circumference, --percent, --size,
        // --thickness) are valid CSS but absent from React.CSSProperties.
        {
          "--circumference": circumference,
          "--percent": Math.round(fraction * 100),
          "--size": `${size}px`,
          "--thickness": `${thickness}px`,
          height: size,
          width: size,
          ...style,
        } as React.CSSProperties
      }
      {...props}
    >
      <svg
        aria-hidden="true"
        data-slot="circular-progress-circle"
        viewBox={`0 0 ${size} ${size}`}
        width={size}
        height={size}
        className="-rotate-90 overflow-visible group-data-indeterminate/circular-progress:animate-[circular-progress-spin_0.8s_linear_infinite]"
      >
        <circle
          data-slot="circular-progress-track"
          cx={center}
          cy={center}
          r={radius}
          fill="none"
          strokeWidth={thickness}
          className="stroke-muted"
        />
        <circle
          data-slot="circular-progress-range"
          cx={center}
          cy={center}
          r={radius}
          fill="none"
          strokeWidth={thickness}
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={circumference * (1 - fraction)}
          className={cn(
            "stroke-primary transition-[stroke-dashoffset,opacity] duration-500 ease-(--ease-out)",
            fraction === 0 && "opacity-0"
          )}
        />
      </svg>
      {children}
    </ProgressPrimitive.Root>
  );
};

const CircularProgressValue = ({
  className,
  ...props
}: ProgressPrimitive.Value.Props) => (
  <ProgressPrimitive.Value
    data-slot="circular-progress-value"
    className={cn(
      "absolute inset-0 flex items-center justify-center text-[0.625rem] font-medium tabular-nums",
      className
    )}
    {...props}
  />
);

export { CircularProgress, CircularProgressValue };
