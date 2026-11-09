import { Progress as ProgressPrimitive } from "@base-ui/react/progress";
import { cn } from "cn";
import type * as React from "react";

const INDETERMINATE_ARC = 0.25;

type CircularProgressProps = Omit<
  ProgressPrimitive.Root.Props,
  "className" | "style"
> & {
  className?: string;
  /**
   * Matches the bar's `motion`. Without it, an indeterminate ring spins a
   * quarter arc. With it, the ring never spins: `sheen` sweeps a soft band
   * across an empty ring until there's a value, `muted` greys the arc, and
   * `pulse` gently pulses it. Once there's a value, `sheen` is a plain arc.
   */
  motion?: "sheen" | "muted" | "pulse" | "still";
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
 * for an indeterminate ring.
 */
const CircularProgress = ({
  className,
  children,
  max = 100,
  min = 0,
  motion,
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

  const spins = indeterminate && motion === undefined;
  const sweeps = indeterminate && motion === "sheen";

  let fraction = 0;

  if (spins) {
    fraction = INDETERMINATE_ARC;
  } else if (!indeterminate) {
    fraction = Math.min(1, Math.max(0, (value - min) / (max - min || 1)));
  }

  // SAFETY: A CSS custom property is a valid key in React.CSSProperties.
  // The sweep's mask reads the stroke width.
  const ringStyle = {
    "--circular-progress-thickness": `${thickness}px`,
    height: size,
    width: size,
    ...style,
  } as React.CSSProperties;

  return (
    <ProgressPrimitive.Root
      data-slot="circular-progress"
      data-motion={motion}
      value={value ?? null}
      min={min}
      max={max}
      className={cn(
        "group/circular-progress relative inline-flex shrink-0 items-center justify-center",
        className
      )}
      style={ringStyle}
      {...props}
    >
      <svg
        aria-hidden="true"
        data-slot="circular-progress-circle"
        viewBox={`0 0 ${size} ${size}`}
        width={size}
        height={size}
        className={cn(
          "-rotate-90 overflow-visible",
          spins && "animate-[circular-progress-spin_0.8s_linear_infinite]"
        )}
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
            "stroke-primary transition-[stroke-dashoffset,opacity,stroke] duration-500 ease-(--ease-out)",
            motion === "muted" && "stroke-muted-foreground/40",
            fraction === 0 && "opacity-0"
          )}
        />
      </svg>
      {sweeps && <span aria-hidden data-slot="circular-progress-sheen" />}
      {children}
    </ProgressPrimitive.Root>
  );
};

export { CircularProgress };
