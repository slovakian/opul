import { cn } from "cn";
import {
  ArrowUpCircle,
  Check,
  ChevronDown,
  CircleAlert,
  CircleCheck,
  Clock,
  Copy,
  Download,
  RefreshCw,
  TriangleAlert,
} from "lucide-react";
import { useState } from "react";
import type * as React from "react";

import {
  IconAction,
  InfoTip,
  ProgressDetails,
  megabytes,
} from "@/components/tools-primitives";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { CircularProgress } from "@/components/ui/circular-progress";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { useVideoTools } from "@/hooks/use-video-tools";
import { bridge, errorMessage } from "@/lib/bridge";
import type {
  SetupStep,
  UpdateFlag,
  VideoToolsSnapshot,
} from "@/lib/video-tools";
import { downloadSize } from "@/lib/video-tools";

export type Execute = ReturnType<typeof useVideoTools>["execute"];

export type VideoState = VideoToolsSnapshot["state"];

type ReadyState = Extract<VideoState, { kind: "ready" }>;

type NotWorkingState = Extract<VideoState, { kind: "not_working" }>;

const failures: Record<
  NotWorkingState["reason"],
  { hint: string; title: string }
> = {
  download: {
    hint: "The download couldn't finish.",
    title: "Download failed",
  },
  health_check: {
    hint: "The video tools didn't pass their health check.",
    title: "Health check failed",
  },
  stopped_working: {
    hint: "Files may be missing or blocked by macOS.",
    title: "Stopped working",
  },
  verification: {
    hint: "The download couldn't be verified.",
    title: "Couldn't verify",
  },
};

export const StatusBadge = ({ state }: { state: VideoState }) => {
  const content = (() => {
    switch (state.kind) {
      case "ready": {
        return (
          <Badge variant="secondary">
            <span
              aria-hidden="true"
              className="bg-foreground size-1.5 rounded-full"
            />
            Ready
          </Badge>
        );
      }

      case "setting_up": {
        return (
          <Badge variant="outline">
            <span className="shimmer">Setting up</span>
          </Badge>
        );
      }

      case "not_working": {
        return (
          <Badge variant="outline">
            <span
              aria-hidden="true"
              className="bg-destructive size-1.5 rounded-full"
            />
            Not working
          </Badge>
        );
      }

      case "turned_off": {
        return <Badge variant="outline">Turned off</Badge>;
      }

      default: {
        return (
          <Badge variant="outline" className="text-muted-foreground">
            Not installed
          </Badge>
        );
      }
    }
  })();

  return (
    <span key={state.kind} className="animate-settle inline-flex">
      {content}
    </span>
  );
};

const UpdateStatus = ({ flag }: { flag: UpdateFlag }) => {
  switch (flag.kind) {
    case "available": {
      return (
        <>
          <ArrowUpCircle aria-hidden="true" className="size-4 shrink-0" />
          <span className="truncate">{flag.version} is available.</span>
        </>
      );
    }

    case "updating": {
      return (
        <>
          <Tooltip>
            <TooltipTrigger
              render={
                <CircularProgress
                  tabIndex={0}
                  aria-label={`Updating to ${flag.version}`}
                  value={flag.progress}
                  size={16}
                  thickness={2}
                  className="focus-visible:ring-ring/50 rounded-full outline-none focus-visible:ring-3"
                />
              }
            />
            <TooltipContent>
              <ProgressDetails label="Updating" percent={flag.progress} />
            </TooltipContent>
          </Tooltip>
          <span className="shimmer truncate">Updating to {flag.version}…</span>
        </>
      );
    }

    case "failed": {
      return (
        <>
          <TriangleAlert
            aria-hidden="true"
            className="text-destructive size-4 shrink-0"
          />
          <span className="truncate">
            Couldn&apos;t update to {flag.version}
          </span>
          <InfoTip label="About this update">Opul will try again.</InfoTip>
        </>
      );
    }

    case "waiting_for_runtime": {
      return (
        <>
          <Clock
            aria-hidden="true"
            className="text-muted-foreground size-4 shrink-0"
          />
          <span className="truncate">{flag.version} needs a newer runtime</span>
          <InfoTip label="About this update">
            Opul will update when a compatible Python runtime is published.
          </InfoTip>
        </>
      );
    }

    default: {
      return (
        <>
          <CircleCheck
            aria-hidden="true"
            className="text-muted-foreground size-4 shrink-0"
          />
          <span className="text-muted-foreground">Up to date</span>
        </>
      );
    }
  }
};

const Footer = ({
  children,
  className,
  stateKey,
}: {
  children: React.ReactNode;
  className?: string;
  stateKey: string;
}) => (
  <div
    data-slot="tools-footer"
    className="bg-muted/40 flex h-13 items-center border-t px-4"
  >
    <div
      key={stateKey}
      className={cn(
        "animate-settle flex min-w-0 flex-1 items-center gap-2",
        className
      )}
    >
      {children}
    </div>
  </div>
);

const ReadyFooter = ({
  busy,
  execute,
  state,
  updateAvailable,
}: {
  busy: boolean;
  execute: Execute;
  state: ReadyState;
  updateAvailable: boolean;
}) => {
  const [checking, setChecking] = useState(false);
  const updating = state.update_flag.kind === "updating";

  const check = async () => {
    setChecking(true);
    await execute({ kind: "check_for_updates" });
    setChecking(false);
  };

  return (
    <Footer stateKey={`ready-${state.update_flag.kind}`}>
      <output className="flex min-w-0 flex-1 items-center gap-2 text-sm">
        <UpdateStatus flag={state.update_flag} />
      </output>
      <IconAction
        icon={RefreshCw}
        label="Check for updates"
        spinning={checking}
        disabled={busy || updating}
        onClick={check}
      >
        Check for updates
      </IconAction>
      {updateAvailable && (
        <Button
          type="button"
          size="sm"
          disabled={busy}
          className="animate-settle"
          onClick={() => execute({ kind: "update" })}
        >
          Update
        </Button>
      )}
    </Footer>
  );
};

