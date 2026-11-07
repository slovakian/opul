import { Check, Circle } from "lucide-react";

import { Progress } from "@/components/ui/progress";
import {
  Trace,
  TraceDescription,
  TraceIcon,
  TraceLine,
  TraceTitle,
} from "@/components/ui/trace";
import type { SetupStep } from "@/lib/video-tools";

const statusLabels = {
  done: "Done",
  downloading: "Downloading",
  failed: "Failed",
  installing: "Installing",
  pending: "Waiting",
  verifying: "Verifying",
  waiting_for_internet: "Waiting for an internet connection…",
};

export const VideoToolsProgress = ({ steps }: { steps: SetupStep[] }) => (
  <Trace aria-label="Video tools setup progress">
    {steps.map((step) => {
      const python = step.component === "python";
      const title = python ? "Setting up Python" : "Setting up yt-dlp";
      const done = step.status === "done";

      const progress = step.total_bytes
        ? Math.min(100, (step.downloaded_bytes / step.total_bytes) * 100)
        : null;

      return (
        <div key={step.component} className="flex flex-col gap-2">
          <TraceLine>
            <TraceIcon>{done ? <Check /> : <Circle />}</TraceIcon>
            <TraceTitle>{title}</TraceTitle>
            <TraceDescription>{statusLabels[step.status]}</TraceDescription>
          </TraceLine>
          <p className="text-muted-foreground text-xs">
            {python
              ? "The language yt-dlp runs on, plus a JavaScript engine for some sites."
              : "Downloads videos and audio from video pages."}
          </p>
          <Progress
            aria-label={`${title} progress`}
            value={done ? 100 : progress}
          />
        </div>
      );
    })}
  </Trace>
);
