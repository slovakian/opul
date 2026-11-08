import type * as React from "react";

import {
  InfoTip,
  SettledStepMark,
  StepRing,
  videoToolParts,
} from "@/components/tools-primitives";
import {
  Trace,
  TraceDescription,
  TraceLine,
  TraceTitle,
} from "@/components/ui/trace";
import type { SetupStep } from "@/lib/video-tools";
import {
  byteProgress,
  isSettledStep,
  isWorkingStep,
  stepLabel,
} from "@/lib/video-tools";

const lineStatus = (
  status: SetupStep["status"]
): React.ComponentProps<typeof TraceLine>["status"] => {
  if (isWorkingStep(status)) {
    return "running";
  }

  switch (status) {
    case "done": {
      return "available";
    }

    case "failed": {
      return "error";
    }

    case "pending": {
      return "pending";
    }

    default: {
      return undefined;
    }
  }
};

/** Byte counts say more than "Downloading" once the size is known. */
const stepDetail = (step: SetupStep) =>
  step.status === "downloading" && step.total_bytes
    ? byteProgress(step.downloaded_bytes, step.total_bytes)
    : stepLabel(step.status);

const StepLine = ({ step }: { step: SetupStep }) => {
  const { hint, name } = videoToolParts[step.component];
  const title = `Setting up ${name}`;
  const detail = stepDetail(step);

  return (
    <TraceLine
      status={lineStatus(step.status)}
      className="h-6 duration-300"
      data-step={step.component}
    >
      {/* Not TraceIcon: that hides its content, and the ring is the step's progressbar. */}
      <span className="flex size-4 shrink-0 items-center justify-center">
        {isSettledStep(step) ? (
          <SettledStepMark label={title} step={step} className="size-4" />
        ) : (
          <StepRing label={title} step={step} size={16} thickness={2} />
        )}
      </span>
      <TraceTitle className="flex-none">{title}</TraceTitle>
      <InfoTip label={`About ${name}`}>{hint}</InfoTip>
      <TraceDescription
        // Only the wording swaps in; ticking byte counts stay put.
        key={step.status}
        className="animate-settle ml-auto"
      >
        {detail}
      </TraceDescription>
    </TraceLine>
  );
};

export const VideoToolsProgress = ({ steps }: { steps: SetupStep[] }) => (
  <Trace aria-label="Video tools setup progress" className="gap-3">
    {steps.map((step) => (
      <StepLine key={step.component} step={step} />
    ))}
  </Trace>
);