const CopyDetails = () => {
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState("");

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(await bridge.videoToolsDetails());
      setCopied(true);
    } catch (error_) {
      setCopyError(errorMessage(error_));
    }
  };

  return (
    <>
      <IconAction
        icon={copied ? Check : Copy}
        label={copied ? "Copied" : "Copy details"}
        onClick={copy}
      >
        {copied ? "Copied" : "Copy details"}
      </IconAction>
      {copyError && (
        <p role="alert" className="text-destructive text-xs">
          {copyError}
        </p>
      )}
    </>
  );
};

const NotWorkingFooter = ({
  busy,
  execute,
  state,
}: {
  busy: boolean;
  execute: Execute;
  state: NotWorkingState;
}) => {
  const failure = failures[state.reason];

  return (
    <Collapsible>
      <Footer stateKey={`not-working-${state.reason}`}>
        <CircleAlert
          aria-hidden="true"
          className="text-destructive size-4 shrink-0"
        />
        <span className="flex-1 truncate text-sm">{failure.title}</span>
        <CollapsibleTrigger
          render={
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="text-muted-foreground group/details"
            />
          }
        >
          Details
          <ChevronDown
            data-icon="inline-end"
            aria-hidden="true"
            className="transition-transform duration-200 ease-(--ease-out) group-data-panel-open/details:rotate-180"
          />
        </CollapsibleTrigger>
        <Button
          type="button"
          size="sm"
          disabled={busy}
          onClick={() => execute({ kind: "retry" })}
        >
          Try again
        </Button>
      </Footer>
      <CollapsibleContent className="h-(--collapsible-panel-height) overflow-hidden transition-[height] duration-200 ease-(--ease-out) data-ending-style:h-0 data-starting-style:h-0">
        <div className="bg-muted/40 flex items-start gap-2 border-t px-4 py-3">
          <div className="flex min-w-0 flex-1 flex-col gap-1.5">
            <p className="text-sm">{failure.hint}</p>
            <pre className="text-muted-foreground scroll-fade max-h-32 overflow-auto font-mono text-xs whitespace-pre-wrap select-text">
              {state.details}
            </pre>
          </div>
          <CopyDetails />
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
};

const SetupFooter = ({
  busy,
  execute,
  steps,
}: {
  busy: boolean;
  execute: Execute;
  steps: SetupStep[];
}) => {
  const known = steps.every((step) => step.total_bytes !== null);
  const total = steps.reduce((sum, step) => sum + (step.total_bytes ?? 0), 0);

  const downloaded = steps.reduce(
    (sum, step) =>
      sum +
      (step.status === "done"
        ? (step.total_bytes ?? 0)
        : step.downloaded_bytes),
    0
  );

  return (
    <Footer stateKey="setting-up">
      <span className="text-muted-foreground flex-1 text-xs tabular-nums">
        {known && total > 0
          ? `${megabytes(downloaded)} of ${megabytes(total)} MB`
          : ""}
      </span>
      <Button
        type="button"
        variant="outline"
        size="sm"
        aria-label="Cancel setup"
        disabled={busy}
        onClick={() => execute({ kind: "cancel_setup" })}
      >
        Cancel
      </Button>
    </Footer>
  );
};

const OfferFooter = ({
  busy,
  execute,
  publishedSize,
  turnedOff,
}: {
  busy: boolean;
  execute: Execute;
  publishedSize: number | null;
  turnedOff: boolean;
}) => {
  const size = downloadSize(publishedSize);

  return (
    <Footer stateKey={turnedOff ? "turned-off" : "not-set-up"}>
      {turnedOff ? (
        <span className="flex-1" />
      ) : (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          disabled={busy}
          className="text-muted-foreground mr-auto -ml-2.5"
          onClick={() => execute({ kind: "turn_off" })}
        >
          Don&apos;t offer again
        </Button>
      )}
      <Button
        type="button"
        size="sm"
        disabled={busy}
        onClick={() => execute({ kind: "set_up" })}
      >
        <Download data-icon="inline-start" aria-hidden="true" />
        Download{" "}
        <span className="text-primary-foreground/60 font-normal tabular-nums">
          {size.replace("about ", "~")}
        </span>
      </Button>
    </Footer>
  );
};

export const VideoFooter = ({
  busy,
  execute,
  video,
}: {
  busy: boolean;
  execute: Execute;
  video: VideoToolsSnapshot;
}) => {
  const { state } = video;

  switch (state.kind) {
    case "setting_up": {
      return <SetupFooter busy={busy} execute={execute} steps={state.steps} />;
    }

    case "ready": {
      return (
        <ReadyFooter
          busy={busy}
          execute={execute}
          state={state}
          updateAvailable={video.update_available}
        />
      );
    }

    case "not_working": {
      return <NotWorkingFooter busy={busy} execute={execute} state={state} />;
    }

    default: {
      return (
        <OfferFooter
          busy={busy}
          execute={execute}
          publishedSize={video.published_size}
          turnedOff={state.kind === "turned_off"}
        />
      );
    }
  }
};
