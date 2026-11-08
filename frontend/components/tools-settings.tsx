import { Film, Maximize2 } from "lucide-react";
import type * as React from "react";

import {
  IconAction,
  InfoTip,
  ProgressDetails,
  SectionHeading,
  SettledStepMark,
  StepRing,
  ToolRow,
  stepPercent,
  videoToolParts,
} from "@/components/tools-primitives";
import { Badge } from "@/components/ui/badge";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  Item,
  ItemActions,
  ItemContent,
  ItemDescription,
  ItemMedia,
  ItemTitle,
} from "@/components/ui/item";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { StatusBadge, VideoFooter } from "@/components/video-tools-footer";
import type { Execute } from "@/components/video-tools-footer";
import { VideoToolsRemoval } from "@/components/video-tools-removal";
import { useVideoTools } from "@/hooks/use-video-tools";
import type { SetupStep, VideoToolsSnapshot } from "@/lib/video-tools";
import {
  byteProgress,
  checkedAgo,
  isIndeterminateStep,
  isSettledStep,
  isWorkingStep,
  stepLabel,
} from "@/lib/video-tools";

const STAGGER_MS = 40;

const stagger = (index: number): React.CSSProperties => ({
  animationDelay: `${index * STAGGER_MS}ms`,
});

const StepIndicator = ({ label, step }: { label: string; step: SetupStep }) => {
  if (isSettledStep(step)) {
    return <SettledStepMark label={label} step={step} className="size-4" />;
  }

  const indeterminate = isIndeterminateStep(step.status);

  const percent = stepPercent(step);

  return (
    <Tooltip>
      <TooltipTrigger
        render={
          <StepRing
            tabIndex={0}
            label={label}
            step={step}
            size={20}
            thickness={2.5}
            className="focus-visible:ring-ring/50 rounded-full outline-none focus-visible:ring-3"
          />
        }
      />
      <TooltipContent side="left">
        <ProgressDetails
          label={stepLabel(step.status, { compact: true })}
          percent={indeterminate ? null : percent}
          detail={
            step.total_bytes === null
              ? undefined
              : byteProgress(step.downloaded_bytes, step.total_bytes)
          }
        />
      </TooltipContent>
    </Tooltip>
  );
};

const componentRow = (
  component: keyof typeof videoToolParts,
  video: VideoToolsSnapshot
): React.ComponentProps<typeof ToolRow> => {
  const { hint, icon, name } = videoToolParts[component];
  const { state } = video;
  const base = { hint, icon, name };

  switch (state.kind) {
    case "setting_up": {
      const step = state.steps.find((item) => item.component === component);

      if (!step) {
        return { ...base, description: "Waiting", dimmed: true };
      }

      return {
        ...base,
        active: isWorkingStep(step.status),
        description: stepLabel(step.status, { compact: true }),
        dimmed: step.status === "pending",
        trailing: <StepIndicator label={`Setting up ${name}`} step={step} />,
      };
    }

    case "ready": {
      const { installed, last_checked: lastChecked } = state;

      if (component === "python") {
        return {
          ...base,
          description: `${installed.python_version} · runtime ${installed.runtime}`,
        };
      }

      return {
        ...base,
        description:
          lastChecked === null
            ? installed.yt_dlp
            : `${installed.yt_dlp} · ${checkedAgo(lastChecked)}`,
        trailing: video.use_nightly ? (
          <Badge variant="outline" className="animate-settle">
            Nightly
          </Badge>
        ) : undefined,
      };
    }

    case "not_working": {
      return { ...base, description: "Unavailable", dimmed: true };
    }

    default: {
      return { ...base, description: "Not installed", dimmed: true };
    }
  }
};

const BuiltInTools = ({
  ffmpeg,
  ffprobe,
}: {
  ffmpeg: string | null;
  ffprobe: string | null;
}) => (
  <section
    aria-labelledby="built-in-tools"
    className="animate-settle flex flex-col gap-1.5"
    style={stagger(0)}
  >
    <SectionHeading
      id="built-in-tools"
      hint="Ships inside Opul. Torrents and direct files need nothing else."
    >
      Included
    </SectionHeading>
    <div className="bg-card overflow-hidden rounded-xl border">
      <Item size="sm" className="flex-nowrap rounded-none px-4 py-3">
        <ItemMedia
          variant="icon"
          className="bg-muted size-8 translate-y-0! self-center! rounded-lg"
        >
          <Film />
        </ItemMedia>
        <ItemContent className="min-w-0 gap-0.5">
          <ItemTitle>FFmpeg</ItemTitle>
          <ItemDescription className="line-clamp-1 text-xs tabular-nums">
            {ffmpeg
              ? `${ffmpeg} · ffprobe ${ffprobe ?? "unavailable"}`
              : "Unavailable"}
          </ItemDescription>
        </ItemContent>
        <ItemActions>
          {ffmpeg ? (
            <Badge variant="secondary">Built in</Badge>
          ) : (
            <Badge variant="outline">
              <span
                aria-hidden="true"
                className="bg-destructive size-1.5 rounded-full"
              />
              Missing
            </Badge>
          )}
        </ItemActions>
      </Item>
    </div>
  </section>
);

