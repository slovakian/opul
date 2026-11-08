import { cn } from "cn";
import { Check } from "lucide-react";

import {
  LiveValue,
  RowLayer,
  Separator,
  rowLine,
  useRetained,
  workingStates,
} from "@/components/download-row-parts";
import type { Completion, StatusKind } from "@/components/download-row-parts";
import { CircularProgress } from "@/components/ui/circular-progress";
import { ItemDescription } from "@/components/ui/item";
import { Progress } from "@/components/ui/progress";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useVideoTools } from "@/hooks/use-video-tools";
import type { Download } from "@/lib/model";
import { liveBytes, stateNames } from "@/lib/model";

const progress = (download: Download) =>
  Math.min(100, Math.max(0, download.progress_millionths / 10_000));

/** Shimmers only while Opul is setting video tools up; waiting on you stays still. */
const WaitingLabel = ({ download }: { download: Download }) => {
  const { tools } = useVideoTools();
  const settingUp = tools?.video.state.kind === "setting_up";

  return (
    <span className={cn(settingUp && "shimmer motion-reduce:shimmer-none")}>
      {download.waiting_label ?? stateNames.WaitingForVideoTools}
    </span>
  );
};

const StatusText = ({
  download,
  state,
}: {
  download: Download;
  state: Download["state"] | null;
}) => {
  switch (state) {
    case "Seeding": {
      return (
        <>
          <span className="sr-only">Uploading at </span>
          <LiveValue className="min-w-[9.5ch] text-right">
            <span aria-hidden>↑ </span>
            {liveBytes(download.upload_rate)}/s
          </LiveValue>
          <span className="@max-[640px]:hidden">
            <Separator />
            <LiveValue className="min-w-[3ch] text-right">
              {download.peers}
            </LiveValue>{" "}
            {download.peers === 1 ? "peer" : "peers"}
          </span>
        </>
      );
    }

    case "WaitingForVideoTools": {
      return <WaitingLabel download={download} />;
    }

    case "Failed": {
      return <span className="text-destructive">Failed</span>;
    }

    case null: {
      return null;
    }

    default: {
      return stateNames[state];
    }
  }
};

const progressValue = (download: Download) => {
  if (download.state === "Starting" || !download.total_size) {
    return null;
  }

  return progress(download);
};

const progressText = (download: Download, value: number | null) => {
  if (value === null) {
    return `${stateNames[download.state]}…`;
  }

  const percent = `${Math.round(value)}%`;

  return download.state === "Paused" || download.state === "Pausing"
    ? `Paused at ${percent}`
    : percent;
};

/**
 * Exactly one of the bar (or ring), the transient check, status text, or
 * nothing, stacked so they crossfade in one spot.
 */
export const DownloadRowStatus = ({
  download,
  completion,
  kind,
}: {
  download: Download;
  completion: Completion;
  kind: StatusKind;
}) => {
  const filling = completion.phase === "filling";

  const progressState =
    filling && completion.from ? completion.from : download.state;

  const value = filling ? 100 : progressValue(download);
  const label = progressText({ ...download, state: progressState }, value);
  const shimmer = workingStates.has(progressState);
  const muted = !shimmer;
  const textState = useRetained(kind === "text" ? download.state : null);

  return (
    <div
      data-slot="download-status"
      className="grid min-w-0 grid-cols-[minmax(0,1fr)] items-center justify-items-end overflow-hidden ps-3.5"
    >
      <RowLayer active={kind === "progress"} className="flex">
        <Tooltip>
          <TooltipTrigger
            render={
              <span className="flex h-8 w-8 items-center justify-end @min-[440px]:w-28 @min-[640px]:w-48" />
            }
          >
            <Progress
              className="w-full @max-[440px]:hidden"
              value={value}
              shimmer={shimmer}
              muted={muted}
              aria-label={`${download.name} progress`}
              getAriaValueText={() => label}
            />
            <span className="flex size-8 items-center justify-center @min-[440px]:hidden">
              <CircularProgress
                value={value}
                size={20}
                thickness={3}
                muted={muted}
                aria-label={`${download.name} progress`}
                getAriaValueText={() => label}
              />
            </span>
          </TooltipTrigger>
          <TooltipContent>{label}</TooltipContent>
        </Tooltip>
      </RowLayer>
      <RowLayer
        active={kind === "check"}
        pop
        className="text-muted-foreground flex size-8 items-center justify-center"
      >
        <Check className="size-4" aria-hidden />
        <span className="sr-only">Download complete</span>
      </RowLayer>
      <RowLayer active={kind === "text"} className="max-w-full">
        <ItemDescription className={cn(rowLine, "text-right")}>
          <StatusText download={download} state={textState} />
        </ItemDescription>
      </RowLayer>
    </div>
  );
};
