import type * as React from "react";

import {
  InfoTip,
  SettledStepMark,
  StepRing,
  isSettledStep,
  megabytes,
  videoToolParts,
} from "@/components/tools-primitives";
import {
  Trace,
  TraceDescription,
  TraceLine,
  TraceTitle,
} from "@/components/ui/trace";
import type { SetupStep } from "@/lib/video-tools";

const statusLabels: Record<SetupStep["status"], string> = {
  done: "Done",
  downloading: "Downloading",
  failed: "Failed",
  installing: "Installing",
  pending: "Waiting",
  verifying: "Verifying",
  waiting_for_internet: "Waiting for an internet connection…",
};

const lineStatus = (
  status: SetupStep["status"]
): React.ComponentProps<typeof TraceLine>["status"] => {
  switch (status) {
    case "downloading":
    case "installing":
    case "verifying": {
      return "running";
    }

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
    ? `${megabytes(step.downloaded_bytes)} of ${megabytes(step.total_bytes)} MB`
    : statusLabels[step.status];

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