const VideoTools = ({
  busy,
  execute,
  onVideoSetup,
  video,
}: {
  busy: boolean;
  execute: Execute;
  onVideoSetup: () => void;
  video: VideoToolsSnapshot;
}) => {
  const { state } = video;
  const canOpenSetup = state.kind !== "ready" && state.kind !== "not_working";

  return (
    <section
      aria-labelledby="video-tools"
      className="animate-settle flex flex-col gap-1.5"
      style={stagger(1)}
    >
      <SectionHeading
        id="video-tools"
        hint="Download videos and audio from video pages."
        trailing={
          <>
            {canOpenSetup && (
              <IconAction
                icon={Maximize2}
                label="Open Setup"
                size="icon-xs"
                onClick={onVideoSetup}
              >
                Set Up Video Tools…
              </IconAction>
            )}
            <StatusBadge state={state} />
          </>
        }
      >
        Video tools
      </SectionHeading>
      <div className="bg-card overflow-hidden rounded-xl border">
        <ToolRow {...componentRow("python", video)} />
        <Separator />
        <ToolRow {...componentRow("yt_dlp", video)} />
        <VideoFooter busy={busy} execute={execute} video={video} />
      </div>
    </section>
  );
};

const VideoPreferences = ({
  busy,
  execute,
  video,
}: {
  busy: boolean;
  execute: Execute;
  video: VideoToolsSnapshot;
}) => {
  const removable =
    video.state.kind === "ready" || video.state.kind === "not_working";

  return (
    <section
      aria-label="Video tools preferences"
      className="animate-settle bg-card divide-y overflow-hidden rounded-xl border"
      style={stagger(2)}
    >
      <Field orientation="horizontal" className="min-h-12 px-4 py-2.5">
        <FieldLabel
          htmlFor="keep-video-tools-up-to-date"
          className="font-normal"
        >
          Keep up to date
        </FieldLabel>
        <Switch
          id="keep-video-tools-up-to-date"
          checked={video.keep_up_to_date}
          disabled={busy}
          onCheckedChange={(enabled) =>
            execute({ enabled, kind: "set_keep_up_to_date" })
          }
        />
      </Field>
      {video.nightly_available && (
        <Field orientation="horizontal" className="min-h-12 px-4 py-2.5">
          <div className="flex flex-1 items-center gap-1.5">
            <FieldLabel htmlFor="use-yt-dlp-nightly" className="font-normal">
              Nightly yt-dlp
            </FieldLabel>
            <InfoTip label="About nightly yt-dlp">
              Early yt-dlp builds with the newest site fixes. Applies right
              away.
            </InfoTip>
          </div>
          <Switch
            id="use-yt-dlp-nightly"
            checked={video.use_nightly}
            disabled={busy}
            onCheckedChange={(enabled) =>
              execute({ enabled, kind: "set_use_nightly" })
            }
          />
        </Field>
      )}
      {removable && (
        <div className="animate-settle flex min-h-12 items-center gap-3 px-4 py-2">
          <span className="flex-1 text-sm">Remove video tools</span>
          <VideoToolsRemoval />
        </div>
      )}
    </section>
  );
};

const ToolsSkeleton = ({ error }: { error: string }) => (
  <div className="flex flex-col gap-6">
    {error ? (
      <p role="alert" className="text-destructive text-sm">
        {error}
      </p>
    ) : (
      <output className="sr-only">Loading tools…</output>
    )}
    <div className="flex flex-col gap-1.5">
      <Skeleton className="mx-1 h-7 w-16" />
      <Skeleton className="h-14 w-full rounded-xl" />
    </div>
    <div className="flex flex-col gap-1.5">
      <Skeleton className="mx-1 h-7 w-20" />
      <Skeleton className="h-40 w-full rounded-xl" />
    </div>
  </div>
);

export const ToolsSettings = ({
  onVideoSetup,
}: {
  onVideoSetup: () => void;
}) => {
  const { busy, error, execute, tools } = useVideoTools();

  if (!tools) {
    return <ToolsSkeleton error={error} />;
  }

  const { video, built_in: builtIn } = tools;

  return (
    <div className="flex flex-col gap-6">
      <BuiltInTools ffmpeg={builtIn.ffmpeg} ffprobe={builtIn.ffprobe} />
      <div className="flex flex-col gap-3">
        <VideoTools
          busy={busy}
          execute={execute}
          onVideoSetup={onVideoSetup}
          video={video}
        />
        <VideoPreferences busy={busy} execute={execute} video={video} />
        {error && (
          <p
            role="alert"
            className="text-destructive animate-settle px-1 text-xs"
          >
            {error}
          </p>
        )}
      </div>
      <p
        className="text-muted-foreground animate-settle px-1 text-xs"
        style={stagger(3)}
      >
        Powered by{" "}
        <a
          className="hover:text-foreground underline underline-offset-2 transition-colors"
          href="https://github.com/yt-dlp/yt-dlp"
          target="_blank"
          rel="noopener noreferrer"
        >
          yt-dlp
        </a>{" "}
        ·{" "}
        <a
          className="hover:text-foreground underline underline-offset-2 transition-colors"
          href="https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md"
          target="_blank"
          rel="noopener noreferrer"
        >
          Supported sites
        </a>
      </p>
    </div>
  );
};
